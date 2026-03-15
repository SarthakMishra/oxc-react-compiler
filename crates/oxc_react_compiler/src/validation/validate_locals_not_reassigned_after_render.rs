use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, IdentifierId, InstructionKind, InstructionValue};
use rustc_hash::{FxHashMap, FxHashSet};

/// Validate that local variables assigned during render are not reassigned
/// in event handlers, effects, or async functions.
///
/// Variables initialized during render carry reactive semantics. If they are
/// later reassigned inside a callback passed to `useEffect` or an event handler,
/// the compiler cannot safely memoize them because the mutation happens outside
/// the render phase.
///
/// Closures that are ONLY called directly during render (and never escape) are
/// fine — the reassignment happens during render, not after. We skip those.
///
/// Also detects reassignment in async functions, which always execute after
/// render completes (upstream: "Cannot reassign variable in async function").
pub fn validate_locals_not_reassigned_after_render(hir: &HIR, errors: &mut ErrorCollector) {
    // Step 1: Collect variables assigned during render (top-level block instructions).
    let render_assigned = collect_render_assigned(hir);

    // Step 2: Build a map from FunctionExpression lvalue IDs to the variable names
    // they get stored into (e.g., FuncExpr id(18) → "fn" via StoreLocal).
    let func_expr_to_var_name = build_func_expr_to_var_name(hir);

    // Step 3: Determine which function variable names are "render-only" — called
    // directly during render and never escape.
    let render_only_fns = collect_render_only_fn_names(hir, &func_expr_to_var_name);

    // Step 4: Check each function expression for reassignments.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    let is_async = lowered_func.is_async;

                    // Async functions always execute after render — check unconditionally
                    if is_async {
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

                    // Check if this function expression is stored to a render-only variable.
                    // Render-only functions can safely reassign render variables directly,
                    // but we still need to recurse into their nested function expressions
                    // (which may themselves escape via return values, etc.).
                    let is_render_only = func_expr_to_var_name
                        .get(&instr.lvalue.identifier.id)
                        .is_some_and(|var_name| render_only_fns.contains(var_name.as_str()));

                    if is_render_only {
                        // Only recurse into nested functions — don't flag direct reassignments
                        check_nested_reassignments_recurse_only(
                            &lowered_func.body,
                            &render_assigned,
                            errors,
                        );
                    } else {
                        // Check for all nested reassignments (direct + recursive)
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

/// Build a mapping from FunctionExpression lvalue IDs to the variable names
/// they get stored into via subsequent StoreLocal instructions.
///
/// E.g., for `const fn = function() { ... }`:
///   FunctionExpression → lvalue id(18) (temporary, no name)
///   StoreLocal { lvalue: id(19)/name="fn", value: id(18) }
/// This produces: id(18) → "fn"
fn build_func_expr_to_var_name(hir: &HIR) -> FxHashMap<IdentifierId, String> {
    let mut func_expr_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if matches!(
                instr.value,
                InstructionValue::FunctionExpression { .. } | InstructionValue::ObjectMethod { .. }
            ) {
                func_expr_ids.insert(instr.lvalue.identifier.id);
            }
        }
    }

    let mut mapping: FxHashMap<IdentifierId, String> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreLocal { lvalue, value, .. } = &instr.value
                && func_expr_ids.contains(&value.identifier.id)
                && let Some(name) = &lvalue.identifier.name
            {
                mapping.insert(value.identifier.id, name.clone());
            }
        }
    }

    mapping
}

/// Determine which function variable names are "render-only" — called directly
/// during render and never escape (never passed as arguments, JSX props, etc.).
///
/// Uses names because SSA renaming preserves names but changes IDs.
fn collect_render_only_fn_names(
    hir: &HIR,
    func_expr_to_var_name: &FxHashMap<IdentifierId, String>,
) -> FxHashSet<String> {
    let fn_var_names: FxHashSet<&str> =
        func_expr_to_var_name.values().map(String::as_str).collect();
    if fn_var_names.is_empty() {
        return FxHashSet::default();
    }

    // Map each identifier ID to a function variable name if it refers to one.
    // LoadLocal { place: name="fn" } → lvalue id(20) maps to "fn"
    let mut id_to_fn_var: FxHashMap<IdentifierId, String> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } =
                &instr.value
                && let Some(name) = &place.identifier.name
                && fn_var_names.contains(name.as_str())
            {
                id_to_fn_var.insert(instr.lvalue.identifier.id, name.clone());
            }
        }
    }

    let mut called_directly: FxHashSet<String> = FxHashSet::default();
    let mut escaping: FxHashSet<String> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::CallExpression { callee, args } => {
                    if let Some(name) = id_to_fn_var.get(&callee.identifier.id) {
                        called_directly.insert(name.clone());
                    }
                    for arg in args {
                        if let Some(name) = id_to_fn_var.get(&arg.identifier.id) {
                            escaping.insert(name.clone());
                        }
                    }
                }
                InstructionValue::MethodCall { args, .. } => {
                    for arg in args {
                        if let Some(name) = id_to_fn_var.get(&arg.identifier.id) {
                            escaping.insert(name.clone());
                        }
                    }
                }
                InstructionValue::JsxExpression { props, children, .. } => {
                    for attr in props {
                        if let Some(name) = id_to_fn_var.get(&attr.value.identifier.id) {
                            escaping.insert(name.clone());
                        }
                    }
                    for child in children {
                        if let Some(name) = id_to_fn_var.get(&child.identifier.id) {
                            escaping.insert(name.clone());
                        }
                    }
                }
                InstructionValue::ArrayExpression { elements } => {
                    for elem in elements {
                        let place = match elem {
                            crate::hir::types::ArrayElement::Expression(p)
                            | crate::hir::types::ArrayElement::Spread(p) => Some(p),
                            crate::hir::types::ArrayElement::Hole => None,
                        };
                        if let Some(p) = place
                            && let Some(name) = id_to_fn_var.get(&p.identifier.id)
                        {
                            escaping.insert(name.clone());
                        }
                    }
                }
                InstructionValue::ObjectExpression { properties } => {
                    for prop in properties {
                        if let Some(name) = id_to_fn_var.get(&prop.value.identifier.id) {
                            escaping.insert(name.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Check if any function shadows its own name with a local variable inside
    // its body. If so, the function is NOT render-only because the shadowed
    // variable reassignment should still be flagged.
    let mut self_shadowing: FxHashSet<&str> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && let Some(var_name) = func_expr_to_var_name.get(&instr.lvalue.identifier.id)
                && fn_var_names.contains(var_name.as_str())
            {
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        if let InstructionValue::StoreLocal { lvalue, .. }
                        | InstructionValue::DeclareLocal { lvalue, .. } = &inner_instr.value
                            && let Some(inner_name) = &lvalue.identifier.name
                            && inner_name == var_name
                        {
                            self_shadowing.insert(var_name.as_str());
                        }
                    }
                }
            }
        }
    }

    // Render-only = called directly AND doesn't escape AND doesn't self-shadow
    fn_var_names
        .into_iter()
        .filter(|name| {
            called_directly.contains(*name)
                && !escaping.contains(*name)
                && !self_shadowing.contains(*name)
        })
        .map(str::to_string)
        .collect()
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
