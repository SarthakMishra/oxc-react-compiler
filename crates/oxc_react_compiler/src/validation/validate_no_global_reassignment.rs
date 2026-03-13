use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue};

/// Validate that component/hook functions do not reassign global or
/// module-level variables.
///
/// Reassigning variables declared outside of the component or hook can cause
/// inconsistent behavior between renders, since React may re-render components
/// in any order and at any time.
pub fn validate_no_global_reassignment(hir: &HIR, errors: &mut ErrorCollector) {
    check_blocks(hir, errors);
}

/// Check all blocks in an HIR for StoreGlobal instructions, recursing into
/// nested function expressions.
fn check_blocks(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreGlobal { name, .. } = &instr.value {
                errors.push(CompilerError::invalid_react(
                    instr.loc,
                    format!(
                        "Cannot reassign variables declared outside of the component/hook. \
                         Variable `{name}` is declared outside of the component/hook \
                         and cannot be reassigned during render."
                    ),
                ));
            }

            // Recurse into nested function bodies
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    check_blocks(&lowered_func.body, errors);
                }
                _ => {}
            }
        }
    }
}
