#![allow(dead_code)]

use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{InstructionValue, HIR};

/// Validate correct usage of `useMemo` and `useCallback`.
///
/// Checks:
/// 1. `useMemo` / `useCallback` must be called with exactly 2 arguments
///    (a callback and a dependency array).
/// 2. The callback argument should be a function expression (not an arbitrary value).
/// 3. The callback passed to `useMemo` should not be async.
pub fn validate_use_memo(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, args } = &instr.value {
                let name = match &callee.identifier.name {
                    Some(n) => n.as_str(),
                    None => continue,
                };

                if name != "useMemo" && name != "useCallback" {
                    continue;
                }

                // Check argument count: must be exactly 2 (callback + deps)
                if args.len() != 2 {
                    errors.push(CompilerError::invalid_react(
                        instr.loc,
                        format!(
                            "\"{}\" requires exactly 2 arguments (a callback and a dependency array), \
                             but received {}.",
                            name,
                            args.len()
                        ),
                    ));
                }

                // For useMemo specifically, check if the callback is async by
                // looking at the instruction that produced the first argument.
                // We scan backwards from this instruction to find the definition
                // of the callback place.
                if name == "useMemo" && !args.is_empty() {
                    let callback_id = args[0].identifier.id;
                    check_memo_callback_async(hir, callback_id, instr.loc, errors);
                }
            }
        }
    }
}

/// Check if the function expression producing the given identifier is async.
fn check_memo_callback_async(
    hir: &HIR,
    callback_id: crate::hir::types::IdentifierId,
    call_loc: crate::hir::types::SourceLocation,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                if lowered_func.is_async {
                    errors.push(CompilerError::invalid_react(
                        call_loc,
                        "useMemo callback must not be async. \
                         The callback should return a value synchronously."
                            .to_string(),
                    ));
                }
            }
        }
    }
}
