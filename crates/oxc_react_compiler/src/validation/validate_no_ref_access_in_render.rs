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
/// Also scans nested function bodies (FunctionExpression, ObjectMethod)
/// for ref.current access, since callbacks invoked during render or passed
/// to hooks like useReducer/useState also count as render-time access.
pub fn validate_no_ref_access_in_render(hir: &HIR, errors: &mut ErrorCollector) {
    // Collect all identifier IDs that are ref-like (by type or name)
    let mut ref_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    // Also collect ref variable names for cross-scope tracking
    let mut ref_names: FxHashSet<String> = FxHashSet::default();

    // Pass 1: Identify ref identifiers from their definition sites
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Check type on the lvalue
            if instr.lvalue.identifier.type_ == Type::Ref {
                ref_ids.insert(instr.lvalue.identifier.id);
                if let Some(name) = &instr.lvalue.identifier.name {
                    ref_names.insert(name.clone());
                }
            }

            // Check name on the lvalue
            if let Some(name) = &instr.lvalue.identifier.name
                && is_ref_name(name)
            {
                ref_ids.insert(instr.lvalue.identifier.id);
                ref_names.insert(name.clone());
            }

            // Track through LoadLocal/LoadContext: if loading a ref variable,
            // the result is also a ref
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::Ref || ref_ids.contains(&place.identifier.id)
                    {
                        ref_ids.insert(instr.lvalue.identifier.id);
                    }
                    if let Some(name) = &place.identifier.name
                        && (is_ref_name(name) || ref_names.contains(name))
                    {
                        ref_ids.insert(instr.lvalue.identifier.id);
                        ref_names.insert(name.clone());
                    }
                }
                // Track through StoreLocal: if storing a ref value, the target is a ref
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    if ref_ids.contains(&value.identifier.id) {
                        ref_ids.insert(instr.lvalue.identifier.id);
                        if let Some(name) = &lvalue.identifier.name {
                            ref_names.insert(name.clone());
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

    // Pass 2: Check for ref.current access (read or write) at top level
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
                return;
            }

            // Also scan nested function bodies for ref.current access.
            // Callbacks passed to hooks (useReducer, useState) or invoked during
            // render can access ref.current — this must also be flagged.
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func } => {
                    if check_nested_ref_access(&lowered_func.body, &ref_names) {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            "Cannot access refs during render. \
                             React refs are values that are not needed for rendering. \
                             Refs should only be accessed in effects or event handlers."
                                .to_string(),
                            DiagnosticKind::RefAccessInRender,
                        ));
                        return;
                    }
                }
                _ => {}
            }
        }
    }
}

/// Check if a nested function body accesses ref.current for any ref name
/// from the outer scope. Uses name-based tracking because nested functions
/// have their own HIR with fresh IdentifierIds.
fn check_nested_ref_access(hir: &HIR, outer_ref_names: &FxHashSet<String>) -> bool {
    // Build local ref_ids within this nested function
    let mut local_ref_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    // Track ref names within this scope (outer refs + any local aliases)
    let mut local_ref_names: FxHashSet<String> = outer_ref_names.clone();

    // Pass 1: Identify ref identifiers in the nested function
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Check type on the lvalue
            if instr.lvalue.identifier.type_ == Type::Ref {
                local_ref_ids.insert(instr.lvalue.identifier.id);
                if let Some(name) = &instr.lvalue.identifier.name {
                    local_ref_names.insert(name.clone());
                }
            }

            // Check name on the lvalue
            if let Some(name) = &instr.lvalue.identifier.name
                && (is_ref_name(name) || local_ref_names.contains(name))
            {
                local_ref_ids.insert(instr.lvalue.identifier.id);
                local_ref_names.insert(name.clone());
            }

            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::Ref
                        || local_ref_ids.contains(&place.identifier.id)
                    {
                        local_ref_ids.insert(instr.lvalue.identifier.id);
                    }
                    if let Some(name) = &place.identifier.name
                        && (is_ref_name(name) || local_ref_names.contains(name))
                    {
                        local_ref_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                // Track aliases: const aliasedRef = ref
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    if local_ref_ids.contains(&value.identifier.id) {
                        local_ref_ids.insert(instr.lvalue.identifier.id);
                        if let Some(name) = &lvalue.identifier.name {
                            local_ref_names.insert(name.clone());
                        }
                    }
                }
                InstructionValue::PropertyLoad { property, .. } => {
                    if is_ref_name(property) {
                        local_ref_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                _ => {}
            }
        }
    }

    // Pass 2: Check for ref.current access
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let is_ref_current = match &instr.value {
                InstructionValue::PropertyLoad { object, property } => {
                    property == "current" && local_ref_ids.contains(&object.identifier.id)
                }
                InstructionValue::PropertyStore { object, property, .. } => {
                    property == "current" && local_ref_ids.contains(&object.identifier.id)
                }
                _ => false,
            };
            if is_ref_current {
                return true;
            }

            // Recurse into further nested functions
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func } => {
                    if check_nested_ref_access(&lowered_func.body, &local_ref_names) {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }

    false
}

/// Check if a name looks like a React ref.
fn is_ref_name(name: &str) -> bool {
    name.ends_with("Ref") || name.ends_with("ref") || name == "ref"
}
