use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionKind, InstructionValue};
use crate::validation::function_context;
use rustc_hash::FxHashSet;

/// Validate that local variables assigned during render are not reassigned
/// in post-render contexts (effect hooks, event handlers, async functions)
/// or in functions that escape the render phase.
///
/// Functions that are ONLY called directly during render (never escape to
/// hooks, JSX props, or return values) are render-only — their direct
/// reassignments are safe. We still recurse into their nested FEs though,
/// since those may themselves escape.
pub fn validate_locals_not_reassigned_after_render(hir: &HIR, errors: &mut ErrorCollector) {
    let render_assigned = collect_render_assigned(hir);
    let directly_called = function_context::collect_directly_called_fe_ids(hir);
    let post_render_ids = function_context::collect_post_render_fn_ids(hir);
    let self_shadowing = function_context::has_self_shadowing(hir);

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    let fe_id = instr.lvalue.identifier.id;

                    // Async functions always execute after render
                    if lowered_func.is_async {
                        if check_nested_reassignments_silent(
                            &lowered_func.body,
                            &render_assigned,
                            true,
                        ) {
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
                        continue;
                    }

                    // A FE is render-only if:
                    // 1. It is directly called at render time
                    // 2. It is NOT in any post-render context
                    // 3. It does NOT shadow its own variable name internally
                    let is_render_only = directly_called.contains(&fe_id)
                        && !post_render_ids.contains(&fe_id)
                        && !self_shadowing.contains(&fe_id);

                    if is_render_only {
                        // Render-only: skip direct reassignment checks but still
                        // recurse into nested FEs (they may escape independently)
                        check_nested_reassignments_recurse_only(
                            &lowered_func.body,
                            &render_assigned,
                            errors,
                        );
                    } else {
                        // Escaping or post-render: check all reassignments
                        check_nested_reassignments(&lowered_func.body, &render_assigned, errors);
                    }
                }
                _ => {}
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

/// Check a nested HIR for reassignments of render-phase variables.
/// Returns true if any reassignment is found (silent — no errors emitted).
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

/// For render-only functions: skip direct reassignment checks but recurse into
/// nested function expressions (which may themselves escape and reassign).
fn check_nested_reassignments_recurse_only(
    nested_hir: &HIR,
    render_assigned: &FxHashSet<String>,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &nested_hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                check_nested_reassignments(&lowered_func.body, render_assigned, errors);
            }
        }
    }
}

/// Check a nested HIR for reassignments or mutations of render-phase variables.
/// Emits errors for each reassignment/mutation found. Recurses into nested functions.
fn check_nested_reassignments(
    nested_hir: &HIR,
    render_assigned: &FxHashSet<String>,
    errors: &mut ErrorCollector,
) {
    // Build a map from identifier ID to original name for LoadLocal/LoadContext
    // so we can resolve MethodCall receivers back to their original names.
    let mut id_to_name: rustc_hash::FxHashMap<crate::hir::types::IdentifierId, String> =
        rustc_hash::FxHashMap::default();
    for (_, block) in &nested_hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } =
                &instr.value
                && let Some(name) = &place.identifier.name
            {
                id_to_name.insert(instr.lvalue.identifier.id, name.clone());
            }
        }
    }

    for (_, block) in &nested_hir.blocks {
        for instr in &block.instructions {
            // Check 1: StoreLocal reassignment of render-assigned variable
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

            // Check 2: MethodCall mutation on render-assigned variable
            // e.g. `cache.set('key', 'value')` where `cache` was assigned at render.
            // The receiver is typically an unnamed temp loaded via LoadLocal/LoadContext;
            // resolve it to the original name.
            if let InstructionValue::MethodCall { receiver, property, .. } = &instr.value
                && is_known_mutating_method(property)
            {
                let receiver_name = receiver.identifier.name.as_deref().or_else(|| {
                    id_to_name.get(&receiver.identifier.id).map(std::string::String::as_str)
                });
                if let Some(name) = receiver_name
                    && render_assigned.contains(name)
                {
                    errors.push(CompilerError::invalid_react_with_kind(
                        instr.loc,
                        format!(
                            "Cannot modify local variables after render completes. \
                             This argument is a function which may reassign or mutate \
                             `{name}` after render, which can cause inconsistent behavior \
                             on subsequent renders. Consider using state instead."
                        ),
                        DiagnosticKind::LocalsReassignedAfterRender,
                    ));
                }
            }

            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                check_nested_reassignments(&lowered_func.body, render_assigned, errors);
            }
        }
    }
}

/// Check if a method name is known to mutate its receiver.
fn is_known_mutating_method(name: &str) -> bool {
    matches!(
        name,
        "set"
            | "add"
            | "delete"
            | "clear"
            | "push"
            | "pop"
            | "shift"
            | "unshift"
            | "splice"
            | "sort"
            | "reverse"
            | "fill"
            | "copyWithin"
    )
}

fn is_reassignment_kind(type_: Option<InstructionKind>) -> bool {
    matches!(type_, Some(InstructionKind::Reassign) | None)
}
