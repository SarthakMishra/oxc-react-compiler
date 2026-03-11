#![allow(dead_code)]

use crate::hir::types::{IdentifierId, InstructionKind, InstructionValue, HIR};
use rustc_hash::FxHashSet;

/// Rewrite instruction kinds based on variable reassignment.
///
/// If a `const` binding is later reassigned (due to SSA transformation),
/// downgrade it to `let`. This handles cases where SSA renamed a `const`
/// variable that was in different branches.
pub fn rewrite_instruction_kinds_based_on_reassignment(hir: &mut HIR) {
    // Phase 1: Find identifiers that are assigned more than once
    let mut assignment_counts: rustc_hash::FxHashMap<IdentifierId, u32> =
        rustc_hash::FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::StoreLocal { lvalue, .. } => {
                    *assignment_counts.entry(lvalue.identifier.id).or_insert(0) += 1;
                }
                InstructionValue::DeclareLocal { lvalue, .. } => {
                    *assignment_counts.entry(lvalue.identifier.id).or_insert(0) += 1;
                }
                _ => {}
            }
        }
    }

    // Phase 2: Collect identifiers assigned more than once
    let reassigned: FxHashSet<IdentifierId> = assignment_counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(id, _)| id)
        .collect();

    if reassigned.is_empty() {
        return;
    }

    // Phase 3: Downgrade const to let for reassigned variables
    for (_, block) in hir.blocks.iter_mut() {
        for instr in &mut block.instructions {
            if let InstructionValue::DeclareLocal { lvalue, type_ } = &mut instr.value {
                if reassigned.contains(&lvalue.identifier.id) && *type_ == InstructionKind::Const {
                    *type_ = InstructionKind::Let;
                }
            }
        }
    }
}
