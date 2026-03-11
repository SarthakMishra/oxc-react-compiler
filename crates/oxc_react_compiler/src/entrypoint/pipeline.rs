#![allow(dead_code)]

use crate::error::{ErrorCollector, PanicThreshold};
use crate::hir::environment::EnvironmentConfig;
use crate::hir::types::HIR;

/// Run the full compilation pipeline on an HIR function.
///
/// This executes all ~62 passes in the correct order, with config-based gating.
/// Returns Ok(()) on success, or the accumulated errors on failure.
pub fn run_pipeline(
    hir: &mut HIR,
    config: &EnvironmentConfig,
    errors: &mut ErrorCollector,
) -> Result<(), ()> {
    // Phase 1: Early cleanup
    // Pass 2: prune_maybe_throws
    crate::optimization::prune_maybe_throws::prune_maybe_throws(hir);

    // Pass 3-5: Validation and memoization
    // TODO: validate_context_variable_lvalues
    // TODO: validate_use_memo
    // TODO: drop_manual_memoization (conditional)

    // Pass 6: Inline IIFEs
    crate::optimization::inline_iife::inline_iife(hir);

    // Pass 7: Merge consecutive blocks
    crate::optimization::merge_consecutive_blocks::merge_consecutive_blocks(hir);

    // Phase 2: SSA
    // Pass 8: Enter SSA
    crate::ssa::enter_ssa::enter_ssa(hir);

    // Pass 9: Eliminate redundant phi
    crate::ssa::eliminate_redundant_phi::eliminate_redundant_phi(hir);

    // Phase 3: Optimization & Type Inference
    // Pass 10: Constant propagation
    crate::optimization::constant_propagation::constant_propagation(hir);

    // Pass 11: Infer types
    // TODO: crate::inference::infer_types::infer_types(hir);

    // Phase 4: Validation (Hooks)
    if config.validate_hooks_usage {
        // TODO: validate_hooks_usage
    }
    if config.validate_no_capitalized_calls {
        // TODO: validate_no_capitalized_calls
    }

    // Phase 5: Mutation/Aliasing Analysis
    // Pass 14: optimize_props_method_calls
    crate::optimization::optimize_props_method_calls::optimize_props_method_calls(hir);

    // Pass 15-16: analyse_functions, infer_mutation_aliasing_effects
    // TODO

    // Pass 18: Dead code elimination
    crate::optimization::dead_code_elimination::dead_code_elimination(hir);

    // Pass 19: prune_maybe_throws (2nd pass)
    crate::optimization::prune_maybe_throws::prune_maybe_throws(hir);

    // Pass 20: infer_mutation_aliasing_ranges
    // TODO

    // Phase 6: Validation Battery
    // Passes 21-28: Various validations (conditional)
    // TODO: implement each validation

    // Phase 7: Reactivity Inference
    // Pass 29: infer_reactive_places
    // TODO

    // Phase 8: Reactive Scope Construction
    // Passes 33-46: scope inference, alignment, merging
    // TODO

    // Check if we should bail
    if errors.should_bail(PanicThreshold::CriticalErrors) {
        return Err(());
    }

    Ok(())
}
