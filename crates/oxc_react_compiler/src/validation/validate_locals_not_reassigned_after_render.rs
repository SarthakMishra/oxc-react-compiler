use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionKind, InstructionValue};
use rustc_hash::FxHashSet;

/// Validate that local variables assigned during render are not reassigned
/// in event handlers, effects, or async functions.
///
/// Variables initialized during render carry reactive semantics. If they are
/// later reassigned inside a callback passed to `useEffect` or an event handler,
/// the compiler cannot safely memoize them because the mutation happens outside
/// the render phase.
///
/// Also detects reassignment in async functions, which always execute after
/// render completes (upstream: "Cannot reassign variable in async function").
pub fn validate_locals_not_reassigned_after_render(hir: &HIR, errors: &mut ErrorCollector) {
    // Step 1: Collect variables assigned during render (top-level block instructions).
    let render_assigned = collect_render_assigned(hir);

    // Step 2: Track which function names (by lvalue name) reassign render variables.
    // This implements a simplified version of upstream's `reassigningFunctions` map.
    // TODO: use reassigning_funcs for propagation through CallExpression args
    let mut reassigning_funcs: FxHashSet<String> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    // Check if the nested function body reassigns any render-phase variables
                    let is_async = lowered_func.is_async;
                    if check_nested_reassignments_silent(
                        &lowered_func.body,
                        &render_assigned,
                        is_async,
                    ) {
                        // This function reassigns render vars — track it
                        if let Some(name) = &instr.lvalue.identifier.name {
                            reassigning_funcs.insert(name.clone());
                        }

                        // If async, emit the async-specific error immediately
                        if is_async {
                            errors.push(CompilerError::invalid_react_with_kind(
                                instr.loc,
                                "Cannot reassign variable in async function. \
                                 Reassigning a variable in an async function can cause \
                                 inconsistent behavior because the async function may \
                                 continue to run after the component has been updated."
                                    .to_string(),
                                DiagnosticKind::LocalsReassignedAfterRender,
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Step 3: Check for function expressions (non-async) that directly reassign
    // render variables — emit errors for those found in event handlers/effects.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && !lowered_func.is_async
            {
                check_nested_reassignments(&lowered_func.body, &render_assigned, errors);
            }
        }
    }
}

/// Collect names of variables that are assigned at the top level (render phase).
fn collect_render_assigned(hir: &HIR) -> FxHashSet<String> {
    let mut assigned = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::DeclareLocal { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name {
                        assigned.insert(name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    assigned
}

/// Check a nested HIR (function body) for reassignments of render-phase variables.
/// Returns true if any reassignment is found (does NOT emit errors).
///
/// Only matches actual reassignments (`type_: Some(Reassign)` or `type_: None`),
/// NOT new declarations that shadow render-phase variables.
fn check_nested_reassignments_silent(
    nested_hir: &HIR,
    render_assigned: &FxHashSet<String>,
    check_deeply: bool,
) -> bool {
    for (_, block) in &nested_hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreLocal { lvalue, type_, .. } = &instr.value
                && let Some(name) = &lvalue.identifier.name
                && render_assigned.contains(name)
                && is_reassignment_kind(*type_)
            {
                return true;
            }

            // For async functions, check deeper into nested callbacks
            // (e.g., await foo().then(result => { value = result; }))
            if check_deeply
                && let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && check_nested_reassignments_silent(&lowered_func.body, render_assigned, true)
            {
                return true;
            }
        }
    }
    false
}

/// Check a nested HIR (function body) for reassignments of render-phase variables.
/// Emits errors for each reassignment found. Recurses into nested functions.
///
/// Only matches actual reassignments (`type_: Some(Reassign)` or `type_: None`),
/// NOT new declarations that shadow render-phase variables.
fn check_nested_reassignments(
    nested_hir: &HIR,
    render_assigned: &FxHashSet<String>,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &nested_hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreLocal { lvalue, type_, .. } = &instr.value
                && let Some(name) = &lvalue.identifier.name
                && render_assigned.contains(name)
                && is_reassignment_kind(*type_)
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    format!(
                        "Local variable \"{name}\" is assigned during render but \
                                 reassigned inside a nested function (effect or event handler). \
                                 This prevents the compiler from memoizing correctly."
                    ),
                    DiagnosticKind::LocalsReassignedAfterRender,
                ));
            }

            // Recurse into nested functions (e.g., callbacks within callbacks)
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                check_nested_reassignments(&lowered_func.body, render_assigned, errors);
            }
        }
    }
}

/// Returns true if the StoreLocal type indicates a real reassignment (not a new
/// declaration that happens to shadow a render variable).
fn is_reassignment_kind(type_: Option<InstructionKind>) -> bool {
    matches!(type_, Some(InstructionKind::Reassign) | None)
}
