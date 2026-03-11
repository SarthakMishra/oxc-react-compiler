#![allow(dead_code)]

use crate::hir::types::{HIR, IdentifierId, InstructionValue, Place};
use rustc_hash::FxHashSet;

/// Infer which places are reactive (depend on props/state).
///
/// A place is reactive if:
/// - It is a function parameter (props)
/// - It is loaded from a hook call (useState, useContext, etc.)
/// - It is derived from a reactive place through data flow
///
/// Uses fixpoint iteration with post-dominator analysis.
pub fn infer_reactive_places(hir: &mut HIR) {
    // Phase 1: Seed reactive set with function params and hook returns
    let mut reactive: FxHashSet<IdentifierId> = FxHashSet::default();

    // Mark all places that come from hook calls as reactive
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if is_hook_result(&instr.value) {
                reactive.insert(instr.lvalue.identifier.id);
            }
        }
    }

    // Mark function parameters as reactive (props change between renders).
    // The entry block's DeclareLocal instructions represent the function params.
    if let Some((_, entry_block)) = hir.blocks.first() {
        for instr in &entry_block.instructions {
            if matches!(instr.value, InstructionValue::DeclareLocal { .. }) {
                reactive.insert(instr.lvalue.identifier.id);
            }
        }
    }

    // Phase 2: Propagate reactivity through data flow (fixpoint)
    let mut changed = true;
    while changed {
        changed = false;
        for (_, block) in &hir.blocks {
            for instr in &block.instructions {
                if reactive.contains(&instr.lvalue.identifier.id) {
                    continue;
                }
                if has_reactive_operand(&instr.value, &reactive) {
                    reactive.insert(instr.lvalue.identifier.id);
                    changed = true;
                }
            }
        }
    }

    // Phase 3: Apply reactive flags to places
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if reactive.contains(&instr.lvalue.identifier.id) {
                instr.lvalue.reactive = true;
            }
        }
    }
}

fn is_hook_result(value: &InstructionValue) -> bool {
    match value {
        InstructionValue::CallExpression { callee, .. } => {
            // Check if callee name starts with "use" (hook convention)
            // This is a simplified check — full implementation uses the shape system
            callee.identifier.name.as_deref().is_some_and(crate::hir::globals::is_hook_name)
        }
        _ => false,
    }
}

fn has_reactive_operand(value: &InstructionValue, reactive: &FxHashSet<IdentifierId>) -> bool {
    let check = |place: &Place| reactive.contains(&place.identifier.id);

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
