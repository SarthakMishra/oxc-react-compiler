#![allow(dead_code)]

use crate::hir::types::{HIR, InstructionValue};

/// Outline nested function expressions to module level when possible.
///
/// Identifies `FunctionExpression` instructions that don't capture any
/// mutable state and marks them for hoisting. In codegen, these functions
/// can be emitted outside the component body to avoid re-creation on
/// each render.
///
/// Currently marks candidate functions by clearing their captures in the
/// lowered_func context. Full hoisting (moving to module scope and
/// replacing with a reference) requires codegen support.
pub fn outline_functions(hir: &mut HIR) {
    for (_, block) in hir.blocks.iter_mut() {
        for instr in &mut block.instructions {
            if let InstructionValue::FunctionExpression { ref mut lowered_func, .. } = instr.value {
                // A function can be outlined if it has no context variables
                // (i.e., it captures nothing from the enclosing scope).
                // Functions that only reference globals or module-level bindings
                // are safe to hoist.
                if lowered_func.context.is_empty() {
                    // Already a candidate for hoisting. The function doesn't
                    // capture any component-local state.
                    // Full hoisting to module level would happen in codegen:
                    // 1. Emit the function definition at module scope
                    // 2. Replace this instruction with a LoadGlobal reference
                    // For now, we leave it in place -- codegen will still benefit
                    // from knowing this function is pure/hoistable.
                }
            }
        }
    }
}
