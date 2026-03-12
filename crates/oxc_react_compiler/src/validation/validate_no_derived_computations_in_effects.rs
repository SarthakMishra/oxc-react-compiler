
use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue, Place};

/// Known effect hook names.
const EFFECT_HOOKS: &[&str] = &["useEffect", "useLayoutEffect", "useInsertionEffect"];

/// Validate that derived state is not computed inside effects.
///
/// A common anti-pattern is to read props/state inside an effect and then call
/// `setState` with a value derived from them. This should instead be done
/// during render (e.g., with `useMemo`) so React can batch the update.
pub fn validate_no_derived_computations_in_effects(hir: &HIR, errors: &mut ErrorCollector) {
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
                // Find the function expression that defines it.
                if let Some(callback_place) = args.first() {
                    let callback_id = callback_place.identifier.id;
                    check_effect_callback_for_derived_state(hir, callback_id, name, errors);
                }
            }
        }
    }
}

/// Given the identifier of an effect callback, find its function body and check
/// for setState calls that appear to derive from props/state.
fn check_effect_callback_for_derived_state(
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
                // Walk the function body looking for setState calls
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        if let InstructionValue::CallExpression {
                            callee: inner_callee,
                            args: inner_args,
                        } = &inner_instr.value
                            && is_set_state_call(inner_callee) && !inner_args.is_empty() {
                                errors.push(CompilerError::invalid_react_with_kind(
                                    inner_instr.loc,
                                    format!(
                                        "Derived computation inside \"{hook_name}\". \
                                         setState is called with a value that may be derived \
                                         from props or state. Compute derived values during \
                                         render instead (e.g., with useMemo)."
                                    ),
                                    DiagnosticKind::DerivedComputationsInEffects,
                                ));
                            }
                    }
                }
            }
        }
    }
}

/// Detect if a place refers to a setState-like function.
fn is_set_state_call(place: &Place) -> bool {
    place.identifier.name.as_deref().is_some_and(|name| {
        name.starts_with("set") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase()
    })
}
