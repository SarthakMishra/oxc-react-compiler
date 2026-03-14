// DIVERGENCE: Upstream relies on StoreContext instructions (populated when the
// HIR builder is aware of outer-scope captures via context_vars) to detect
// reassignment of module-level variables from within component/hook bodies.
// Our HIR builder does not populate context_vars for nested function builders,
// so module-level assignments in nested functions produce StoreLocal (not
// StoreContext or StoreGlobal). We use a name-based approach: collect locally
// declared variables and flag any StoreLocal/Reassign targeting undeclared names.

use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{HIR, HIRFunction, IdentifierId, InstructionKind, InstructionValue, Param};
use rustc_hash::{FxHashMap, FxHashSet};

fn global_reassignment_error(name: &str) -> String {
    format!(
        "Cannot reassign variables declared outside of the component/hook. \
         Variable `{name}` is declared outside of the component/hook \
         and cannot be reassigned during render."
    )
}

/// Validate that component/hook functions do not reassign global or
/// module-level variables.
///
/// Reassigning variables declared outside of the component or hook can cause
/// inconsistent behavior between renders, since React may re-render components
/// in any order and at any time.
pub fn validate_no_global_reassignment(hir: &HIR, errors: &mut ErrorCollector) {
    // Collect names declared at the component's top-level scope.
    // This includes function parameters (emitted as DeclareLocal by the builder).
    let component_locals = collect_locally_declared_hir(hir);

    check_blocks(hir, &component_locals, errors);
}

/// Collect all variable names declared within an HIR scope.
fn collect_locally_declared_hir(hir: &HIR) -> FxHashSet<String> {
    let mut declared = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(name) = &lvalue.identifier.name {
                        declared.insert(name.clone());
                    }
                }
                // StoreLocal with Let/Const/Var type_ is a declaration
                InstructionValue::StoreLocal {
                    lvalue,
                    type_:
                        Some(InstructionKind::Let | InstructionKind::Const | InstructionKind::Var),
                    ..
                } => {
                    if let Some(name) = &lvalue.identifier.name {
                        declared.insert(name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    declared
}

/// Collect all variable names declared in an HIRFunction (including params).
fn collect_locally_declared_func(func: &HIRFunction) -> FxHashSet<String> {
    let mut declared = collect_locally_declared_hir(&func.body);

    // Add function parameters
    for param in &func.params {
        let place = match param {
            Param::Identifier(p) | Param::Spread(p) => p,
        };
        if let Some(name) = &place.identifier.name {
            declared.insert(name.clone());
        }
    }

    declared
}

/// Hook names whose first callback argument is considered a "non-render" context
/// where global mutations are allowed. Matches upstream behavior.
const EFFECT_HOOKS: &[&str] = &["useEffect", "useLayoutEffect", "useInsertionEffect"];

/// Collect IDs of function expressions that are used in "safe" (non-render) contexts:
/// - First argument to useEffect / useLayoutEffect / useInsertionEffect
/// - Value of a JSX prop (event handler like onClick)
/// - First argument to useCallback (callback body)
///
/// Global mutations inside these functions are allowed because they don't execute
/// during render — they run in effects or in response to user events.
fn collect_safe_callback_ids(hir: &HIR) -> FxHashSet<IdentifierId> {
    let mut safe_ids: FxHashSet<IdentifierId> = FxHashSet::default();

    // Build id-to-name map to resolve callee identifiers
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadGlobal { binding } => {
                    id_to_name.insert(instr.lvalue.identifier.id, binding.name.clone());
                }
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name.clone());
                    }
                }
                _ => {}
            }
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_name.entry(instr.lvalue.identifier.id).or_insert_with(|| name.clone());
            }
        }
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                // useEffect(callback, deps) — callback is safe
                InstructionValue::CallExpression { callee, args } => {
                    let callee_name = callee
                        .identifier
                        .name
                        .as_deref()
                        .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));
                    if let Some(name) = callee_name
                        && EFFECT_HOOKS.contains(&name)
                        && !args.is_empty()
                    {
                        safe_ids.insert(args[0].identifier.id);
                    }
                    // useCallback(callback) — callback body is a non-render context
                    if let Some(name) = callee_name
                        && name == "useCallback"
                        && !args.is_empty()
                    {
                        safe_ids.insert(args[0].identifier.id);
                    }
                }
                // React.useEffect(callback) — method call form
                InstructionValue::MethodCall { property, args, .. } => {
                    if EFFECT_HOOKS.contains(&property.as_str()) && !args.is_empty() {
                        safe_ids.insert(args[0].identifier.id);
                    }
                    if property == "useCallback" && !args.is_empty() {
                        safe_ids.insert(args[0].identifier.id);
                    }
                }
                // JSX event handler props: <div onClick={handler}> — handler is safe
                // We filter to props that look like event handlers (onXxx) since
                // other prop values (className, value, etc.) are not callbacks.
                InstructionValue::JsxExpression { props, .. } => {
                    for attr in props {
                        let is_event_handler = match &attr.name {
                            crate::hir::types::JsxAttributeName::Named(name) => {
                                name.starts_with("on")
                                    && name.len() > 2
                                    && name.as_bytes()[2].is_ascii_uppercase()
                            }
                            _ => false,
                        };
                        if is_event_handler {
                            safe_ids.insert(attr.value.identifier.id);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Propagate safety through LoadLocal/StoreLocal chains:
    // If `const cb = () => { ... }; useEffect(cb)`, then cb's function expr
    // ID differs from the ID passed to useEffect. We need to trace through
    // StoreLocal → LoadLocal chains.
    let mut id_aliases: FxHashMap<IdentifierId, IdentifierId> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    // lvalue.id aliases place.id
                    id_aliases.insert(instr.lvalue.identifier.id, place.identifier.id);
                }
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    // lvalue.id aliases value.id
                    id_aliases.insert(lvalue.identifier.id, value.identifier.id);
                }
                _ => {}
            }
        }
    }

    // Walk alias chains: if a safe ID maps back to a function expression,
    // also mark the function expression's lvalue as safe
    let safe_copy: Vec<IdentifierId> = safe_ids.iter().copied().collect();
    for id in safe_copy {
        let mut current = id;
        for _ in 0..10 {
            // depth limit to avoid cycles
            if let Some(&alias) = id_aliases.get(&current) {
                safe_ids.insert(alias);
                current = alias;
            } else {
                break;
            }
        }
    }

    safe_ids
}

/// Check all blocks in an HIR for global/outer-scope reassignments,
/// recursing into nested function expressions.
fn check_blocks(hir: &HIR, component_locals: &FxHashSet<String>, errors: &mut ErrorCollector) {
    // DIVERGENCE: Upstream allows global mutations inside effect callbacks and
    // JSX event handlers. We collect IDs of function expressions used in safe
    // contexts and skip global mutation checks inside those functions.
    let safe_callback_ids = collect_safe_callback_ids(hir);

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Explicit StoreGlobal (for well-known globals)
            if let InstructionValue::StoreGlobal { name, .. } = &instr.value {
                errors
                    .push(CompilerError::invalid_react(instr.loc, global_reassignment_error(name)));
            }

            // StoreLocal with Reassign on undeclared names (e.g., x = ... where x is global)
            // This catches destructuring assignments to global variables like [x] = props
            if let InstructionValue::StoreLocal {
                lvalue,
                type_: Some(InstructionKind::Reassign) | None,
                ..
            } = &instr.value
                && let Some(name) = &lvalue.identifier.name
                && !component_locals.contains(name)
            {
                errors
                    .push(CompilerError::invalid_react(instr.loc, global_reassignment_error(name)));
            }

            // PostfixUpdate/PrefixUpdate on undeclared names (e.g., renderCount++)
            // Upstream emits a Todo error for UpdateExpression on globals.
            match &instr.value {
                InstructionValue::PostfixUpdate { lvalue, .. }
                | InstructionValue::PrefixUpdate { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name
                        && !component_locals.contains(name)
                    {
                        errors.push(CompilerError::todo(
                            instr.loc,
                            "(BuildHIR::lowerExpression) Support UpdateExpression where \
                             argument is a global"
                                .to_string(),
                        ));
                    }
                }
                _ => {}
            }

            // Recurse into nested function bodies — check for outer-scope stores
            // Skip functions used in safe contexts (effects, JSX event handlers)
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    if !safe_callback_ids.contains(&instr.lvalue.identifier.id) {
                        check_nested_for_outer_scope_stores(
                            lowered_func,
                            component_locals,
                            &safe_callback_ids,
                            errors,
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

/// Check a nested function body for stores to variables not declared locally.
/// Any StoreLocal/Reassign targeting a name not in the nested function's locals
/// AND not in any ancestor scope is a module-scope reassignment.
/// `all_ancestor_locals` includes all variable names from the component and any
/// intermediate function scopes (prevents false positives on intermediate captures).
/// `safe_callback_ids` contains IDs of functions used in safe (non-render) contexts.
fn check_nested_for_outer_scope_stores(
    func: &HIRFunction,
    all_ancestor_locals: &FxHashSet<String>,
    safe_callback_ids: &FxHashSet<IdentifierId>,
    errors: &mut ErrorCollector,
) {
    let nested_locals = collect_locally_declared_func(func);

    // Also collect safe callback IDs within this function body
    let nested_safe_ids = collect_safe_callback_ids(&func.body);

    // Build: lvalue ID → variable name for LoadLocal of undeclared vars
    // Used to detect PropertyStore/MethodCall on outer-scope variables
    let mut undeclared_load_ids: FxHashMap<IdentifierId, String> = FxHashMap::default();

    for (_, block) in &func.body.blocks {
        for instr in &block.instructions {
            if let InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } =
                &instr.value
                && let Some(name) = &place.identifier.name
                && !nested_locals.contains(name)
                && !all_ancestor_locals.contains(name)
            {
                undeclared_load_ids.insert(instr.lvalue.identifier.id, name.clone());
            }
        }
    }

    for (_, block) in &func.body.blocks {
        for instr in &block.instructions {
            match &instr.value {
                // StoreLocal with Reassign type_ on a name not declared in this function
                // and not in the component's locals — it's a module-scope variable
                InstructionValue::StoreLocal {
                    lvalue,
                    type_: Some(InstructionKind::Reassign),
                    ..
                }
                | InstructionValue::StoreLocal { lvalue, type_: None, .. } => {
                    if let Some(name) = &lvalue.identifier.name
                        && !nested_locals.contains(name)
                        && !all_ancestor_locals.contains(name)
                    {
                        errors.push(CompilerError::invalid_react(
                            instr.loc,
                            global_reassignment_error(name),
                        ));
                    }
                }
                // PropertyStore/ComputedStore where object is an undeclared outer var
                InstructionValue::PropertyStore { object, .. }
                | InstructionValue::ComputedStore { object, .. } => {
                    if let Some(name) = undeclared_load_ids.get(&object.identifier.id) {
                        errors.push(CompilerError::invalid_react(
                            instr.loc,
                            global_reassignment_error(name),
                        ));
                    }
                }
                // MethodCall where receiver is an undeclared outer var
                InstructionValue::MethodCall { receiver, .. } => {
                    if let Some(name) = undeclared_load_ids.get(&receiver.identifier.id) {
                        errors.push(CompilerError::invalid_react(
                            instr.loc,
                            global_reassignment_error(name),
                        ));
                    }
                }
                // PostfixUpdate/PrefixUpdate on outer-scope names
                InstructionValue::PostfixUpdate { lvalue, .. }
                | InstructionValue::PrefixUpdate { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name
                        && !nested_locals.contains(name)
                        && !all_ancestor_locals.contains(name)
                    {
                        errors.push(CompilerError::todo(
                            instr.loc,
                            "(BuildHIR::lowerExpression) Support UpdateExpression where \
                             argument is a global"
                                .to_string(),
                        ));
                    }
                }
                // Recurse deeper into nested functions, passing all ancestor scopes
                // Skip functions used in safe contexts (effects, JSX event handlers)
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    let id = instr.lvalue.identifier.id;
                    if !safe_callback_ids.contains(&id) && !nested_safe_ids.contains(&id) {
                        // Merge current function's locals into ancestor set for deeper recursion
                        let mut merged = all_ancestor_locals.clone();
                        merged.extend(nested_locals.iter().cloned());
                        check_nested_for_outer_scope_stores(
                            lowered_func,
                            &merged,
                            safe_callback_ids,
                            errors,
                        );

                        // Render helper detection: if this nested function returns JSX,
                        // also check for PropertyStore/MethodCall on global variables
                        if body_contains_jsx(&lowered_func.body) {
                            check_render_helper_global_mutations(&lowered_func.body, errors);
                        }
                    }
                }
                // Explicit StoreGlobal in nested context
                InstructionValue::StoreGlobal { name, .. } => {
                    errors.push(CompilerError::invalid_react(
                        instr.loc,
                        global_reassignment_error(name),
                    ));
                }
                _ => {}
            }
        }
    }
}

/// Check if an HIR body contains any JSX expression (render helper detection).
fn body_contains_jsx(hir: &HIR) -> bool {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if matches!(
                instr.value,
                InstructionValue::JsxExpression { .. } | InstructionValue::JsxFragment { .. }
            ) {
                return true;
            }
        }
    }
    false
}

/// Check a render helper (function returning JSX) for PropertyStore/MethodCall on globals.
/// DIVERGENCE: Upstream uses `ValidateNoSetStateInRender` with render helper detection
/// via abstract interpretation. We approximate by scanning for LoadGlobal → PropertyStore chains.
fn check_render_helper_global_mutations(hir: &HIR, errors: &mut ErrorCollector) {
    // Build: lvalue ID → global name for LoadGlobal values
    let mut global_load_ids: FxHashMap<IdentifierId, String> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::LoadGlobal { binding } = &instr.value {
                global_load_ids.insert(instr.lvalue.identifier.id, binding.name.clone());
            }
        }
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::PropertyStore { object, .. }
                | InstructionValue::ComputedStore { object, .. } => {
                    if let Some(name) = global_load_ids.get(&object.identifier.id) {
                        errors.push(CompilerError::invalid_react(
                            instr.loc,
                            global_reassignment_error(name),
                        ));
                    }
                }
                InstructionValue::MethodCall { receiver, .. } => {
                    if let Some(name) = global_load_ids.get(&receiver.identifier.id) {
                        errors.push(CompilerError::invalid_react(
                            instr.loc,
                            global_reassignment_error(name),
                        ));
                    }
                }
                _ => {}
            }
        }
    }
}
