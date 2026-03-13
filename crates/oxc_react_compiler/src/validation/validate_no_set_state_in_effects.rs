use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, IdentifierId, InstructionValue, Type};
use rustc_hash::{FxHashMap, FxHashSet};

/// Known effect hook names.
const EFFECT_HOOKS: &[&str] = &["useEffect", "useLayoutEffect", "useInsertionEffect"];

/// Validate that synchronous setState is not called directly in effect bodies.
///
/// Calling setState synchronously in an effect body (not inside a callback or
/// promise `.then`) causes an extra re-render on every commit.
pub fn validate_no_set_state_in_effects(hir: &HIR, errors: &mut ErrorCollector) {
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

                let hook_name = name.to_string();

                // The first argument to an effect hook is the callback.
                if let Some(callback_place) = args.first() {
                    let callback_id = callback_place.identifier.id;
                    check_effect_body_for_set_state(hir, callback_id, &hook_name, errors);
                }
            }
        }
    }
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

/// Collect all setState identifier IDs in an HIR body.
fn collect_set_state_ids(hir: &HIR) -> FxHashSet<IdentifierId> {
    let mut set_state_ids: FxHashSet<IdentifierId> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.type_ == Type::SetState {
                set_state_ids.insert(instr.lvalue.identifier.id);
            }
            if let Some(name) = &instr.lvalue.identifier.name
                && is_set_state_name(name)
            {
                set_state_ids.insert(instr.lvalue.identifier.id);
            }
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::SetState
                        || set_state_ids.contains(&place.identifier.id)
                    {
                        set_state_ids.insert(instr.lvalue.identifier.id);
                    }
                    if let Some(name) = &place.identifier.name
                        && is_set_state_name(name)
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

/// Given the identifier of an effect callback, find its function body and
/// check for direct (synchronous) setState calls at the top level of that body.
fn check_effect_body_for_set_state(
    hir: &HIR,
    callback_id: IdentifierId,
    hook_name: &str,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }

            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                // Collect setState identifiers in the effect callback body
                let set_state_ids = collect_set_state_ids(&lowered_func.body);

                // Only check top-level instructions in the effect callback body.
                // setState inside nested function expressions (event handlers,
                // promise callbacks) is acceptable.
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        // Skip nested function expressions — setState inside those is OK
                        if matches!(&inner_instr.value, InstructionValue::FunctionExpression { .. })
                        {
                            continue;
                        }

                        if let InstructionValue::CallExpression { callee: inner_callee, .. } =
                            &inner_instr.value
                            && set_state_ids.contains(&inner_callee.identifier.id)
                        {
                            errors.push(CompilerError::invalid_react_with_kind(
                                inner_instr.loc,
                                format!(
                                    "setState is called directly inside \"{hook_name}\". \
                                         Synchronous setState in effects causes an extra \
                                         re-render. Consider deriving the value during render \
                                         or moving the update into a callback."
                                ),
                                DiagnosticKind::SetStateInEffects,
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Check if a name looks like a setState function (setX where X is uppercase).
fn is_set_state_name(name: &str) -> bool {
    name.starts_with("set") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase()
}
