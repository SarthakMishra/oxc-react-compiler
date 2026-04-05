// DIVERGENCE: Upstream relies on StoreContext instructions (populated when the
// HIR builder is aware of outer-scope captures via context_vars) to detect
// reassignment of module-level variables from within component/hook bodies.
// Our HIR builder now populates context_vars for nested function builders,
// so nested functions correctly emit StoreContext/LoadContext for captured vars.
// We still use a name-based approach for the top-level component scope: collect
// locally declared variables and flag any StoreLocal/StoreContext/Reassign
// targeting undeclared names.

use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{
    HIR, HIRFunction, IdSet, IdVec, IdentifierId, InstructionKind, InstructionValue, Param,
};
use rustc_hash::FxHashSet;

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
pub fn validate_no_global_reassignment(
    hir: &HIR,
    errors: &mut ErrorCollector,
    param_names: &[String],
) {
    // Collect names declared at the component's top-level scope.
    let mut component_locals = collect_locally_declared_hir(hir);
    // Also include function parameter names -- these are local to the component
    // but not present in the HIR body instructions (they're in HIRFunction.params).
    for name in param_names {
        component_locals.insert(name.clone());
    }

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

/// Recursively collect id-to-name mappings from all blocks including nested functions.
fn collect_id_names_recursive(hir: &HIR, id_to_name: &mut IdVec<IdentifierId, String>) {
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
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    collect_id_names_recursive(&lowered_func.body, id_to_name);
                }
                _ => {}
            }
            if let Some(name) = &instr.lvalue.identifier.name
                && !id_to_name.contains_key(instr.lvalue.identifier.id)
            {
                id_to_name.insert(instr.lvalue.identifier.id, name.clone());
            }
        }
    }
}

/// Recursively collect safe callback IDs from all blocks including nested functions.
fn collect_safe_ids_recursive(
    hir: &HIR,
    id_to_name: &IdVec<IdentifierId, String>,
    safe_ids: &mut IdSet<IdentifierId>,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::CallExpression { callee, args, .. } => {
                    let callee_name = callee
                        .identifier
                        .name
                        .as_deref()
                        .or_else(|| id_to_name.get(callee.identifier.id).map(String::as_str));
                    if let Some(name) = callee_name
                        && EFFECT_HOOKS.contains(&name)
                        && !args.is_empty()
                    {
                        safe_ids.insert(args[0].identifier.id);
                    }
                    if let Some(name) = callee_name
                        && name == "useCallback"
                        && !args.is_empty()
                    {
                        safe_ids.insert(args[0].identifier.id);
                    }
                }
                InstructionValue::MethodCall { property, args, .. } => {
                    if EFFECT_HOOKS.contains(&property.as_str()) && !args.is_empty() {
                        safe_ids.insert(args[0].identifier.id);
                    }
                    if property == "useCallback" && !args.is_empty() {
                        safe_ids.insert(args[0].identifier.id);
                    }
                }
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
                // Recurse into nested function bodies to find JSX event handlers
                // and effect callbacks inside useMemo/useCallback/etc.
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    collect_safe_ids_recursive(&lowered_func.body, id_to_name, safe_ids);
                }
                _ => {}
            }
        }
    }
}

/// Recursively collect id alias mappings from all blocks including nested functions.
fn collect_id_aliases_recursive(hir: &HIR, id_aliases: &mut IdVec<IdentifierId, IdentifierId>) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    id_aliases.insert(instr.lvalue.identifier.id, place.identifier.id);
                }
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    id_aliases.insert(lvalue.identifier.id, value.identifier.id);
                }
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    collect_id_aliases_recursive(&lowered_func.body, id_aliases);
                }
                _ => {}
            }
        }
    }
}

fn collect_safe_callback_ids(hir: &HIR) -> IdSet<IdentifierId> {
    let mut safe_ids: IdSet<IdentifierId> = IdSet::new();

    // Build id-to-name map to resolve callee identifiers.
    let mut id_to_name: IdVec<IdentifierId, String> = IdVec::new();
    collect_id_names_recursive(hir, &mut id_to_name);

    // Scan all blocks (including nested function bodies) for safe callback patterns.
    collect_safe_ids_recursive(hir, &id_to_name, &mut safe_ids);

    // Propagate safety through LoadLocal/StoreLocal chains
    let mut id_aliases: IdVec<IdentifierId, IdentifierId> = IdVec::new();
    collect_id_aliases_recursive(hir, &mut id_aliases);

    // Walk alias chains
    let safe_copy: Vec<IdentifierId> =
        safe_ids.iter_indices().map(|idx| IdentifierId(idx as u32)).collect();
    for id in safe_copy {
        let mut current = id;
        for _ in 0..10 {
            if let Some(&alias) = id_aliases.get(current) {
                safe_ids.insert(alias);
                current = alias;
            } else {
                break;
            }
        }
    }

    safe_ids
}

/// Collect names of functions that are safe callbacks (event handlers, effects).
fn collect_safe_callback_names(hir: &HIR) -> FxHashSet<String> {
    let safe_ids = collect_safe_callback_ids(hir);
    let mut id_to_name: IdVec<IdentifierId, String> = IdVec::new();
    collect_id_names_recursive(hir, &mut id_to_name);

    let mut names = FxHashSet::default();
    for (idx, name) in id_to_name.iter() {
        if safe_ids.contains(IdentifierId(idx as u32)) {
            names.insert(name.clone());
        }
    }
    // Also collect names from StoreLocal targets that alias safe IDs
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreLocal { lvalue, value, .. } = &instr.value
                && safe_ids.contains(value.identifier.id)
                && let Some(name) = &lvalue.identifier.name
            {
                names.insert(name.clone());
            }
        }
    }
    names
}

/// Check all blocks in an HIR for global/outer-scope reassignments,
/// recursing into nested function expressions.
fn check_blocks(hir: &HIR, component_locals: &FxHashSet<String>, errors: &mut ErrorCollector) {
    let safe_callback_ids = collect_safe_callback_ids(hir);
    let safe_callback_names = collect_safe_callback_names(hir);

    // Build a mapping from temp IDs to their assigned variable names.
    let mut id_to_assigned_name: IdVec<IdentifierId, String> = IdVec::new();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreLocal { lvalue, value, .. } = &instr.value
                && let Some(name) = &lvalue.identifier.name
            {
                id_to_assigned_name.insert(value.identifier.id, name.clone());
            }
        }
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Explicit StoreGlobal (for well-known globals)
            if let InstructionValue::StoreGlobal { name, .. } = &instr.value {
                errors
                    .push(CompilerError::invalid_react(instr.loc, global_reassignment_error(name)));
            }

            // StoreLocal with Reassign on undeclared names
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

            // PostfixUpdate/PrefixUpdate on undeclared names
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

            // Recurse into nested function bodies
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    let id = instr.lvalue.identifier.id;
                    let is_safe_by_id = safe_callback_ids.contains(id);
                    let assigned_name = id_to_assigned_name.get(id);
                    let lvalue_name = instr.lvalue.identifier.name.as_ref();
                    let is_safe_by_name = assigned_name
                        .or(lvalue_name)
                        .is_some_and(|n| safe_callback_names.contains(n));
                    if !is_safe_by_id && !is_safe_by_name {
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
fn check_nested_for_outer_scope_stores(
    func: &HIRFunction,
    all_ancestor_locals: &FxHashSet<String>,
    safe_callback_ids: &IdSet<IdentifierId>,
    errors: &mut ErrorCollector,
) {
    let nested_locals = collect_locally_declared_func(func);

    // Also collect safe callback IDs within this function body
    let nested_safe_ids = collect_safe_callback_ids(&func.body);

    // Build: lvalue ID -> variable name for LoadLocal of undeclared vars
    let mut undeclared_load_ids: IdVec<IdentifierId, String> = IdVec::new();

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
                InstructionValue::StoreLocal {
                    lvalue,
                    type_: Some(InstructionKind::Reassign),
                    ..
                }
                | InstructionValue::StoreLocal { lvalue, type_: None, .. }
                | InstructionValue::StoreContext { lvalue, .. } => {
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
                InstructionValue::PropertyStore { object, .. }
                | InstructionValue::ComputedStore { object, .. } => {
                    if let Some(name) = undeclared_load_ids.get(object.identifier.id) {
                        errors.push(CompilerError::invalid_react(
                            instr.loc,
                            global_reassignment_error(name),
                        ));
                    }
                }
                InstructionValue::MethodCall { receiver, .. } => {
                    if let Some(name) = undeclared_load_ids.get(receiver.identifier.id) {
                        errors.push(CompilerError::invalid_react(
                            instr.loc,
                            global_reassignment_error(name),
                        ));
                    }
                }
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
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    let id = instr.lvalue.identifier.id;
                    if !safe_callback_ids.contains(id) && !nested_safe_ids.contains(id) {
                        let mut merged = all_ancestor_locals.clone();
                        merged.extend(nested_locals.iter().cloned());
                        check_nested_for_outer_scope_stores(
                            lowered_func,
                            &merged,
                            safe_callback_ids,
                            errors,
                        );

                        if body_contains_jsx(&lowered_func.body) {
                            check_render_helper_global_mutations(&lowered_func.body, errors);
                        }
                    }
                }
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

/// Check a render helper for PropertyStore/MethodCall on globals.
fn check_render_helper_global_mutations(hir: &HIR, errors: &mut ErrorCollector) {
    let mut global_load_ids: IdVec<IdentifierId, String> = IdVec::new();

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
                    if let Some(name) = global_load_ids.get(object.identifier.id) {
                        errors.push(CompilerError::invalid_react(
                            instr.loc,
                            global_reassignment_error(name),
                        ));
                    }
                }
                InstructionValue::MethodCall { receiver, .. } => {
                    if let Some(name) = global_load_ids.get(receiver.identifier.id) {
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
