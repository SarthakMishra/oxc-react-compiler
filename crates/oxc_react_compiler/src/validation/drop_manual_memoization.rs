#![allow(dead_code)]

use crate::hir::types::{HIR, InstructionValue};

/// Remove manual memoization markers (StartMemoize/FinishMemoize)
/// when the configuration says not to preserve them.
///
/// This is a conditional pass that runs when
/// `enable_preserve_existing_memoization_guarantees` is false.
pub fn drop_manual_memoization(hir: &mut HIR) {
    for (_, block) in hir.blocks.iter_mut() {
        block.instructions.retain(|instr| {
            !matches!(
                &instr.value,
                InstructionValue::StartMemoize { .. } | InstructionValue::FinishMemoize { .. }
            )
        });
    }
}
