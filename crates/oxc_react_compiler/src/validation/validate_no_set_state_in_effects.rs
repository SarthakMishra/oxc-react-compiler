#![allow(dead_code)]

use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{InstructionValue, Place, HIR};

/// Known effect hook names.
const EFFECT_HOOKS: &[&str] = &["useEffect", "useLayoutEffect", "useInsertionEffect"];

/// Validate that synchronous setState is not called directly in effect bodies.
///
/// Calling setState synchronously in an effect body (not inside a callback or
/// promise `.then`) causes an extra re-render on every commit. If the state
/// update is needed, it should typically be derived during render or wrapped
/// in a condition.
pub fn validate_no_set_state_in_effects(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, args } = &instr.value {
                let name = match &callee.identifier.name {
                    Some(n) => n.as_str(),
                    None => continue,
                };

                if !EFFECT_HOOKS.contains(&name) {
                    continue;
                }

                // The first argument to an effect hook is the callback.
                if let Some(callback_place) = args.first() {
                    let callback_id = callback_place.identifier.id;
                    check_effect_body_for_set_state(hir, callback_id, name, errors);
                }
            }
        }
    }
}

/// Given the identifier of an effect callback, find its function body and
/// check for direct (synchronous) setState calls at the top level of that body.
fn check_effect_body_for_set_state(
    hir: &HIR,
    callback_id: crate::hir::types::IdentifierId,
    hook_name: &str,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }

            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                // Only check top-level instructions in the effect callback body.
                // setState inside nested function expressions (event handlers,
                // promise callbacks) is acceptable.
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        // Skip nested function expressions — setState inside those is OK
                        if matches!(
                            &inner_instr.value,
                            InstructionValue::FunctionExpression { .. }
                        ) {
                            continue;
                        }

                        if let InstructionValue::CallExpression {
                            callee: inner_callee,
                            ..
                        } = &inner_instr.value
                        {
                            if is_set_state_call(inner_callee) {
                                errors.push(CompilerError::invalid_react(
                                    inner_instr.loc,
                                    format!(
                                        "setState is called directly inside \"{}\". \
                                         Synchronous setState in effects causes an extra \
                                         re-render. Consider deriving the value during render \
                                         or moving the update into a callback.",
                                        hook_name
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Detect if a place refers to a setState-like function.
fn is_set_state_call(place: &Place) -> bool {
    place.identifier.name.as_deref().map_or(false, |name| {
        name.starts_with("set") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase()
    })
}
