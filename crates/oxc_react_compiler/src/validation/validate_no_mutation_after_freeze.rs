// DIVERGENCE: Upstream detects frozen-value mutations inside
// InferMutableRanges.ts as part of the abstract interpretation.
// Our port uses a post-effects validation pass that tracks freeze/mutate
// by variable name, because our HIR creates fresh IdentifierIds per
// Place reference — there is no single stable ID across references.

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{
    AliasingEffect, ArrayElement, DestructureArrayItem, DestructurePattern, DestructureTarget, HIR,
    IdentifierId, InstructionValue, ObjectPropertyKey, Place,
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

/// Resolve a place's name: check id_to_source_name first, fall back to identifier.name.
fn resolve_name<'a>(
    place: &'a Place,
    id_to_source_name: &'a FxHashMap<IdentifierId, &'a str>,
) -> Option<&'a str> {
    id_to_source_name.get(&place.identifier.id).copied().or(place.identifier.name.as_deref())
}

/// Detect mutations to values that have been frozen (used in JSX or passed to hooks).
///
/// After `infer_mutation_aliasing_effects` runs, each instruction has computed effects.
/// This pass walks instructions in program order and tracks which variable names have
/// been frozen. Any mutation to a frozen variable is an error.
///
/// Since the HIR creates fresh IdentifierIds for every Place reference, we track by
/// variable name using a lvalue-ID → source-variable-name mapping built from
/// `LoadLocal`/`LoadContext` instructions.
pub fn validate_no_mutation_after_freeze(
    hir: &HIR,
    errors: &mut ErrorCollector,
    param_names: &[String],
) {
    // Build lvalue_id → source variable name map (borrows from HIR, no clones).
    // When LoadLocal { place: x_ref } → lvalue_temp, this maps lvalue_temp's ID to "x".
    let mut id_to_source_name: FxHashMap<IdentifierId, &str> = FxHashMap::default();

    // Also track: hook return lvalue IDs (for pre-freeze of useContext/useState results)
    let mut hook_return_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    // Map: function lvalue ID → captured variable names (for hook-call-freezes-captures)
    let mut func_captures: FxHashMap<IdentifierId, Vec<&str>> = FxHashMap::default();
    // Map: function name → captured variable names (resolved through LoadLocal chains)
    let mut name_to_func_captures: FxHashMap<&str, Vec<&str>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_source_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::StoreContext { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_source_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_source_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::CallExpression { callee, .. } => {
                    // Detect hook calls for pre-freeze of return values
                    let callee_name = id_to_source_name
                        .get(&callee.identifier.id)
                        .copied()
                        .or(callee.identifier.name.as_deref());
                    if callee_name.is_some_and(is_hook_name) {
                        hook_return_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                InstructionValue::FunctionExpression { lowered_func, .. } => {
                    // Collect captured variable names from function context
                    let captures: Vec<&str> = lowered_func
                        .context
                        .iter()
                        .filter_map(|p| {
                            id_to_source_name
                                .get(&p.identifier.id)
                                .copied()
                                .or(p.identifier.name.as_deref())
                        })
                        .collect();
                    if !captures.is_empty() {
                        func_captures.insert(instr.lvalue.identifier.id, captures.clone());
                        if let Some(name) = &instr.lvalue.identifier.name {
                            name_to_func_captures.insert(name, captures);
                        }
                    }
                }
                _ => {}
            }
            // Fallback: map lvalue's own name (for instruction types not matched above)
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_source_name.insert(instr.lvalue.identifier.id, name);
            }

            // Also map ALL operand place IDs to names. This is critical because
            // our HIR creates fresh IdentifierIds per Place reference, so an
            // operand in a JSX child or function arg has a different ID than the
            // LoadLocal lvalue it came from. By mapping operand IDs too, we can
            // resolve names when Freeze/Mutate effects reference operand places.
            map_operand_ids(&instr.value, &mut id_to_source_name);
        }
    }

    // DIVERGENCE: Upstream tracks freeze transitively through the abstract
    // interpreter (InferMutableRanges). We approximate by pre-freezing hook
    // return values and all their destructured targets at their definition site.
    // This over-freezes setters (e.g., setState from useState) but in practice
    // setters are never mutated via property stores, so no false positives arise.
    //
    // DIVERGENCE: We also pre-freeze function parameters (e.g., "props" in component
    // functions, "options" in hooks). Upstream freezes params transitively through
    // InferMutableRanges; we approximate by marking them frozen at the start.
    // The param_names are extracted from the HIRFunction's params list by the pipeline.
    let mut pre_frozen: FxHashSet<&str> = FxHashSet::default();

    // Pre-freeze function parameters: component props, hook arguments, etc.
    for name in param_names {
        pre_frozen.insert(name);
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Propagate hook_return_ids through StoreLocal chains
            match &instr.value {
                InstructionValue::StoreLocal { value, lvalue, .. }
                | InstructionValue::StoreContext { value, lvalue } => {
                    if hook_return_ids.contains(&value.identifier.id) {
                        hook_return_ids.insert(instr.lvalue.identifier.id);
                        if let Some(name) = &lvalue.identifier.name {
                            pre_frozen.insert(name);
                        }
                    }
                }
                // Destructure of hook return: const [state, setState] = useState(...)
                // Freezes ALL destructured targets. This over-freezes the setter,
                // but setters are never mutated via property stores in practice.
                InstructionValue::Destructure { value, lvalue_pattern } => {
                    if hook_return_ids.contains(&value.identifier.id) {
                        collect_frozen_from_destructure(lvalue_pattern, &mut pre_frozen);
                    }
                }
                _ => {}
            }
        }
    }

    // Walk instructions in program order, tracking frozen variable names
    let mut frozen_names: FxHashSet<&str> = pre_frozen;

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // First: process freeze effects to update frozen_names
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    match effect {
                        AliasingEffect::Freeze { value, .. }
                        | AliasingEffect::ImmutableCapture { from: value, .. } => {
                            if let Some(name) = id_to_source_name.get(&value.identifier.id) {
                                frozen_names.insert(name);
                            }
                            if let Some(name) = &value.identifier.name {
                                frozen_names.insert(name);
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Hook calls freeze captured variables of function arguments.
            // e.g., useIdentity(() => { mutate(x) }) freezes x after the call.
            if let InstructionValue::CallExpression { callee, args } = &instr.value {
                let callee_name = resolve_name(callee, &id_to_source_name);
                if callee_name.is_some_and(is_hook_name) {
                    for arg in args {
                        // Check if this arg is a function with captures
                        let arg_captures = func_captures.get(&arg.identifier.id).or_else(|| {
                            let arg_name = resolve_name(arg, &id_to_source_name)?;
                            name_to_func_captures.get(arg_name)
                        });
                        if let Some(captures) = arg_captures {
                            for &captured_name in captures {
                                frozen_names.insert(captured_name);
                            }
                        }
                    }
                }
            }

            // Check instruction-level mutations (MethodCall, PropertyStore, etc.)
            if check_instruction_mutation(instr, &id_to_source_name, &frozen_names) {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    FROZEN_MUTATION_ERROR,
                    DiagnosticKind::ImmutabilityViolation,
                ));
                return;
            }

            // Check for mutations inside nested function bodies (closures).
            // If a FunctionExpression captures frozen variables, scan its body
            // for mutations to those variables.
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && !frozen_names.is_empty()
                && check_nested_function_mutation(&lowered_func.body, &frozen_names)
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    FROZEN_MUTATION_ERROR,
                    DiagnosticKind::ImmutabilityViolation,
                ));
                return;
            }

            // Also check explicit Mutate effects from the aliasing pass
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    let mutated_name = match effect {
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value }
                        | AliasingEffect::MutateFrozen { place: value, .. } => {
                            id_to_source_name.get(&value.identifier.id).copied()
                        }
                        _ => None,
                    };

                    if let Some(name) = mutated_name
                        && frozen_names.contains(name)
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
    }
}

/// Collect variable names from a destructure pattern and add to frozen set.
/// Used for hook return destructuring: const [state, setter] = useState(...)
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

/// Check if a nested function body contains mutations to any of the outer frozen variables.
/// This handles cases like: const onChange = () => { x.value = ...; } where x is frozen.
fn check_nested_function_mutation(hir: &HIR, outer_frozen: &FxHashSet<&str>) -> bool {
    // Build a local id_to_source_name for the nested function's HIR
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
            map_operand_ids(&instr.value, &mut local_id_map);
        }
    }

    // Check for mutations using outer frozen names
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if check_instruction_mutation(instr, &local_id_map, outer_frozen) {
                return true;
            }

            // Also check Mutate effects
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    let mutated_name = match effect {
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value }
                        | AliasingEffect::MutateFrozen { place: value, .. } => {
                            local_id_map.get(&value.identifier.id).copied()
                        }
                        _ => None,
                    };
                    if let Some(name) = mutated_name
                        && outer_frozen.contains(name)
                    {
                        return true;
                    }
                }
            }

            // Recurse into nested nested functions (depth-limited by HIR structure)
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && check_nested_function_mutation(&lowered_func.body, outer_frozen)
            {
                return true;
            }
        }
    }

    false
}

/// Map operand place IDs to variable names for all places in an instruction value.
/// This covers the ID disconnect: operand places in JSX children, function args, etc.
/// have fresh IDs that differ from the LoadLocal lvalue IDs.
fn map_operand_ids<'a>(value: &'a InstructionValue, id_map: &mut FxHashMap<IdentifierId, &'a str>) {
    let mut map_place = |place: &'a Place| {
        if let Some(name) = &place.identifier.name {
            id_map.entry(place.identifier.id).or_insert(name);
        }
    };

    match value {
        InstructionValue::LoadLocal { place }
        | InstructionValue::LoadContext { place }
        | InstructionValue::Await { value: place }
        | InstructionValue::GetIterator { collection: place }
        | InstructionValue::IteratorNext { iterator: place, .. }
        | InstructionValue::NextPropertyOf { value: place }
        | InstructionValue::UnaryExpression { value: place, .. }
        | InstructionValue::TypeCastExpression { value: place, .. } => {
            map_place(place);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            map_place(lvalue);
            map_place(value);
        }
        InstructionValue::DeclareLocal { lvalue, .. }
        | InstructionValue::DeclareContext { lvalue } => {
            map_place(lvalue);
        }
        InstructionValue::Destructure { value, .. } => {
            map_place(value);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            map_place(left);
            map_place(right);
        }
        InstructionValue::CallExpression { callee, args }
        | InstructionValue::NewExpression { callee, args } => {
            map_place(callee);
            for arg in args {
                map_place(arg);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            map_place(receiver);
            for arg in args {
                map_place(arg);
            }
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            map_place(object);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            map_place(object);
            map_place(value);
        }
        InstructionValue::ComputedLoad { object, property } => {
            map_place(object);
            map_place(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            map_place(object);
            map_place(property);
            map_place(value);
        }
        InstructionValue::ComputedDelete { object, property } => {
            map_place(object);
            map_place(property);
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            map_place(tag);
            for attr in props {
                map_place(&attr.value);
            }
            for child in children {
                map_place(child);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                map_place(child);
            }
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                map_place(&prop.value);
                if let ObjectPropertyKey::Computed(key) = &prop.key {
                    map_place(key);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    ArrayElement::Expression(p) | ArrayElement::Spread(p) => map_place(p),
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                map_place(sub);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value: tpl } => {
            map_place(tag);
            for sub in &tpl.subexpressions {
                map_place(sub);
            }
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            map_place(lvalue);
        }
        InstructionValue::StoreGlobal { value, .. } => {
            map_place(value);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            map_place(decl);
            for dep in deps {
                map_place(dep);
            }
        }
        // No operands to map
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. } => {}
    }
}

/// Check if an instruction directly mutates a frozen value via its instruction value.
/// This catches cases where the aliasing effects don't generate explicit Mutate effects,
/// such as MethodCall receivers (x.push()) and PropertyStore (x.prop = ...).
fn check_instruction_mutation(
    instr: &crate::hir::types::Instruction,
    id_to_source_name: &FxHashMap<IdentifierId, &str>,
    frozen_names: &FxHashSet<&str>,
) -> bool {
    let check_frozen = |id: &IdentifierId| -> bool {
        id_to_source_name.get(id).is_some_and(|name| frozen_names.contains(name))
    };

    match &instr.value {
        // x.push(...), x.splice(...), etc. — method call may mutate receiver
        InstructionValue::MethodCall { receiver, .. } => check_frozen(&receiver.identifier.id),
        // x.prop = value — direct property mutation
        InstructionValue::PropertyStore { object, .. }
        | InstructionValue::ComputedStore { object, .. } => check_frozen(&object.identifier.id),
        // delete x[i]
        InstructionValue::PropertyDelete { object, .. }
        | InstructionValue::ComputedDelete { object, .. } => check_frozen(&object.identifier.id),
        // ++x, x++
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => check_frozen(&lvalue.identifier.id),
        _ => false,
    }
}
