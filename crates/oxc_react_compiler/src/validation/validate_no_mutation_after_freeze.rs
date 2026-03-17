// Frozen-mutation validation: detect mutations to values that have been frozen.
//
// This pass uses a hybrid approach:
// 1. MutateFrozen effects from infer_mutation_aliasing_effects (Pass 16) catch
//    definite Mutate effects on values frozen in the heap (params, JSX-frozen values
//    when IDs align).
// 2. Targeted instruction-level checks catch cases the effects pass can't handle:
//    - MethodCall on frozen receivers
//    - PropertyStore on frozen values (hook returns, JSX-frozen)
//    - Mutations inside nested function bodies on frozen outer variables
//    - Hook call freezes captures of function arguments

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{
    AliasingEffect, DestructureArrayItem, DestructurePattern, DestructureTarget, HIR, IdentifierId,
    InstructionValue,
};
use rustc_hash::{FxHashMap, FxHashSet};

const FROZEN_MUTATION_ERROR: &str = "This value cannot be modified. Modifying a value used \
     previously in JSX is not allowed. Consider moving the \
     modification before the JSX expression.";

/// Check if a name looks like a React hook (starts with "use" + uppercase, or is "use").
fn is_hook_name(name: &str) -> bool {
    if name == "use" {
        return true;
    }
    if let Some(rest) = name.strip_prefix("use") {
        rest.starts_with(|c: char| c.is_ascii_uppercase())
    } else {
        false
    }
}

/// Resolve a place's name: check id_to_name first, fall back to identifier.name.
fn resolve_name<'a>(
    id: IdentifierId,
    id_to_name: &'a FxHashMap<IdentifierId, &'a str>,
    fallback_name: Option<&'a str>,
) -> Option<&'a str> {
    id_to_name.get(&id).copied().or(fallback_name)
}

/// Detect mutations to values that have been frozen (used in JSX or passed to hooks).
pub fn validate_no_mutation_after_freeze(
    hir: &HIR,
    errors: &mut ErrorCollector,
    param_names: &[String],
) {
    let mut id_to_name: FxHashMap<IdentifierId, &str> = FxHashMap::default();
    let mut frozen_names: FxHashSet<&str> = FxHashSet::default();
    let mut hook_return_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    // Map: function lvalue ID → captured variable names
    let mut func_captures: FxHashMap<IdentifierId, Vec<&str>> = FxHashMap::default();
    // Map: function name → captured variable names
    let mut name_to_func_captures: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
    // Map: function lvalue ID → reference to lowered function body
    let mut func_bodies: FxHashMap<IdentifierId, &HIR> = FxHashMap::default();

    // Pre-freeze function parameters (props, hook arguments)
    for name in param_names {
        frozen_names.insert(name);
    }

    // First pass: build ID-to-name map, track hook returns and function captures
    for (_, block) in &hir.blocks {
        for phi in &block.phis {
            if let Some(name) = &phi.place.identifier.name {
                id_to_name.insert(phi.place.identifier.id, name);
            }
            for (_, operand) in &phi.operands {
                if let Some(name) = &operand.identifier.name {
                    id_to_name.entry(operand.identifier.id).or_insert(name);
                }
            }
        }

        for instr in &block.instructions {
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_name.insert(instr.lvalue.identifier.id, name);
            }
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::StoreContext { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::CallExpression { callee, .. } => {
                    let callee_name = id_to_name
                        .get(&callee.identifier.id)
                        .copied()
                        .or(callee.identifier.name.as_deref());
                    if callee_name.is_some_and(is_hook_name) {
                        hook_return_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                InstructionValue::FunctionExpression { lowered_func, .. } => {
                    // Collect captured variable names
                    let mut captures: Vec<&str> = lowered_func
                        .context
                        .iter()
                        .filter_map(|p| {
                            id_to_name
                                .get(&p.identifier.id)
                                .copied()
                                .or(p.identifier.name.as_deref())
                        })
                        .collect();
                    // Scan inner body for outer-scope references
                    let inner_locals = collect_inner_declared_names(&lowered_func.body);
                    for (_, inner_block) in &lowered_func.body.blocks {
                        for inner_instr in &inner_block.instructions {
                            if let InstructionValue::LoadLocal { place }
                            | InstructionValue::LoadContext { place } = &inner_instr.value
                                && let Some(name) = &place.identifier.name
                                && !inner_locals.contains(name.as_str())
                                && !captures.contains(&name.as_str())
                            {
                                captures.push(name);
                            }
                        }
                    }
                    if !captures.is_empty() {
                        func_captures.insert(instr.lvalue.identifier.id, captures.clone());
                        if let Some(name) = &instr.lvalue.identifier.name {
                            name_to_func_captures.insert(name, captures);
                        }
                    }
                    func_bodies.insert(instr.lvalue.identifier.id, &lowered_func.body);
                }
                _ => {}
            }
        }
    }

    // Pre-freeze hook return values (useState state, useReducer state, etc.)
    // Skip ref-typed values: useRef() returns are designed to be mutable
    // (ref.current can always be modified). Upstream's type system handles
    // this via Type::Ref; we check the lvalue type here.
    //
    // Also collect ref-typed names so we don't freeze them via other paths.
    // Collect ref-typed names and names from useRef() calls.
    // Refs are mutable by design (ref.current can always be modified).
    // Detect via: Type::Ref annotation, or useRef() call result, or
    // naming convention (variable name ends with "Ref").
    let mut ref_names: FxHashSet<String> = FxHashSet::default();
    let mut ref_hook_result_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Type-based detection
            if matches!(instr.lvalue.identifier.type_, crate::hir::types::Type::Ref)
                && let Some(name) = &instr.lvalue.identifier.name
            {
                ref_names.insert(name.clone());
            }
            // useRef() call detection
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                let callee_name = id_to_name
                    .get(&callee.identifier.id)
                    .copied()
                    .or(callee.identifier.name.as_deref());
                if callee_name == Some("useRef") {
                    ref_hook_result_ids.insert(instr.lvalue.identifier.id);
                }
            }
            // Track StoreLocal of useRef results
            if let InstructionValue::StoreLocal { lvalue, value, .. } = &instr.value {
                if ref_hook_result_ids.contains(&value.identifier.id)
                    && let Some(name) = &lvalue.identifier.name
                {
                    ref_names.insert(name.clone());
                }
                if matches!(lvalue.identifier.type_, crate::hir::types::Type::Ref)
                    && let Some(name) = &lvalue.identifier.name
                {
                    ref_names.insert(name.clone());
                }
            }
        }
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::StoreLocal { value, lvalue, .. }
                | InstructionValue::StoreContext { value, lvalue } => {
                    if hook_return_ids.contains(&value.identifier.id) {
                        hook_return_ids.insert(instr.lvalue.identifier.id);
                        if let Some(name) = &lvalue.identifier.name {
                            // Don't freeze ref values — they're designed to be mutable
                            if !ref_names.contains(name.as_str()) {
                                frozen_names.insert(name);
                            }
                        }
                    }
                }
                InstructionValue::Destructure { value, lvalue_pattern } => {
                    if hook_return_ids.contains(&value.identifier.id) {
                        collect_frozen_from_destructure(lvalue_pattern, &mut frozen_names);
                    }
                }
                _ => {}
            }
        }
    }

    // Remove any ref names that leaked into frozen_names via other paths
    for name in &ref_names {
        frozen_names.remove(name.as_str());
    }

    // Main pass: walk instructions, check for frozen mutations
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Check 1: MutateFrozen effects from the aliasing pass
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    if matches!(effect, AliasingEffect::MutateFrozen { .. }) {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            FROZEN_MUTATION_ERROR,
                            DiagnosticKind::ImmutabilityViolation,
                        ));
                        return;
                    }
                }
            }

            // Update frozen_names from Freeze effects (for instruction-level checks)
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    if let AliasingEffect::Freeze { value, .. } = effect
                        && let Some(name) = resolve_name(
                            value.identifier.id,
                            &id_to_name,
                            value.identifier.name.as_deref(),
                        )
                    {
                        frozen_names.insert(name);
                    }
                }
            }

            // Hook calls freeze captured variables of function arguments
            if let InstructionValue::CallExpression { callee, args } = &instr.value {
                let callee_name = resolve_name(
                    callee.identifier.id,
                    &id_to_name,
                    callee.identifier.name.as_deref(),
                );
                if callee_name.is_some_and(is_hook_name) {
                    for arg in args {
                        let arg_captures = func_captures.get(&arg.identifier.id).or_else(|| {
                            let arg_name = resolve_name(
                                arg.identifier.id,
                                &id_to_name,
                                arg.identifier.name.as_deref(),
                            )?;
                            name_to_func_captures.get(arg_name)
                        });
                        if let Some(captures) = arg_captures {
                            for &captured_name in captures {
                                frozen_names.insert(captured_name);
                            }
                        }

                        // Re-check function body for mutations to now-frozen variables
                        if let Some(fn_body) = func_bodies.get(&arg.identifier.id)
                            && has_mutation_on_frozen_names(fn_body, &frozen_names)
                        {
                            errors.push(CompilerError::invalid_react_with_kind(
                                instr.loc,
                                FROZEN_MUTATION_ERROR,
                                DiagnosticKind::ImmutabilityViolation,
                            ));
                            return;
                        }
                    }
                }
            }

            // Check 2: MethodCall on frozen receiver — only flag if the method
            // is KNOWN to mutate. Read-only methods (.map(), .at(), .filter(),
            // .toString(), .foo()) are safe on frozen values.
            // Upstream uses method signatures; we use a conservative allowlist.
            if let InstructionValue::MethodCall { receiver, property, .. } = &instr.value {
                let receiver_name = resolve_name(
                    receiver.identifier.id,
                    &id_to_name,
                    receiver.identifier.name.as_deref(),
                );
                if receiver_name.is_some_and(|name| frozen_names.contains(name))
                    && is_known_mutating_method(property)
                {
                    errors.push(CompilerError::invalid_react_with_kind(
                        instr.loc,
                        FROZEN_MUTATION_ERROR,
                        DiagnosticKind::ImmutabilityViolation,
                    ));
                    return;
                }
            }

            // Check 3: PropertyStore/ComputedStore/Delete on frozen values
            match &instr.value {
                InstructionValue::PropertyStore { object, .. }
                | InstructionValue::ComputedStore { object, .. }
                | InstructionValue::PropertyDelete { object, .. }
                | InstructionValue::ComputedDelete { object, .. } => {
                    let obj_name = resolve_name(
                        object.identifier.id,
                        &id_to_name,
                        object.identifier.name.as_deref(),
                    );
                    if obj_name.is_some_and(|name| frozen_names.contains(name)) {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            FROZEN_MUTATION_ERROR,
                            DiagnosticKind::ImmutabilityViolation,
                        ));
                        return;
                    }
                }
                _ => {}
            }

            // Check 4: Mutations inside nested function bodies on frozen outer variables
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && !frozen_names.is_empty()
                && has_mutation_on_frozen_names(&lowered_func.body, &frozen_names)
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    FROZEN_MUTATION_ERROR,
                    DiagnosticKind::ImmutabilityViolation,
                ));
                return;
            }

            // Check 5: Mutate effects on frozen names (catches cases where
            // the effects pass generates Mutate but the value isn't frozen in the heap).
            // For call instructions (CallExpression, MethodCall, NewExpression), only
            // check definite mutations — conditional ones come from Apply fallback and
            // cause false positives on frozen params passed to functions.
            if let Some(ref effects) = instr.effects {
                let is_call = matches!(
                    instr.value,
                    InstructionValue::CallExpression { .. }
                        | InstructionValue::MethodCall { .. }
                        | InstructionValue::NewExpression { .. }
                );
                for effect in effects {
                    let mutated_name = match effect {
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateTransitive { value } => resolve_name(
                            value.identifier.id,
                            &id_to_name,
                            value.identifier.name.as_deref(),
                        ),
                        AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitiveConditionally { value }
                            if !is_call =>
                        {
                            resolve_name(
                                value.identifier.id,
                                &id_to_name,
                                value.identifier.name.as_deref(),
                            )
                        }
                        _ => None,
                    };
                    if mutated_name.is_some_and(|name| frozen_names.contains(name)) {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            FROZEN_MUTATION_ERROR,
                            DiagnosticKind::ImmutabilityViolation,
                        ));
                        return;
                    }
                }
            }
        }
    }
}

/// Returns true if a method name is known to mutate its receiver.
/// Used to distinguish mutating methods (.push(), .splice()) from
/// read-only ones (.map(), .at(), .filter()) when checking MethodCall
/// on frozen receivers.
fn is_known_mutating_method(method: &str) -> bool {
    matches!(
        method,
        // Array mutating methods
        "push" | "pop" | "shift" | "unshift" | "splice" | "sort" | "reverse"
        | "fill" | "copyWithin"
        // Set/Map mutating methods
        | "add" | "set" | "delete" | "clear"
        // Generic mutating patterns
        | "append" | "remove" | "insert" | "assign"
    )
}

/// Check if a nested function body contains mutations to any of the outer frozen variables.
fn has_mutation_on_frozen_names(hir: &HIR, outer_frozen: &FxHashSet<&str>) -> bool {
    let mut local_id_map: FxHashMap<IdentifierId, &str> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        local_id_map.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::StoreContext { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name {
                        local_id_map.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(name) = &lvalue.identifier.name {
                        local_id_map.insert(instr.lvalue.identifier.id, name);
                    }
                }
                _ => {}
            }
            if let Some(name) = &instr.lvalue.identifier.name {
                local_id_map.insert(instr.lvalue.identifier.id, name);
            }

            // Check MutateFrozen effects
            // For call instructions (CallExpression, MethodCall, NewExpression), only
            // check definite mutations — conditional ones come from Apply fallback and
            // cause false positives on frozen params passed to functions.
            // This mirrors the logic in the outer Check 5.
            if let Some(ref effects) = instr.effects {
                let is_call = matches!(
                    instr.value,
                    InstructionValue::CallExpression { .. }
                        | InstructionValue::MethodCall { .. }
                        | InstructionValue::NewExpression { .. }
                );
                for effect in effects {
                    if matches!(effect, AliasingEffect::MutateFrozen { .. }) {
                        return true;
                    }
                    let mutated_name = match effect {
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateTransitive { value } => {
                            local_id_map.get(&value.identifier.id).copied()
                        }
                        AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitiveConditionally { value }
                            if !is_call =>
                        {
                            local_id_map.get(&value.identifier.id).copied()
                        }
                        _ => None,
                    };
                    if mutated_name.is_some_and(|name| outer_frozen.contains(name)) {
                        return true;
                    }
                }
            }

            // Check instruction-level mutations
            let check_frozen = |id: &IdentifierId| -> bool {
                local_id_map.get(id).is_some_and(|name| outer_frozen.contains(name))
            };
            match &instr.value {
                InstructionValue::MethodCall { receiver, property, .. } => {
                    if check_frozen(&receiver.identifier.id) && is_known_mutating_method(property) {
                        return true;
                    }
                }
                InstructionValue::PropertyStore { object, .. }
                | InstructionValue::ComputedStore { object, .. }
                | InstructionValue::PropertyDelete { object, .. }
                | InstructionValue::ComputedDelete { object, .. } => {
                    if check_frozen(&object.identifier.id) {
                        return true;
                    }
                }
                InstructionValue::PrefixUpdate { lvalue, .. }
                | InstructionValue::PostfixUpdate { lvalue, .. } => {
                    if check_frozen(&lvalue.identifier.id) {
                        return true;
                    }
                }
                _ => {}
            }

            // Recurse into nested functions
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && has_mutation_on_frozen_names(&lowered_func.body, outer_frozen)
            {
                return true;
            }
        }
    }

    false
}

/// Collect all variable names declared within an inner function body.
fn collect_inner_declared_names(hir: &HIR) -> FxHashSet<&str> {
    let mut names = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(name) = &lvalue.identifier.name {
                        names.insert(name.as_str());
                    }
                }
                InstructionValue::StoreLocal {
                    lvalue,
                    type_:
                        Some(
                            crate::hir::types::InstructionKind::Let
                            | crate::hir::types::InstructionKind::Const
                            | crate::hir::types::InstructionKind::Var,
                        ),
                    ..
                } => {
                    if let Some(name) = &lvalue.identifier.name {
                        names.insert(name.as_str());
                    }
                }
                _ => {}
            }
        }
    }
    names
}

/// Collect variable names from a destructure pattern and add to frozen set.
fn collect_frozen_from_destructure<'a>(
    pattern: &'a DestructurePattern,
    frozen: &mut FxHashSet<&'a str>,
) {
    match pattern {
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(DestructureTarget::Place(p)) => {
                        if let Some(name) = &p.identifier.name {
                            frozen.insert(name);
                        }
                    }
                    DestructureArrayItem::Value(DestructureTarget::Pattern(nested)) => {
                        collect_frozen_from_destructure(nested, frozen);
                    }
                    DestructureArrayItem::Spread(p) => {
                        if let Some(name) = &p.identifier.name {
                            frozen.insert(name);
                        }
                    }
                    DestructureArrayItem::Hole => {}
                }
            }
            if let Some(rest) = rest
                && let Some(name) = &rest.identifier.name
            {
                frozen.insert(name);
            }
        }
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                match &prop.value {
                    DestructureTarget::Place(p) => {
                        if let Some(name) = &p.identifier.name {
                            frozen.insert(name);
                        }
                    }
                    DestructureTarget::Pattern(nested) => {
                        collect_frozen_from_destructure(nested, frozen);
                    }
                }
            }
            if let Some(rest) = rest
                && let Some(name) = &rest.identifier.name
            {
                frozen.insert(name);
            }
        }
    }
}
