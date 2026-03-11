#![allow(dead_code)]

use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{InstructionValue, Place, HIR};

/// Validate that ref values are not accessed during render.
///
/// Accessing `.current` on a ref during render can cause tearing
/// because refs are mutable and not tracked by React.
pub fn validate_no_ref_access_in_render(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::PropertyLoad { object, property } = &instr.value {
                if property == "current" && is_likely_ref(object) {
                    errors.push(CompilerError::invalid_react(
                        instr.loc,
                        "Accessing ref.current during render. \
                         Refs are mutable and reading them during render can cause tearing."
                            .to_string(),
                    ));
                }
            }
        }
    }
}

/// Heuristic to detect if a place is likely a React ref.
///
/// In a full implementation, this would use type information from the
/// environment to determine if the place was produced by `useRef`.
/// For now, we use naming conventions.
fn is_likely_ref(place: &Place) -> bool {
    place.identifier.name.as_deref().map_or(false, |name| {
        name.ends_with("Ref") || name.ends_with("ref") || name == "ref"
    })
}
