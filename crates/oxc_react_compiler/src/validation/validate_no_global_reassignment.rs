// DIVERGENCE: Upstream relies on StoreContext instructions (populated when the
// HIR builder is aware of outer-scope captures via context_vars) to detect
// reassignment of module-level variables from within component/hook bodies.
// Our HIR builder does not populate context_vars for nested function builders,
// so module-level assignments in nested functions produce StoreLocal (not
// StoreContext or StoreGlobal). We use a name-based approach: collect locally
// declared variables and flag any StoreLocal/Reassign targeting undeclared names.

use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{HIR, HIRFunction, InstructionKind, InstructionValue, Param};
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

/// Check all blocks in an HIR for global/outer-scope reassignments,
/// recursing into nested function expressions.
fn check_blocks(hir: &HIR, component_locals: &FxHashSet<String>, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Explicit StoreGlobal (for well-known globals)
            if let InstructionValue::StoreGlobal { name, .. } = &instr.value {
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
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    check_nested_for_outer_scope_stores(lowered_func, component_locals, errors);
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
fn check_nested_for_outer_scope_stores(
    func: &HIRFunction,
    all_ancestor_locals: &FxHashSet<String>,
    errors: &mut ErrorCollector,
) {
    let nested_locals = collect_locally_declared_func(func);

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
                            format!(
                                "Cannot reassign variables declared outside of the \
                                 component/hook. Variable `{name}` is declared outside \
                                 of the component/hook and cannot be reassigned during \
                                 render."
                            ),
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
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    // Merge current function's locals into ancestor set for deeper recursion
                    let mut merged = all_ancestor_locals.clone();
                    merged.extend(nested_locals.iter().cloned());
                    check_nested_for_outer_scope_stores(lowered_func, &merged, errors);
                }
                // Explicit StoreGlobal in nested context
                InstructionValue::StoreGlobal { name, .. } => {
                    errors.push(CompilerError::invalid_react(
                        instr.loc,
                        format!(
                            "Cannot reassign variables declared outside of the component/hook. \
                             Variable `{name}` is declared outside of the component/hook \
                             and cannot be reassigned during render."
                        ),
                    ));
                }
                _ => {}
            }
        }
    }
}
