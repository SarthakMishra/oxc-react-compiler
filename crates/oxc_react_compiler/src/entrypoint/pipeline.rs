use crate::error::{ErrorCollector, PanicThreshold};
use crate::hir::environment::EnvironmentConfig;
use crate::hir::types::{HIR, HIRFunction, ReactiveFunction};

/// Default bail-out threshold used throughout the pipeline.
///
/// `AllErrors` causes the pipeline to bail on any validation error
/// (InvalidReact, InvalidJS, Todo, InvariantViolation), matching
/// Babel's behavior of skipping functions that have validation issues.
const PIPELINE_BAIL_THRESHOLD: PanicThreshold = PanicThreshold::AllErrors;

/// Run the HIR compilation pipeline (passes 2–46).
///
/// This executes all HIR-level passes in the correct order, with config-based gating.
/// After this, the HIR is ready for reactive function construction.
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

    // Pass 9.5: Prune temporary lvalues (post-SSA cleanup)
    crate::optimization::prune_temporary_lvalues::prune_temporary_lvalues(hir);

    // Phase 3: Optimization & Type Inference
    // Pass 10: Constant propagation
    crate::optimization::constant_propagation::constant_propagation(hir);

    // Pass 11: Infer types
    crate::inference::infer_types::infer_types(hir);

    // Pass 12: rewrite_instruction_kinds_based_on_reassignment (1st pass, pre-hooks)
    crate::inference::rewrite_instruction_kinds::rewrite_instruction_kinds_based_on_reassignment(
        hir,
    );

    // Phase 4: Validation (Hooks)
    // Pass 13: validate_hooks_usage (conditional)
    if config.validate_hooks_usage {
        crate::validation::validate_hooks_usage::validate_hooks_usage(hir, errors);
    }

    // Pass 14: validate_no_capitalized_calls (conditional)
    if config.validate_no_capitalized_calls {
        crate::validation::validate_no_capitalized_calls::validate_no_capitalized_calls(
            hir, errors,
        );
    }

    // Bail early if hooks validation found critical errors
    if errors.should_bail(PIPELINE_BAIL_THRESHOLD) {
        return Err(());
    }

    // Phase 5: Mutation/Aliasing Analysis
    // Pass 14: optimize_props_method_calls
    crate::optimization::optimize_props_method_calls::optimize_props_method_calls(hir);

    // Pass 15: analyse_functions
    crate::inference::analyse_functions::analyse_functions(hir, errors);

    // Pass 16: infer_mutation_aliasing_effects
    crate::inference::infer_mutation_aliasing_effects::infer_mutation_aliasing_effects(hir);

    // Pass 17: SSR optimization (conditional, runs before DCE to allow removal of client-only code)
    if config.enable_ssr {
        crate::optimization::optimize_for_ssr::optimize_for_ssr(hir);
    }

    // Pass 18: Dead code elimination
    crate::optimization::dead_code_elimination::dead_code_elimination(hir);

    // Pass 19: prune_maybe_throws (2nd pass)
    crate::optimization::prune_maybe_throws::prune_maybe_throws(hir);

    // Pass 20: infer_mutation_aliasing_ranges
    crate::inference::infer_mutation_aliasing_ranges::infer_mutation_aliasing_ranges(hir);

    // Pass 21: validate_locals_not_reassigned_after_render
    crate::validation::validate_locals_not_reassigned_after_render::validate_locals_not_reassigned_after_render(hir, errors);

    // Pass 22: assert_valid_mutable_ranges (config-gated, default off)
    if config.assert_valid_mutable_ranges {
        crate::validation::assert_valid_mutable_ranges::assert_valid_mutable_ranges(hir, errors);
    }

    // Phase 6: Validation Battery
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

    // Pass 28.5: validate_no_impure_functions_in_render (conditional)
    if config.validate_no_impure_functions_in_render {
        crate::validation::validate_no_impure_functions_in_render::validate_no_impure_functions_in_render(hir, errors);
    }

    // Pass 28.6: validate_blocklisted_imports (conditional)
    if !config.blocklisted_imports.is_empty() {
        crate::validation::validate_blocklisted_imports::validate_blocklisted_imports(
            hir,
            &config.blocklisted_imports,
            errors,
        );
    }

    // Pass 28.7: assert_well_formed_break_targets
    crate::validation::assert_well_formed_break_targets::assert_well_formed_break_targets(
        hir, errors,
    );

    // Phase 7: Reactivity Inference
    // Pass 29: infer_reactive_places
    crate::inference::infer_reactive_places::infer_reactive_places(hir);

    // Pass 30: validate_exhaustive_dependencies (conditional)
    // Runs if either memo deps validation is on, or effect deps mode is not Off,
    // or the legacy bool flag is set.
    if config.validate_exhaustive_memo_dependencies
        || config.validate_exhaustive_effect_dependencies
        || config.validate_exhaustive_effect_dependencies_mode
            != crate::hir::environment::ExhaustiveDepsMode::Off
    {
        crate::validation::validate_exhaustive_dependencies::validate_exhaustive_dependencies(
            hir, errors,
        );
    }

    // Pass 31: rewrite_instruction_kinds_based_on_reassignment (2nd pass, post-reactivity)
    crate::inference::rewrite_instruction_kinds::rewrite_instruction_kinds_based_on_reassignment(
        hir,
    );

    // Pass 31.5: compute_unconditional_blocks (feeds CollectHoistablePropertyLoads)
    let unconditional = crate::hir::compute_unconditional_blocks::compute_unconditional_blocks(hir);

    // Pass 31.6: collect_hoistable_property_loads (non-null guarantees for dependency precision)
    let _hoistable =
        crate::inference::collect_hoistable_property_loads::collect_hoistable_property_loads(
            hir,
            &unconditional,
        );

    // Pass 31.7: collect_optional_chain_dependencies (safe dependency paths for ?. chains)
    let _optional_chains =
        crate::inference::collect_optional_chain_dependencies::collect_optional_chain_dependencies(
            hir,
        );

    // Pass 32: validate_static_components
    crate::validation::validate_static_components::validate_static_components(hir, errors);

    // Phase 8: Reactive Scope Construction
    // Pass 33: infer_reactive_scope_variables
    crate::reactive_scopes::infer_reactive_scope_variables::infer_reactive_scope_variables(hir);

    // Pass 34: memoize_fbt_and_macro_operands_in_same_scope
    crate::reactive_scopes::prune_scopes::memoize_fbt_and_macro_operands_in_same_scope(hir);

    // Pass 35: outline_jsx (conditional)
    if config.enable_jsx_outlining {
        crate::optimization::outline_jsx::outline_jsx(hir);
    }

    // Pass 36: name_anonymous_functions (config-gated, default on)
    if config.enable_name_anonymous_functions {
        crate::optimization::name_anonymous_functions::name_anonymous_functions(hir);
    }

    // Pass 37: outline_functions (conditional)
    if config.enable_function_outlining {
        crate::optimization::outline_functions::outline_functions(hir);
    }

    // Pass 38: align_method_call_scopes
    crate::reactive_scopes::align_scopes::align_method_call_scopes(hir);

    // Pass 39: align_object_method_scopes
    crate::reactive_scopes::align_scopes::align_object_method_scopes(hir);

    // Pass 40: prune_unused_labels_hir
    crate::reactive_scopes::align_scopes::prune_unused_labels_hir(hir);

    // Pass 41: align_reactive_scopes_to_block_scopes_hir
    crate::reactive_scopes::align_scopes::align_reactive_scopes_to_block_scopes_hir(hir);

    // Pass 42: merge_overlapping_reactive_scopes_hir
    crate::reactive_scopes::merge_scopes::merge_overlapping_reactive_scopes_hir(hir);

    // Pass 43: build_reactive_scope_terminals_hir
    crate::reactive_scopes::prune_scopes::build_reactive_scope_terminals_hir(hir);

    // Pass 44: flatten_reactive_loops_hir
    crate::reactive_scopes::prune_scopes::flatten_reactive_loops_hir(hir);

    // Pass 45: flatten_scopes_with_hooks_or_use_hir
    crate::reactive_scopes::prune_scopes::flatten_scopes_with_hooks_or_use_hir(hir);

    // Pass 46: propagate_scope_dependencies_hir
    crate::reactive_scopes::propagate_dependencies::propagate_scope_dependencies_hir(hir);

    // Pass 46.5: derive_minimal_dependencies_hir (dependency tree minimization)
    crate::reactive_scopes::derive_minimal_dependencies::derive_minimal_dependencies_hir(hir);

    // Check if we should bail before building reactive function
    if errors.should_bail(PIPELINE_BAIL_THRESHOLD) {
        return Err(());
    }

    Ok(())
}

/// Run the full compilation pipeline: HIR passes → build reactive function → RF optimization.
///
/// Takes ownership of the `HIRFunction` produced by BuildHIR, runs all HIR passes,
/// constructs a `ReactiveFunction`, then runs RF optimization passes.
pub fn run_full_pipeline(
    mut hir_func: HIRFunction,
    config: &EnvironmentConfig,
    errors: &mut ErrorCollector,
) -> Result<ReactiveFunction, ()> {
    // Run HIR passes (2–46)
    run_pipeline(&mut hir_func.body, config, errors)?;

    // Pass 47: Build reactive function (CFG → tree IR)
    let mut rf = crate::reactive_scopes::build_reactive_function::build_reactive_function(
        hir_func.body,
        hir_func.params,
        hir_func.id,
        hir_func.loc,
        hir_func.directives,
    );

    // Phase 9: RF Optimization Passes (48–60)
    crate::reactive_scopes::prune_scopes::prune_unused_labels(&mut rf);
    crate::reactive_scopes::prune_scopes::prune_non_escaping_scopes(&mut rf);
    crate::reactive_scopes::prune_scopes::prune_non_reactive_dependencies(&mut rf);
    crate::reactive_scopes::prune_scopes::prune_unused_scopes(&mut rf);
    crate::reactive_scopes::merge_scopes::merge_reactive_scopes_that_invalidate_together(&mut rf);
    crate::reactive_scopes::prune_scopes::prune_always_invalidating_scopes(&mut rf);
    crate::reactive_scopes::prune_scopes::propagate_early_returns(&mut rf);
    crate::reactive_scopes::prune_scopes::inline_load_locals(&mut rf);
    crate::reactive_scopes::prune_scopes::prune_unused_lvalues(&mut rf);
    crate::reactive_scopes::prune_scopes::promote_used_temporaries(&mut rf);
    crate::reactive_scopes::prune_scopes::extract_scope_declarations_from_destructuring(&mut rf);
    crate::reactive_scopes::prune_scopes::stabilize_block_ids(&mut rf);
    crate::reactive_scopes::prune_scopes::rename_variables(&mut rf);

    // Pass 60: prune_hoisted_contexts
    crate::reactive_scopes::prune_scopes::prune_hoisted_contexts(&mut rf);

    // Pass 61: validate_preserved_manual_memoization (conditional)
    if config.enable_preserve_existing_memoization_guarantees {
        crate::validation::validate_preserved_manual_memoization::validate_preserved_manual_memoization(&rf, errors);
    }

    // Check for errors after RF passes
    if errors.should_bail(PIPELINE_BAIL_THRESHOLD) {
        return Err(());
    }

    Ok(rf)
}

/// Run the pipeline in lint mode: execute all validation and analysis passes
/// but skip codegen. Useful for editor integrations and CI lint checks.
pub fn run_lint_pipeline(
    hir: &mut HIR,
    config: &EnvironmentConfig,
    errors: &mut ErrorCollector,
) -> Result<(), ()> {
    run_pipeline(hir, config, errors)?;
    Ok(())
}
