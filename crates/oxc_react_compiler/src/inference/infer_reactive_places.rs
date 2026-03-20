use crate::hir::types::{HIR, IdentifierId, InstructionValue, Place, Type};
use rustc_hash::{FxHashMap, FxHashSet};

/// Infer which places are reactive (depend on props/state).
///
/// A place is reactive if:
/// - It is a function parameter (props)
/// - It is loaded from a hook call (useState, useContext, etc.)
/// - It is derived from a reactive place through data flow
///
/// Uses fixpoint iteration. Since the HIR builder creates fresh IdentifierIds for every
/// Place reference (even for the same variable), we track reactivity by variable NAME
/// in addition to ID, so that DeclareLocal(count) → LoadLocal(count) propagation works.
pub fn infer_reactive_places(hir: &mut HIR, param_names: &[String], param_ids: &[IdentifierId]) {
    // Build id → name map for resolving names across instruction boundaries.
    // LoadGlobal: lvalue_id → global name (e.g., "useState")
    // DeclareLocal: lvalue_id → inner lvalue name (e.g., "count")
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Map instruction lvalue ID to its name (if any)
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_name.insert(instr.lvalue.identifier.id, name.clone());
            }

            match &instr.value {
                InstructionValue::LoadGlobal { binding } => {
                    // Map the lvalue ID to the global name so that CallExpression
                    // can resolve the callee's name for hook detection
                    id_to_name.insert(instr.lvalue.identifier.id, binding.name.clone());
                }
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    // Map lvalue ID to the declared variable's name
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    // Build stable_ids: identifiers with Type::SetState or Type::Ref are stable
    // (they never change between renders). These must not be marked reactive even
    // if derived from a reactive source (e.g., destructured from useState return).
    // Upstream: InferReactivePlaces.ts checks identifier.type for stability.
    let mut stable_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut stable_names: FxHashSet<String> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if matches!(instr.lvalue.identifier.type_, Type::SetState | Type::Ref) {
                stable_ids.insert(instr.lvalue.identifier.id);
                if let Some(name) = &instr.lvalue.identifier.name {
                    stable_names.insert(name.clone());
                }
            }
        }
    }

    // Phase 1: Seed reactive set with function params and hook returns
    let mut reactive_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut reactive_names: FxHashSet<String> = FxHashSet::default();

    // Mark all places that come from hook calls as reactive
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if is_hook_result(&instr.value, &id_to_name) {
                reactive_ids.insert(instr.lvalue.identifier.id);
                if let Some(name) = id_to_name.get(&instr.lvalue.identifier.id) {
                    reactive_names.insert(name.clone());
                }
            }
        }
    }

    // Mark function parameters as reactive (props change between renders).
    // Upstream: InferReactivePlaces.ts seeds reactivity from `params` of the function.
    //
    // We seed from DeclareLocal instructions in the entry block whose inner
    // lvalue name matches a param name. This catches destructured params
    // (e.g., `function Foo({a, b})` produces DeclareLocal for `a` and `b`).
    //
    // With stable IDs, param Places share one IdentifierId across all references.
    // Seed directly from param_ids so that LoadLocal/PropertyLoad of params
    // propagates reactivity through the fixpoint loop.
    // Over-memoization is prevented by the `any_mutable` gate in
    // infer_reactive_scope_variables (primitives-only reactive sets don't get scopes).
    for (id, name) in param_ids.iter().zip(param_names.iter()) {
        reactive_ids.insert(*id);
        reactive_names.insert(name.clone());
    }
    // Also seed DeclareLocal in the entry block for destructured params
    if let Some((_, entry_block)) = hir.blocks.first() {
        for instr in &entry_block.instructions {
            if let InstructionValue::DeclareLocal { lvalue, .. } = &instr.value
                && let Some(name) = &lvalue.identifier.name
                && param_names.iter().any(|p| p == name)
            {
                reactive_ids.insert(instr.lvalue.identifier.id);
                reactive_names.insert(name.clone());
            }
        }
    }

    // Phase 2: Propagate reactivity through data flow (fixpoint)
    // Uses both ID-based and name-based matching since the HIR builder creates
    // fresh IDs for every Place reference.
    // Stable identifiers (SetState, Ref) act as firewalls — they absorb
    // reactivity from their sources but do not propagate it outward.
    let mut changed = true;
    while changed {
        changed = false;
        for (_, block) in &hir.blocks {
            for instr in &block.instructions {
                if reactive_ids.contains(&instr.lvalue.identifier.id) {
                    continue;
                }
                // Skip stable identifiers — they should never be marked reactive
                if stable_ids.contains(&instr.lvalue.identifier.id) {
                    continue;
                }
                if has_reactive_operand(&instr.value, &reactive_ids, &reactive_names) {
                    reactive_ids.insert(instr.lvalue.identifier.id);
                    if let Some(name) = id_to_name.get(&instr.lvalue.identifier.id) {
                        // Don't add stable names to reactive_names to prevent
                        // name-based propagation through stable values
                        if !stable_names.contains(name) {
                            reactive_names.insert(name.clone());
                        }
                    }
                    // For Destructure, also mark pattern target names as reactive
                    // (but skip stable targets like setState/dispatch)
                    if let InstructionValue::Destructure { lvalue_pattern, .. } = &instr.value {
                        collect_pattern_names_filtered(
                            lvalue_pattern,
                            &mut reactive_names,
                            &stable_names,
                        );
                    }
                    changed = true;
                }
            }
        }
    }

    // Phase 3: Apply reactive flags to places
    // Skip stable identifiers even if they ended up in reactive_ids
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if reactive_ids.contains(&instr.lvalue.identifier.id)
                && !stable_ids.contains(&instr.lvalue.identifier.id)
            {
                instr.lvalue.reactive = true;
            }
        }
    }
}

fn is_hook_result(value: &InstructionValue, id_to_name: &FxHashMap<IdentifierId, String>) -> bool {
    match value {
        InstructionValue::CallExpression { callee, .. } => {
            // First check the callee place's name directly
            if callee.identifier.name.as_deref().is_some_and(crate::hir::globals::is_hook_name) {
                return true;
            }
            // Also check via id_to_name (resolves LoadGlobal → global name)
            if let Some(name) = id_to_name.get(&callee.identifier.id) {
                return crate::hir::globals::is_hook_name(name);
            }
            false
        }
        _ => false,
    }
}

fn has_reactive_operand(
    value: &InstructionValue,
    reactive_ids: &FxHashSet<IdentifierId>,
    reactive_names: &FxHashSet<String>,
) -> bool {
    let check = |place: &Place| {
        // Check by ID first (fast path)
        if reactive_ids.contains(&place.identifier.id) {
            return true;
        }
        // Fall back to name-based check for cross-ID reactivity propagation
        if let Some(name) = &place.identifier.name {
            return reactive_names.contains(name);
        }
        false
    };

    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            check(place)
        }
        InstructionValue::BinaryExpression { left, right, .. } => check(left) || check(right),
        InstructionValue::UnaryExpression { value, .. } => check(value),
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => check(object),
        InstructionValue::ComputedLoad { object, property, .. }
        | InstructionValue::ComputedDelete { object, property } => check(object) || check(property),
        InstructionValue::ComputedStore { object, property, value } => {
            check(object) || check(property) || check(value)
        }
        InstructionValue::CallExpression { callee, args, .. }
        | InstructionValue::NewExpression { callee, args } => {
            check(callee) || args.iter().any(check)
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            check(receiver) || args.iter().any(check)
        }
        InstructionValue::Destructure { value, .. } => check(value),
        InstructionValue::StoreLocal { value, .. }
        | InstructionValue::StoreContext { value, .. } => check(value),
        InstructionValue::PropertyStore { object, value, .. } => check(object) || check(value),
        InstructionValue::Await { value }
        | InstructionValue::StoreGlobal { value, .. }
        | InstructionValue::NextPropertyOf { value }
        | InstructionValue::TypeCastExpression { value, .. } => check(value),
        InstructionValue::GetIterator { collection } => check(collection),
        InstructionValue::IteratorNext { iterator, .. } => check(iterator),
        InstructionValue::JsxExpression { tag, props, children } => {
            check(tag) || props.iter().any(|a| check(&a.value)) || children.iter().any(check)
        }
        InstructionValue::JsxFragment { children } => children.iter().any(check),
        InstructionValue::ObjectExpression { properties } => properties.iter().any(|p| {
            check(&p.value)
                || matches!(&p.key, crate::hir::types::ObjectPropertyKey::Computed(k) if check(k))
        }),
        InstructionValue::ArrayExpression { elements } => elements.iter().any(|e| match e {
            crate::hir::types::ArrayElement::Expression(p)
            | crate::hir::types::ArrayElement::Spread(p) => check(p),
            crate::hir::types::ArrayElement::Hole => false,
        }),
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            subexpressions.iter().any(check)
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            check(tag) || value.subexpressions.iter().any(check)
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => check(decl) || deps.iter().any(check),
        _ => false,
    }
}

/// Collect variable names from a destructure pattern, skipping stable names.
/// Stable names (e.g., setState, dispatch) are excluded to prevent reactive
/// propagation through stable hook return values.
fn collect_pattern_names_filtered(
    pattern: &crate::hir::types::DestructurePattern,
    names: &mut FxHashSet<String>,
    stable_names: &FxHashSet<String>,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern};

    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                collect_target_names_filtered(&prop.value, names, stable_names);
            }
            if let Some(rest_place) = rest
                && let Some(name) = &rest_place.identifier.name
                && !stable_names.contains(name)
            {
                names.insert(name.clone());
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(target) => {
                        collect_target_names_filtered(target, names, stable_names);
                    }
                    DestructureArrayItem::Spread(place) => {
                        if let Some(name) = &place.identifier.name
                            && !stable_names.contains(name)
                        {
                            names.insert(name.clone());
                        }
                    }
                    DestructureArrayItem::Hole => {}
                }
            }
            if let Some(rest_place) = rest
                && let Some(name) = &rest_place.identifier.name
                && !stable_names.contains(name)
            {
                names.insert(name.clone());
            }
        }
    }
}

fn collect_target_names_filtered(
    target: &crate::hir::types::DestructureTarget,
    names: &mut FxHashSet<String>,
    stable_names: &FxHashSet<String>,
) {
    use crate::hir::types::DestructureTarget;

    match target {
        DestructureTarget::Place(place) => {
            if let Some(name) = &place.identifier.name
                && !stable_names.contains(name)
            {
                names.insert(name.clone());
            }
        }
        DestructureTarget::Pattern(nested) => {
            collect_pattern_names_filtered(nested, names, stable_names);
        }
    }
}
