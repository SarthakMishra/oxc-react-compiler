#![allow(dead_code)]

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue, Place};

/// Validate that setState is not called unconditionally during render.
///
/// Calling setState during render causes infinite re-render loops.
/// We check for calls to functions matching the `setX` naming convention
/// that appear at the top level (not inside callbacks or event handlers).
pub fn validate_no_set_state_in_render(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value
                && is_set_state_call(callee) {
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

/// Detect if a place refers to a setState-like function.
///
/// Matches the common React convention where `useState` returns `[state, setState]`
/// and the setter is named `setX` where X starts with an uppercase letter.
fn is_set_state_call(place: &Place) -> bool {
    place.identifier.name.as_deref().is_some_and(|name| {
        // Common patterns: setX, setState
        name.starts_with("set") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase()
    })
}
