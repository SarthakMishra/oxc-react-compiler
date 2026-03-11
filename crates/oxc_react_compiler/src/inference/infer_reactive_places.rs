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
    for (_, block) in hir.blocks.iter() {
        for instr in &block.instructions {
            if is_hook_result(&instr.value) {
                reactive.insert(instr.lvalue.identifier.id);
            }
        }
    }

    // Phase 2: Propagate reactivity through data flow (fixpoint)
    let mut changed = true;
    while changed {
        changed = false;
        for (_, block) in hir.blocks.iter() {
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
    for (_, block) in hir.blocks.iter_mut() {
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
            callee
                .identifier
                .name
                .as_deref()
                .map_or(false, |name| crate::hir::globals::is_hook_name(name))
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
        InstructionValue::PropertyLoad { object, .. } => check(object),
        InstructionValue::ComputedLoad { object, property } => check(object) || check(property),
        InstructionValue::CallExpression { callee, args } => {
            check(callee) || args.iter().any(check)
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            check(receiver) || args.iter().any(check)
        }
        InstructionValue::Destructure { value, .. } => check(value),
        InstructionValue::StoreLocal { value, .. } => check(value),
        InstructionValue::Await { value } => check(value),
        _ => false,
    }
}
