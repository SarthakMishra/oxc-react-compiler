#![allow(dead_code)]

use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{HIR, InstructionKind, InstructionValue};

/// Validate that context variables (captured from outer scope) are not reassigned
/// when they were originally declared as `const`.
///
/// Context variables represent values captured from an enclosing scope. If the
/// original binding was `const`, the compiler must reject any `StoreContext`
/// that attempts to write to it.
pub fn validate_context_variable_lvalues(hir: &HIR, errors: &mut ErrorCollector) {
    // First pass: collect context variables that are declared as const.
    // A DeclareContext followed by a StoreLocal with InstructionKind::Const
    // indicates a const-bound context variable.
    let const_context_ids: Vec<_> = hir
        .blocks
        .iter()
        .flat_map(|(_, block)| block.instructions.iter())
        .filter_map(|instr| match &instr.value {
            InstructionValue::StoreLocal {
                lvalue, type_: Some(InstructionKind::Const), ..
            } => lvalue.identifier.name.clone(),
            InstructionValue::DeclareLocal { lvalue, type_: InstructionKind::Const } => {
                lvalue.identifier.name.clone()
            }
            _ => None,
        })
        .collect();

    // Second pass: find StoreContext instructions that write to const-bound names.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreContext { lvalue, .. } = &instr.value {
                if let Some(name) = &lvalue.identifier.name {
                    if const_context_ids.contains(name) {
                        errors.push(CompilerError::invalid_js(
                            instr.loc,
                            format!(
                                "Cannot reassign context variable \"{}\". \
                                 It was declared as a const binding and is immutable.",
                                name
                            ),
                        ));
                    }
                }
            }
        }
    }
}
