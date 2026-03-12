use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue, Place, Type};

/// Validate that setState is not called unconditionally during render.
///
/// Calling setState during render causes infinite re-render loops.
/// Uses type-based detection (Type::SetState from useState/useReducer
/// destructuring) with naming heuristic fallback.
pub fn validate_no_set_state_in_render(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value
                && is_set_state(callee)
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    "setState is called unconditionally during render. \
                     This will cause an infinite re-render loop."
                        .to_string(),
                    DiagnosticKind::SetStateInRender,
                ));
            }
        }
    }
}

/// Detect if a place refers to a setState-like function, using type
/// information first, then falling back to naming heuristic.
fn is_set_state(place: &Place) -> bool {
    // Type-based detection: infer_types marks useState/useReducer setter as Type::SetState
    if place.identifier.type_ == Type::SetState {
        return true;
    }

    // Naming heuristic fallback: matches `setX` where X is uppercase
    place.identifier.name.as_deref().is_some_and(|name| {
        name.starts_with("set") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase()
    })
}
