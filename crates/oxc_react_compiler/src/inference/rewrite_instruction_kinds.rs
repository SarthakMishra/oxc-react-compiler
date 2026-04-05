use crate::hir::types::{HIR, IdSet, IdVec, IdentifierId, InstructionKind, InstructionValue};

/// Rewrite instruction kinds based on variable reassignment.
///
/// If a `const` binding is later reassigned (due to SSA transformation),
/// downgrade it to `let`. This handles cases where SSA renamed a `const`
/// variable that was in different branches.
pub fn rewrite_instruction_kinds_based_on_reassignment(hir: &mut HIR) {
    // Phase 1: Find identifiers that are assigned more than once
    let mut assignment_counts: IdVec<IdentifierId, u32> = IdVec::new();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::StoreLocal { lvalue, .. } => {
                    *assignment_counts.entry_or_insert_with(lvalue.identifier.id, || 0) += 1;
                }
                InstructionValue::DeclareLocal { lvalue, .. } => {
                    *assignment_counts.entry_or_insert_with(lvalue.identifier.id, || 0) += 1;
                }
                _ => {}
            }
        }
    }

    // Phase 2: Collect identifiers assigned more than once
    let mut reassigned: IdSet<IdentifierId> = IdSet::new();
    for (idx, count) in assignment_counts.iter() {
        if *count > 1 {
            reassigned.insert(IdentifierId(idx as u32));
        }
    }

    if reassigned.is_empty() {
        return;
    }

    // Phase 3: Downgrade const to let for reassigned variables
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let InstructionValue::DeclareLocal { lvalue, type_ } = &mut instr.value
                && reassigned.contains(lvalue.identifier.id)
                && *type_ == InstructionKind::Const
            {
                *type_ = InstructionKind::Let;
            }
        }
    }
}
