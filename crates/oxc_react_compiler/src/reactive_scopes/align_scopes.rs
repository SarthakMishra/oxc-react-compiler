#![allow(dead_code)]

use crate::hir::types::{HIR, InstructionValue, Terminal};

/// Align method call scopes: ensure receiver and method call are in the same scope.
pub fn align_method_call_scopes(hir: &mut HIR) {
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let InstructionValue::MethodCall { receiver, .. } = &instr.value {
                // If the receiver has a scope and the instruction's lvalue has a different scope,
                // extend the receiver's scope to encompass the method call.
                if let (Some(receiver_scope), Some(lvalue_scope)) =
                    (&receiver.identifier.scope, &mut instr.lvalue.identifier.scope)
                {
                    // Extend lvalue scope range to include receiver scope range
                    let new_start = lvalue_scope.range.start.0.min(receiver_scope.range.start.0);
                    let new_end = lvalue_scope.range.end.0.max(receiver_scope.range.end.0);
                    lvalue_scope.range.start.0 = new_start;
                    lvalue_scope.range.end.0 = new_end;
                }
            }
        }
    }
}

/// Align object method scopes: ensure object and its methods are in the same scope.
pub fn align_object_method_scopes(hir: &mut HIR) {
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let InstructionValue::ObjectExpression { properties } = &instr.value {
                // For each property value, if it has a scope, extend the object's scope
                for prop in properties {
                    if let (Some(prop_scope), Some(lvalue_scope)) =
                        (&prop.value.identifier.scope, &mut instr.lvalue.identifier.scope)
                    {
                        let new_start = lvalue_scope.range.start.0.min(prop_scope.range.start.0);
                        let new_end = lvalue_scope.range.end.0.max(prop_scope.range.end.0);
                        lvalue_scope.range.start.0 = new_start;
                        lvalue_scope.range.end.0 = new_end;
                    }
                }
            }
        }
    }
}

/// Align reactive scopes to block scopes in the HIR.
/// Ensures that reactive scopes don't cross block boundaries.
pub fn align_reactive_scopes_to_block_scopes_hir(hir: &mut HIR) {
    // Collect block instruction ranges to determine block boundaries
    let mut block_ranges: Vec<(crate::hir::types::BlockId, u32, u32)> = Vec::new();
    for (block_id, block) in &hir.blocks {
        if let (Some(first), Some(last)) = (block.instructions.first(), block.instructions.last()) {
            block_ranges.push((*block_id, first.id.0, last.id.0));
        }
    }

    // For each block, clamp any instruction's scope range to the block boundaries
    for (_, block) in &mut hir.blocks {
        let block_start = block.instructions.first().map(|i| i.id.0).unwrap_or(0);
        let block_end = block.instructions.last().map(|i| i.id.0 + 1).unwrap_or(0);

        for instr in &mut block.instructions {
            if let Some(ref mut scope) = instr.lvalue.identifier.scope {
                // Clamp scope range to not exceed the block
                if scope.range.start.0 < block_start {
                    scope.range.start.0 = block_start;
                }
                if scope.range.end.0 > block_end {
                    scope.range.end.0 = block_end;
                }
            }
        }
    }
}

/// Prune unused labels in the HIR.
pub fn prune_unused_labels_hir(hir: &mut HIR) {
    // Collect all labels referenced by break/continue terminals
    let mut used_labels = rustc_hash::FxHashSet::default();

    // In this IR, break/continue are modeled as gotos, so we check Label terminals
    // and see if any other terminal references their label ID.
    // For now, collect all label IDs that appear in Label terminals.
    let mut label_blocks: Vec<(crate::hir::types::BlockId, u32, crate::hir::types::BlockId)> =
        Vec::new();

    for (block_id, block) in &hir.blocks {
        if let Terminal::Label { block: _label_block, fallthrough, label } = &block.terminal {
            label_blocks.push((*block_id, *label, *fallthrough));
            // Mark label as used if any instruction in the label's body
            // references it (simplified: always keep labels for now)
            used_labels.insert(*label);
        }
    }

    // Replace Label terminals whose labels are never referenced with Goto
    // For a more complete implementation, we'd track break/continue targets.
    // Currently, we keep all labels since the IR doesn't have explicit break targets yet.
    let _ = used_labels;
    let _ = label_blocks;
}
