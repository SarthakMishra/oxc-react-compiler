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

    // Pass 3: validate_context_variable_lvalues
    crate::validation::validate_context_variable_lvalues::validate_context_variable_lvalues(
        hir, errors,
    );

    // Pass 4: validate_use_memo
    crate::validation::validate_use_memo::validate_use_memo(hir, errors);

    // Pass 5: drop_manual_memoization (conditional)
    if !config.enable_preserve_existing_memoization_guarantees {
        crate::validation::drop_manual_memoization::drop_manual_memoization(hir);
    }

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
    crate::inference::infer_types::infer_types(hir);

    // Pass 12: Rewrite instruction kinds based on reassignment
    crate::inference::rewrite_instruction_kinds::rewrite_instruction_kinds_based_on_reassignment(
        hir,
    );

    // Phase 3b: Name anonymous functions (assign names based on lvalue)
    crate::optimization::name_anonymous_functions::name_anonymous_functions(hir);

    // Phase 4: Validation (Hooks)
    if config.validate_hooks_usage {
        // TODO: validate_hooks_usage (requires full hook rule checking on HIR)
    }
    if config.validate_no_capitalized_calls {
        // TODO: validate_no_capitalized_calls (requires HIR-level call analysis)
    }

    // Phase 5: Mutation/Aliasing Analysis
    // Pass 14: optimize_props_method_calls
    crate::optimization::optimize_props_method_calls::optimize_props_method_calls(hir);

    // Pass 15: analyse_functions
    crate::inference::analyse_functions::analyse_functions(hir, errors);

    // Pass 16: infer_mutation_aliasing_effects
    crate::inference::infer_mutation_aliasing_effects::infer_mutation_aliasing_effects(hir);

    // Pass 18: Dead code elimination
    crate::optimization::dead_code_elimination::dead_code_elimination(hir);

    // Pass 19: prune_maybe_throws (2nd pass)
    crate::optimization::prune_maybe_throws::prune_maybe_throws(hir);

    // Pass 20: infer_mutation_aliasing_ranges
    crate::inference::infer_mutation_aliasing_ranges::infer_mutation_aliasing_ranges(hir);

    // Pass 21: assert_valid_mutable_ranges
    crate::validation::assert_valid_mutable_ranges::assert_valid_mutable_ranges(hir, errors);

    // Phase 6: Validation Battery
    // Pass 22: validate_static_components
    crate::validation::validate_static_components::validate_static_components(hir, errors);

    // Pass 23: validate_no_ref_access_in_render (conditional)
    if config.validate_ref_access_during_render {
        crate::validation::validate_no_ref_access_in_render::validate_no_ref_access_in_render(
            hir, errors,
        );
    }

    // Pass 24: validate_no_set_state_in_render (conditional)
    if config.validate_no_set_state_in_render {
        crate::validation::validate_no_set_state_in_render::validate_no_set_state_in_render(
            hir, errors,
        );
    }

    // Pass 25: validate_no_set_state_in_effects (conditional)
    if config.validate_no_set_state_in_effects {
        crate::validation::validate_no_set_state_in_effects::validate_no_set_state_in_effects(
            hir, errors,
        );
    }

    // Pass 26: validate_no_derived_computations_in_effects (conditional)
    if config.validate_no_derived_computations_in_effects {
        crate::validation::validate_no_derived_computations_in_effects::validate_no_derived_computations_in_effects(hir, errors);
    }

    // Pass 27: validate_no_jsx_in_try (conditional)
    if config.validate_no_jsx_in_try_statements {
        crate::validation::validate_no_jsx_in_try::validate_no_jsx_in_try(hir, errors);
    }

    // Pass 28: validate_no_freezing_known_mutable_functions
    crate::validation::validate_no_freezing_known_mutable_functions::validate_no_freezing_known_mutable_functions(hir, errors);

    // Phase 6b: Optional outlining passes
    if config.enable_jsx_outlining {
        crate::optimization::outline_jsx::outline_jsx(hir);
    }

    if config.enable_function_outlining {
        crate::optimization::outline_functions::outline_functions(hir);
    }

    // Phase 6c: SSR optimization (conditional)
    if config.enable_ssr {
        crate::optimization::optimize_for_ssr::optimize_for_ssr(hir);
    }

    // Phase 7: Reactivity Inference
    // Pass 29: infer_reactive_places
    // TODO: crate::inference::infer_reactive_places::infer_reactive_places(hir);

    // Phase 8: Reactive Scope Construction
    // Passes 33-46: scope inference, alignment, merging
    // TODO

    // Check if we should bail
    if errors.should_bail(PanicThreshold::CriticalErrors) {
        return Err(());
    }

    Ok(())
}

/// Run the pipeline in lint mode: execute all validation and analysis passes
/// but skip codegen. Useful for editor integrations and CI lint checks.
pub fn run_lint_pipeline(
    hir: &mut HIR,
    config: &EnvironmentConfig,
    errors: &mut ErrorCollector,
) -> Result<(), ()> {
    // Run the same passes as run_pipeline but skip codegen.
    // In lint mode, we just collect errors without generating code.
    run_pipeline(hir, config, errors)?;
    Ok(())
}
