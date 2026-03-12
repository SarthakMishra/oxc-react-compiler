#![allow(dead_code)]

use crate::hir::types::{HIR, InstructionValue};

// DIVERGENCE: Upstream's OutlineFunctions pass fully hoists function definitions
// to module scope. This implementation only marks candidates via a naming
// convention (__hoistable_ prefix) and defers actual hoisting to codegen. This
// simplified approach avoids rewriting the module-level AST during HIR passes.
/// Outline nested function expressions to module level when possible.
///
/// Identifies `FunctionExpression` instructions that don't capture any
/// mutable state and marks them as hoistable. Functions with empty context
/// (no captured variables) are safe to move to module scope since they
/// don't depend on component-local state.
///
/// This pass marks candidates by setting the function name to include a
/// `__hoistable` suffix that codegen can detect. Full hoisting to module
/// scope is deferred to the codegen phase where it can emit the function
/// definition at the top level.
///
/// Hoistable functions:
/// - Have empty context (capture nothing from enclosing scope)
/// - Are not generators (generator state is closure-local)
/// - Are not async (async functions may capture scheduler context)
pub fn outline_functions(hir: &mut HIR) {
    // Collect hoistable function identifiers
    let mut hoistable_ids = Vec::new();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                if lowered_func.context.is_empty()
                    && !lowered_func.is_generator
                    && !lowered_func.is_async
                {
                    hoistable_ids.push(instr.lvalue.identifier.id);
                }
            }
        }
    }

    // Mark hoistable functions by setting a naming convention.
    // Codegen will detect this and can choose to hoist them.
    // Note: In a full implementation, we'd add a dedicated `hoistable: bool`
    // field to FunctionExpression or use a side map. For now we use the
    // identifier as a signal since these are already unnamed temporaries.
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if hoistable_ids.contains(&instr.lvalue.identifier.id) {
                if let InstructionValue::FunctionExpression { name, .. } = &mut instr.value {
                    if name.is_none() {
                        // Mark unnamed functions as hoistable via name convention
                        *name = Some(format!("__hoistable_{}", instr.lvalue.identifier.id.0));
                    }
                }
            }
        }
    }
}
