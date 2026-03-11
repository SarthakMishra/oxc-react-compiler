#![allow(dead_code)]

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue};
use rustc_hash::FxHashSet;

/// Validate that local variables assigned during render are not reassigned
/// in event handlers or effects.
///
/// Variables initialized during render carry reactive semantics. If they are
/// later reassigned inside a callback passed to `useEffect` or an event handler,
/// the compiler cannot safely memoize them because the mutation happens outside
/// the render phase.
pub fn validate_locals_not_reassigned_after_render(hir: &HIR, errors: &mut ErrorCollector) {
    // Step 1: Collect variables assigned during render (top-level block instructions).
    let render_assigned = collect_render_assigned(hir);

    // Step 2: Find function expressions that are passed to effect hooks or event handlers.
    // Then check if those functions reassign any render-phase variables.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                // Check all instructions inside the nested function body
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
fn check_nested_reassignments(
    nested_hir: &HIR,
    render_assigned: &FxHashSet<String>,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &nested_hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreLocal { lvalue, .. } = &instr.value {
                if let Some(name) = &lvalue.identifier.name {
                    if render_assigned.contains(name) {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            format!(
                                "Local variable \"{}\" is assigned during render but \
                                 reassigned inside a nested function (effect or event handler). \
                                 This prevents the compiler from memoizing correctly.",
                                name
                            ),
                            DiagnosticKind::LocalsReassignedAfterRender,
                        ));
                    }
                }
            }
        }
    }
}
