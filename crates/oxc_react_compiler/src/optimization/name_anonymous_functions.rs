#![allow(dead_code)]
use crate::hir::types::{InstructionValue, HIR};

/// Give names to anonymous function expressions based on their assignment target.
/// e.g., `const foo = () => {}` names the function "foo".
pub fn name_anonymous_functions(hir: &mut HIR) {
    for (_, block) in hir.blocks.iter_mut() {
        for instr in &mut block.instructions {
            if let InstructionValue::FunctionExpression { name, .. } = &mut instr.value {
                if name.is_none() {
                    if let Some(ref lvalue_name) = instr.lvalue.identifier.name {
                        *name = Some(lvalue_name.clone());
                    }
                }
            }
        }
    }
}
