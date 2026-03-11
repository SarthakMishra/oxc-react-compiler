#![allow(dead_code)]

use crate::hir::types::{HIR, IdentifierId, InstructionValue, Place};
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
pub fn infer_reactive_places(hir: &mut HIR) {
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
    // The entry block's DeclareLocal instructions represent the function params.
    if let Some((_, entry_block)) = hir.blocks.first() {
        for instr in &entry_block.instructions {
            if let InstructionValue::DeclareLocal { lvalue, .. } = &instr.value {
                reactive_ids.insert(instr.lvalue.identifier.id);
                if let Some(name) = &lvalue.identifier.name {
                    reactive_names.insert(name.clone());
                }
            }
        }
    }

    // Phase 2: Propagate reactivity through data flow (fixpoint)
    // Uses both ID-based and name-based matching since the HIR builder creates
    // fresh IDs for every Place reference.
    let mut changed = true;
    while changed {
        changed = false;
        for (_, block) in &hir.blocks {
            for instr in &block.instructions {
                if reactive_ids.contains(&instr.lvalue.identifier.id) {
                    continue;
                }
                if has_reactive_operand(&instr.value, &reactive_ids, &reactive_names) {
                    reactive_ids.insert(instr.lvalue.identifier.id);
                    if let Some(name) = id_to_name.get(&instr.lvalue.identifier.id) {
                        reactive_names.insert(name.clone());
                    }
                    // For Destructure, also mark pattern target names as reactive
                    if let InstructionValue::Destructure { lvalue_pattern, .. } = &instr.value {
                        collect_pattern_names(lvalue_pattern, &mut reactive_names);
                    }
                    changed = true;
                }
            }
        }
    }

    // Phase 3: Apply reactive flags to places
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if reactive_ids.contains(&instr.lvalue.identifier.id) {
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
        InstructionValue::ComputedLoad { object, property }
        | InstructionValue::ComputedDelete { object, property } => check(object) || check(property),
        InstructionValue::ComputedStore { object, property, value } => {
            check(object) || check(property) || check(value)
        }
        InstructionValue::CallExpression { callee, args }
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

/// Collect all variable names from a destructure pattern.
fn collect_pattern_names(
    pattern: &crate::hir::types::DestructurePattern,
    names: &mut FxHashSet<String>,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};

    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                collect_target_names(&prop.value, names);
            }
            if let Some(rest_place) = rest {
                if let Some(name) = &rest_place.identifier.name {
                    names.insert(name.clone());
                }
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(target) => {
                        collect_target_names(target, names);
                    }
                    DestructureArrayItem::Spread(place) => {
                        if let Some(name) = &place.identifier.name {
                            names.insert(name.clone());
                        }
                    }
                    DestructureArrayItem::Hole => {}
                }
            }
            if let Some(rest_place) = rest {
                if let Some(name) = &rest_place.identifier.name {
                    names.insert(name.clone());
                }
            }
        }
    }
}

fn collect_target_names(
    target: &crate::hir::types::DestructureTarget,
    names: &mut FxHashSet<String>,
) {
    use crate::hir::types::DestructureTarget;

    match target {
        DestructureTarget::Place(place) => {
            if let Some(name) = &place.identifier.name {
                names.insert(name.clone());
            }
        }
        DestructureTarget::Pattern(nested) => {
            collect_pattern_names(nested, names);
        }
    }
}
