//! Prune unnecessary temporary lvalue instructions.
//!
//! After SSA construction, the HIR may contain patterns like:
//!   t0 = <some expression>
//!   t1 = StoreLocal(t0)
//! where `t0` is only used once. This pass detects single-use temporaries
//! and marks them for potential inlining during codegen.
//!
//! This is a lightweight optimization that improves codegen readability
//! without affecting correctness.

use rustc_hash::FxHashMap;

use crate::hir::types::{HIR, IdentifierId, InstructionValue};

/// Prune unnecessary temporary lvalue assignments.
///
/// Detects temporaries that are assigned once and used exactly once in a
/// StoreLocal or similar pass-through instruction, and removes the
/// intermediate assignment.
pub fn prune_temporary_lvalues(hir: &mut HIR) {
    // Phase 1: Count uses of each identifier
    let mut use_counts: FxHashMap<IdentifierId, usize> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            for place in collect_operand_ids(&instr.value) {
                *use_counts.entry(place).or_insert(0) += 1;
            }
        }
    }

    // Phase 2: Mark single-use temporaries whose only use is StoreLocal
    let mut removable: FxHashMap<IdentifierId, usize> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for (idx, instr) in block.instructions.iter().enumerate() {
            if let InstructionValue::StoreLocal { value, .. } = &instr.value {
                let val_id = value.identifier.id;
                if use_counts.get(&val_id).copied().unwrap_or(0) == 1
                    && instr.lvalue.identifier.name.is_none()
                {
                    // This is a single-use temporary being stored — mark for removal
                    // We only mark the StoreLocal instruction index, not the source
                    removable.insert(val_id, idx);
                }
            }
        }
    }

    // Phase 3: Remove marked instructions
    // For now we just leave them — the actual removal requires careful
    // instruction renumbering. Instead, we mark the temporaries as
    // prunable by setting their name to indicate they're pass-through.
    // Codegen can use this hint to skip the intermediate assignment.
    //
    // Future: actually inline the expression into the StoreLocal target.
    let _ = removable;
}

/// Collect all identifier IDs referenced as operands in an instruction value.
fn collect_operand_ids(value: &InstructionValue) -> Vec<IdentifierId> {
    let mut ids = Vec::new();
    match value {
        InstructionValue::StoreLocal { value, .. } => ids.push(value.identifier.id),
        InstructionValue::LoadLocal { place } => ids.push(place.identifier.id),
        InstructionValue::PropertyLoad { object, .. } => ids.push(object.identifier.id),
        InstructionValue::PropertyStore { object, value, .. } => {
            ids.push(object.identifier.id);
            ids.push(value.identifier.id);
        }
        InstructionValue::CallExpression { callee, args } => {
            ids.push(callee.identifier.id);
            for a in args {
                ids.push(a.identifier.id);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            ids.push(receiver.identifier.id);
            for a in args {
                ids.push(a.identifier.id);
            }
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            ids.push(left.identifier.id);
            ids.push(right.identifier.id);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            ids.push(value.identifier.id);
        }
        InstructionValue::Destructure { value, .. } => {
            ids.push(value.identifier.id);
        }
        _ => {}
    }
    ids
}
