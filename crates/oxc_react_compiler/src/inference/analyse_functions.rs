
use crate::error::ErrorCollector;
use crate::hir::types::{HIR, HIRFunction, InstructionValue};

/// Recursively analyze nested functions within the HIR.
///
/// For each FunctionExpression/ObjectMethod instruction, analyze the
/// nested function's effects to determine how it affects captured variables.
/// This information is used by InferMutationAliasingEffects to properly
/// track mutations through closures.
pub fn analyse_functions(hir: &mut HIR, errors: &mut ErrorCollector) {
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            match &mut instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. } => {
                    analyse_nested_function(lowered_func, errors);
                }
                InstructionValue::ObjectMethod { lowered_func } => {
                    analyse_nested_function(lowered_func, errors);
                }
                _ => {}
            }
        }
    }
}

fn analyse_nested_function(func: &mut HIRFunction, errors: &mut ErrorCollector) {
    // Recursively analyze functions within this function
    analyse_functions(&mut func.body, errors);

    // Run inference passes on the nested function's HIR
    crate::inference::infer_types::infer_types(&mut func.body);
    crate::inference::infer_mutation_aliasing_effects::infer_mutation_aliasing_effects(
        &mut func.body,
    );
    crate::inference::infer_mutation_aliasing_ranges::infer_mutation_aliasing_ranges(
        &mut func.body,
    );
}
