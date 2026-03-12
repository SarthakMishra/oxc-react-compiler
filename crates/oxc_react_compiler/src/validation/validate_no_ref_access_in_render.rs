
use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue, Place, Type};

/// Validate that ref values are not accessed during render.
///
/// Accessing `.current` on a ref during render can cause tearing
/// because refs are mutable and not tracked by React.
///
/// Uses both type-based detection (Type::Ref from useRef() calls) and
/// naming heuristic fallback for cases where type information is unavailable.
pub fn validate_no_ref_access_in_render(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::PropertyLoad { object, property } = &instr.value
                && property == "current"
                && is_ref(object)
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    "Accessing ref.current during render. \
                     Refs are mutable and reading them during render can cause tearing."
                        .to_string(),
                    DiagnosticKind::RefAccessInRender,
                ));
            }
        }
    }
}

/// Detect if a place is a React ref, using type information first,
/// then falling back to naming heuristic.
fn is_ref(place: &Place) -> bool {
    // Type-based detection: infer_types marks useRef() return values as Type::Ref
    if place.identifier.type_ == Type::Ref {
        return true;
    }

    // Naming heuristic fallback for cases where type inference didn't run
    // or the ref was passed in as a prop (no call site to infer from)
    place
        .identifier
        .name
        .as_deref()
        .is_some_and(|name| name.ends_with("Ref") || name.ends_with("ref") || name == "ref")
}
