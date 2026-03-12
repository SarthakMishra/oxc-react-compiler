
use crate::hir::types::HIR;

/// Remove unnecessary `MaybeThrow` terminals from the HIR.
///
/// After lowering, many blocks have `MaybeThrow` terminals that are not
/// actually needed (e.g., simple assignments that cannot throw). This pass
/// replaces them with direct `Goto` terminals to simplify the CFG.
pub fn prune_maybe_throws(hir: &mut HIR) {
    for (_id, block) in &mut hir.blocks {
        if let crate::hir::types::Terminal::MaybeThrow { continuation, handler: _ } =
            &block.terminal
        {
            // If the block contains no instructions that can throw,
            // replace MaybeThrow with a direct Goto to the continuation.
            let can_throw = block.instructions.iter().any(|instr| {
                matches!(
                    instr.value,
                    crate::hir::types::InstructionValue::CallExpression { .. }
                        | crate::hir::types::InstructionValue::MethodCall { .. }
                        | crate::hir::types::InstructionValue::NewExpression { .. }
                        | crate::hir::types::InstructionValue::PropertyLoad { .. }
                        | crate::hir::types::InstructionValue::ComputedLoad { .. }
                        | crate::hir::types::InstructionValue::Await { .. }
                        | crate::hir::types::InstructionValue::GetIterator { .. }
                        | crate::hir::types::InstructionValue::IteratorNext { .. }
                )
            });

            if !can_throw {
                let cont = *continuation;
                block.terminal = crate::hir::types::Terminal::Goto { block: cont };
            }
        }
    }
}
