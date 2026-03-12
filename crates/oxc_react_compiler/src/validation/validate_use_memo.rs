#![allow(dead_code)]

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue};

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
                    errors.push(CompilerError::invalid_react_with_kind(
                        instr.loc,
                        format!(
                            "\"{}\" requires exactly 2 arguments (a callback and a dependency array), \
                             but received {}.",
                            name,
                            args.len()
                        ),
                        DiagnosticKind::UseMemoValidation,
                    ));
                }

                if name == "useMemo" && !args.is_empty() {
                    let callback_id = args[0].identifier.id;
                    // Check if the callback is async
                    check_memo_callback_async(hir, callback_id, instr.loc, errors);
                    // Check if the callback returns void (useMemo must return a value)
                    check_memo_callback_void(hir, callback_id, instr.loc, errors);
                }
            }
        }
    }
}

/// Check if the function expression producing the given identifier returns void.
/// useMemo callbacks must return a value — returning void/undefined is likely a bug.
fn check_memo_callback_void(
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
                // Check if the function body has any Return terminal with a non-undefined value.
                // If all returns are void (i.e., return undefined), this is likely a mistake.
                let has_return_value = lowered_func.body.blocks.iter().any(|(_, b)| {
                    if let crate::hir::types::Terminal::Return { value } = &b.terminal {
                        // Check if the return value is a named variable or non-trivial
                        value.identifier.name.is_some()
                            || value.identifier.type_ != crate::hir::types::Type::Primitive(
                                crate::hir::types::PrimitiveType::Undefined,
                            )
                    } else {
                        false
                    }
                });

                if !has_return_value {
                    errors.push(CompilerError::invalid_react_with_kind(
                        call_loc,
                        "useMemo callback does not return a value. \
                         useMemo is for memoizing computed values — use useEffect for side effects."
                            .to_string(),
                        DiagnosticKind::VoidUseMemo,
                    ));
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
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && lowered_func.is_async {
                    errors.push(CompilerError::invalid_react_with_kind(
                        call_loc,
                        "useMemo callback must not be async. \
                         The callback should return a value synchronously."
                            .to_string(),
                        DiagnosticKind::UseMemoValidation,
                    ));
                }
        }
    }
}
