#![allow(dead_code)]
use crate::hir::types::{HIR, InstructionValue};

/// Give names to anonymous function expressions based on their assignment target.
/// e.g., `const foo = () => {}` names the function "foo".
pub fn name_anonymous_functions(hir: &mut HIR) {
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let InstructionValue::FunctionExpression { name, .. } = &mut instr.value
                && name.is_none()
                    && let Some(ref lvalue_name) = instr.lvalue.identifier.name {
                        *name = Some(lvalue_name.clone());
                    }
        }
    }
}
