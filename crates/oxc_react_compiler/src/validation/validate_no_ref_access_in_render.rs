use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, IdentifierId, InstructionValue, Type};
use rustc_hash::FxHashSet;

/// Validate that ref values are not accessed during render.
///
/// Accessing `.current` on a ref during render can cause tearing
/// because refs are mutable and not tracked by React.
///
/// Uses both type-based detection (Type::Ref from useRef() calls) and
/// naming heuristic fallback. Resolves identities through SSA temporaries.
pub fn validate_no_ref_access_in_render(hir: &HIR, errors: &mut ErrorCollector) {
    // Collect all identifier IDs that are ref-like (by type or name)
    let mut ref_ids: FxHashSet<IdentifierId> = FxHashSet::default();

    // Pass 1: Identify ref identifiers from their definition sites
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Check type on the lvalue
            if instr.lvalue.identifier.type_ == Type::Ref {
                ref_ids.insert(instr.lvalue.identifier.id);
            }

            // Check name on the lvalue
            if let Some(name) = &instr.lvalue.identifier.name {
                if is_ref_name(name) {
                    ref_ids.insert(instr.lvalue.identifier.id);
                }
            }

            // Track through LoadLocal/LoadContext: if loading a ref variable,
            // the result is also a ref
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::Ref || ref_ids.contains(&place.identifier.id)
                    {
                        ref_ids.insert(instr.lvalue.identifier.id);
                    }
                    if let Some(name) = &place.identifier.name {
                        if is_ref_name(name) {
                            ref_ids.insert(instr.lvalue.identifier.id);
                        }
                    }
                }
                // Track through PropertyLoad: `props.ref` or `x.someRef` produces a ref
                InstructionValue::PropertyLoad { property, .. } => {
                    if is_ref_name(property) {
                        ref_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                _ => {}
            }
        }
    }

    // Pass 2: Check for ref.current access (read or write)
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let is_ref_current = match &instr.value {
                InstructionValue::PropertyLoad { object, property } => {
                    property == "current" && ref_ids.contains(&object.identifier.id)
                }
                InstructionValue::PropertyStore { object, property, .. } => {
                    property == "current" && ref_ids.contains(&object.identifier.id)
                }
                _ => false,
            };
            if is_ref_current {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    "Cannot access refs during render. \
                     React refs are values that are not needed for rendering. \
                     Refs should only be accessed in effects or event handlers."
                        .to_string(),
                    DiagnosticKind::RefAccessInRender,
                ));
            }
        }
    }
}

/// Check if a name looks like a React ref.
fn is_ref_name(name: &str) -> bool {
    name.ends_with("Ref") || name.ends_with("ref") || name == "ref"
}
