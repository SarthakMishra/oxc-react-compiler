use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, IdentifierId, InstructionValue, Place};
use rustc_hash::FxHashMap;

/// Known effect hook names.
const EFFECT_HOOKS: &[&str] = &["useEffect", "useLayoutEffect", "useInsertionEffect"];

/// Validate that derived state is not computed inside effects.
///
/// A common anti-pattern is to read props/state inside an effect and then call
/// `setState` with a value derived from them. This should instead be done
/// during render (e.g., with `useMemo`) so React can batch the update.
pub fn validate_no_derived_computations_in_effects(hir: &HIR, errors: &mut ErrorCollector) {
    // Build id-to-name map for resolving SSA temporaries
    let id_to_name = build_name_map(hir);

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, args } = &instr.value {
                let name = callee
                    .identifier
                    .name
                    .as_deref()
                    .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));

                let Some(name) = name else { continue };

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
    _hook_name: &str,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }

            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                // Build name map for the inner function body
                let inner_name_map = build_name_map(&lowered_func.body);
                // Collect setState identifiers in the callback
                let set_state_ids = collect_set_state_ids(&lowered_func.body);

                // Walk the function body looking for setState calls
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        if let InstructionValue::CallExpression {
                            callee: inner_callee,
                            args: inner_args,
                        } = &inner_instr.value
                            && (is_set_state_call(inner_callee, &inner_name_map)
                                || set_state_ids.contains(&inner_callee.identifier.id))
                            && !inner_args.is_empty()
                        {
                            errors.push(CompilerError::invalid_react_with_kind(
                                inner_instr.loc,
                                format!(
                                    "Values derived from props and state should be calculated \
                                     during render, not in an effect. \
                                     (https://react.dev/learn/you-might-not-need-an-effect)"
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
fn is_set_state_call(place: &Place, name_map: &FxHashMap<IdentifierId, String>) -> bool {
    let name = place
        .identifier
        .name
        .as_deref()
        .or_else(|| name_map.get(&place.identifier.id).map(String::as_str));
    if let Some(name) = name {
        if name.starts_with("set") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase() {
            return true;
        }
    }
    place.identifier.type_ == crate::hir::types::Type::SetState
}

/// Collect all setState identifier IDs in an HIR body.
fn collect_set_state_ids(hir: &HIR) -> rustc_hash::FxHashSet<IdentifierId> {
    use crate::hir::types::Type;
    let mut set_state_ids = rustc_hash::FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.type_ == Type::SetState {
                set_state_ids.insert(instr.lvalue.identifier.id);
            }
            if let Some(name) = &instr.lvalue.identifier.name {
                if name.starts_with("set")
                    && name.len() > 3
                    && name.as_bytes()[3].is_ascii_uppercase()
                {
                    set_state_ids.insert(instr.lvalue.identifier.id);
                }
            }
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::SetState
                        || set_state_ids.contains(&place.identifier.id)
                    {
                        set_state_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                _ => {}
            }
        }
    }

    set_state_ids
}

/// Build a map from identifier ID → name for SSA resolution.
fn build_name_map(hir: &HIR) -> FxHashMap<IdentifierId, String> {
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadGlobal { binding } => {
                    id_to_name.insert(instr.lvalue.identifier.id, binding.name.clone());
                }
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name.clone());
                    }
                }
                _ => {}
            }
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_name.entry(instr.lvalue.identifier.id).or_insert_with(|| name.clone());
            }
        }
    }

    id_to_name
}
