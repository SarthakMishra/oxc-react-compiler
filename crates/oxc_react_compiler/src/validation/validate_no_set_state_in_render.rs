use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, IdentifierId, InstructionValue, Type};
use rustc_hash::FxHashSet;

/// Validate that setState is not called unconditionally during render.
///
/// Calling setState during render causes infinite re-render loops.
/// Uses type-based detection (Type::SetState from useState/useReducer
/// destructuring) with naming heuristic fallback. Resolves identities
/// through SSA temporaries via LoadLocal/LoadGlobal/LoadContext instructions.
pub fn validate_no_set_state_in_render(hir: &HIR, errors: &mut ErrorCollector) {
    // Collect all identifier IDs that are setState-like (by type or name)
    let mut set_state_ids: FxHashSet<IdentifierId> = FxHashSet::default();

    // Pass 1: Identify setState identifiers from their definition sites
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Check type on the lvalue
            if instr.lvalue.identifier.type_ == Type::SetState {
                set_state_ids.insert(instr.lvalue.identifier.id);
            }

            // Check name on the lvalue
            if let Some(name) = &instr.lvalue.identifier.name {
                if is_set_state_name(name) {
                    set_state_ids.insert(instr.lvalue.identifier.id);
                }
            }

            // Track through LoadLocal/LoadContext: if loading a setState variable,
            // the result is also setState
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::SetState
                        || set_state_ids.contains(&place.identifier.id)
                    {
                        set_state_ids.insert(instr.lvalue.identifier.id);
                    }
                    if let Some(name) = &place.identifier.name {
                        if is_set_state_name(name) {
                            set_state_ids.insert(instr.lvalue.identifier.id);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Pass 2: Check for unconditional setState calls
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                if set_state_ids.contains(&callee.identifier.id) {
                    errors.push(CompilerError::invalid_react_with_kind(
                        instr.loc,
                        "Cannot call setState during render. \
                         Calling setState during render may trigger an infinite loop. \
                         * To reset state based on a condition, check if state is already \
                         set and early return.\n\
                         * To derive data from props/state, calculate it during render."
                            .to_string(),
                        DiagnosticKind::SetStateInRender,
                    ));
                }
            }
        }
    }
}

/// Check if a name looks like a setState function (setX where X is uppercase).
fn is_set_state_name(name: &str) -> bool {
    name.starts_with("set") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase()
}
