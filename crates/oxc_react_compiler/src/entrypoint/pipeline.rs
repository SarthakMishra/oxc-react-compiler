use crate::error::ErrorCollector;
use crate::hir::environment::EnvironmentConfig;
use crate::hir::types::{HIR, HIRFunction, IdentifierId, Param, ReactiveFunction};

/// Run the HIR compilation pipeline (passes 2–46).
///
/// This executes all HIR-level passes in the correct order, with config-based gating.
/// After this, the HIR is ready for reactive function construction.
///
/// The bail behavior is controlled by `config.bail_threshold`:
/// - `AllErrors`: bail on any validation error (strictest)
/// - `CriticalErrors`: bail only on invariant violations (default, matches upstream)
/// - `None`: never bail, always attempt compilation
pub fn run_pipeline(
    hir: &mut HIR,
    config: &EnvironmentConfig,
    errors: &mut ErrorCollector,
    param_names: &[String],
    param_ids: &[IdentifierId],
    returns_id: Option<IdentifierId>,
) -> Result<(), ()> {
    let bail_threshold = config.bail_threshold;
    // Phase 0: Reject unsupported patterns (matches upstream BuildHIR Todo errors)
    crate::validation::validate_no_unsupported_nodes::validate_no_unsupported_nodes(hir, errors);
    if errors.should_bail(bail_threshold) {
        return Err(());
    }

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
    // Keep manual memo markers if either the preserve-memo flag or the
    // validate-memo flag is set — the markers are needed for Pass 61.
    if !config.enable_preserve_existing_memoization_guarantees
        && !config.validate_preserve_existing_memoization_guarantees
    {
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

    // Pass 9.6: Inline LoadLocal temps into their consumers (post-SSA).
    // With named lvalues on Store/Declare, LoadLocal creates an emit-temp
    // that's a copy of the named variable. Replace references to the
    // emit-temp with the named variable in subsequent instructions and
    // terminals. Excludes globals/imports to avoid validation errors.
    inline_load_local_temps(hir);

    // Phase 3: Optimization & Type Inference
    // Pass 10: Constant propagation (with binary/unary expression folding)
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
        crate::validation::validate_hooks_usage::validate_hooks_usage(
            hir,
            errors,
            &config.hook_aliases,
        );
    }

    // Pass 14: validate_no_capitalized_calls (conditional)
    if config.validate_no_capitalized_calls {
        crate::validation::validate_no_capitalized_calls::validate_no_capitalized_calls(
            hir, errors,
        );
    }

    // Pass 14.5: validate_no_global_reassignment
    crate::validation::validate_no_global_reassignment::validate_no_global_reassignment(
        hir,
        errors,
        param_names,
    );

    // Pass 14.6: validate_no_eval
    crate::validation::validate_no_eval::validate_no_eval(hir, errors);

    // Bail early if hooks/capitalized/global/eval validation found critical errors
    if errors.should_bail(bail_threshold) {
        return Err(());
    }

    // Phase 5: Mutation/Aliasing Analysis
    // Pass 14: optimize_props_method_calls
    crate::optimization::optimize_props_method_calls::optimize_props_method_calls(hir);

    // Pass 15: analyse_functions
    let mut fn_signatures = crate::inference::analyse_functions::analyse_functions(hir, errors);

    // Pass 15.5: populate_builtin_signatures (Phase 2e)
    // Add known signatures for React hooks, pure global functions, etc.
    // so the abstract interpreter can reason precisely about their effects
    // instead of falling back to conservative defaults.
    crate::inference::analyse_functions::populate_builtin_signatures(hir, &mut fn_signatures);

    // Pass 15.6: populate_method_signatures
    // Build method-level signatures for known global objects (Math, JSON, Object, console, etc.)
    // and array instance methods (push, map, etc.). Enables precise effects for MethodCall
    // instructions instead of conservative fallback.
    let method_signatures = crate::inference::analyse_functions::populate_method_signatures(hir);

    // Pass 16: infer_mutation_aliasing_effects (with param pre-freezing)
    // Threading param_names enables the abstract interpreter to pre-freeze
    // component params in the heap, producing MutateFrozen effects for actual
    // frozen-value mutations instead of relying on name-based tracking.
    crate::inference::infer_mutation_aliasing_effects::infer_mutation_aliasing_effects_with_params(
        hir,
        &fn_signatures,
        &method_signatures,
        param_names,
    );

    // Pass 16.5: validate_no_mutation_after_freeze (uses effects from Pass 16)
    // Must run BEFORE DCE (Pass 18) because DCE may remove standalone JSX
    // expressions whose Freeze effects are needed for detecting mutations.
    // Uses effect-derived mutation ranges to avoid over-freezing hook call arguments.
    crate::validation::validate_no_mutation_after_freeze::validate_no_mutation_after_freeze(
        hir,
        errors,
        param_names,
        param_ids,
    );

    // Bail if frozen-mutation detected (before expensive downstream passes)
    if errors.should_bail(bail_threshold) {
        return Err(());
    }

    // Pass 17: SSR optimization (conditional, runs before DCE to allow removal of client-only code)
    if config.enable_ssr {
        crate::optimization::optimize_for_ssr::optimize_for_ssr(hir);
    }

    // Pass 18: Dead code elimination (extended: also removes unused DeclareLocal).
    // Safe to use the extended version here because all validation passes have
    // already run, so we won't remove declarations that validators depend on.
    crate::optimization::dead_code_elimination::dead_code_elimination_with_unused_declares(hir);

    // Pass 19: prune_maybe_throws (2nd pass)
    crate::optimization::prune_maybe_throws::prune_maybe_throws(hir);

    // Pass 20: infer_mutation_aliasing_ranges
    crate::inference::infer_mutation_aliasing_ranges::infer_mutation_aliasing_ranges(
        hir, returns_id,
    );

    // Pass 20.5: annotate_last_use (stamps identifier.last_use for scope inference)
    crate::inference::infer_mutation_aliasing_ranges::annotate_last_use(hir);

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
            hir,
            errors,
            config.enable_treat_set_identifiers_as_state_setters,
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
    crate::inference::infer_reactive_places::infer_reactive_places(hir, param_names, param_ids);

    // Pass 30: validate_exhaustive_dependencies (conditional)
    // Runs if either memo deps validation is on, or effect deps mode is not Off,
    // or the legacy bool flag is set.
    if config.validate_exhaustive_memo_dependencies
        || config.validate_exhaustive_effect_dependencies
        || config.validate_exhaustive_effect_dependencies_mode
            != crate::hir::environment::ExhaustiveDepsMode::Off
    {
        crate::validation::validate_exhaustive_dependencies::validate_exhaustive_dependencies(
            hir, config, errors,
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
    crate::reactive_scopes::infer_reactive_scope_variables::infer_reactive_scope_variables(
        hir, param_ids,
    );

    // Pass 33.5: propagate_scope_membership_hir
    // Pull unscoped instructions into their consuming scope when ALL consumers
    // are in the same scope. This ensures instructions that produce values used
    // exclusively within one scope become members of that scope.
    crate::reactive_scopes::infer_reactive_scope_variables::propagate_scope_membership_hir(hir);

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

    // Pass 42.5: flatten_scopes_with_hooks_or_use_hir
    // Must run BEFORE build_reactive_scope_terminals_hir so that split scopes
    // get proper Terminal::Scope structures. This pass splits scopes around
    // hook calls instead of removing them entirely.
    crate::reactive_scopes::prune_scopes::flatten_scopes_with_hooks_or_use_hir(hir);

    // Pass 43: build_reactive_scope_terminals_hir
    crate::reactive_scopes::prune_scopes::build_reactive_scope_terminals_hir(hir);

    // Pass 44: flatten_reactive_loops_hir
    crate::reactive_scopes::prune_scopes::flatten_reactive_loops_hir(hir);

    // Pass 46: propagate_scope_dependencies_hir
    crate::reactive_scopes::propagate_dependencies::propagate_scope_dependencies_hir(
        hir,
        param_names,
    );

    // Pass 46.5: derive_minimal_dependencies_hir (dependency tree minimization)
    crate::reactive_scopes::derive_minimal_dependencies::derive_minimal_dependencies_hir(hir);

    // Check if we should bail before building reactive function
    if errors.should_bail(bail_threshold) {
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
    let bail_threshold = config.bail_threshold;

    // Extract function parameter names and IDs for reactivity seeding and Pass 46.
    let param_names: Vec<String> = extract_param_names(&hir_func.params);
    let param_ids: Vec<IdentifierId> = extract_param_ids(&hir_func.params);

    // Run HIR passes (2–46)
    let returns_id = Some(hir_func.returns.place.identifier.id);
    run_pipeline(&mut hir_func.body, config, errors, &param_names, &param_ids, returns_id)?;

    // Pass 47: Build reactive function (CFG → tree IR)
    let mut rf = crate::reactive_scopes::build_reactive_function::build_reactive_function(
        hir_func.body,
        hir_func.params,
        hir_func.id,
        hir_func.loc,
        hir_func.directives,
        hir_func.is_arrow,
        hir_func.is_async,
        hir_func.is_generator,
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
    // Run when either the enable flag (which changes compiler behavior) or the
    // validate-only flag (which just validates without changing behavior) is set.
    if config.enable_preserve_existing_memoization_guarantees
        || config.validate_preserve_existing_memoization_guarantees
    {
        crate::validation::validate_preserved_manual_memoization::validate_preserved_manual_memoization(&rf, errors);
    }

    // Check for errors after RF passes
    if errors.should_bail(bail_threshold) {
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
    // Lint mode doesn't have function params available, pass empty slice.
    // Free variable detection in Pass 46 may be less accurate but lint mode
    // doesn't produce output code, so this doesn't affect correctness.
    run_pipeline(hir, config, errors, &[], &[], None)?;
    Ok(())
}

/// Extract named parameter names from function params.
///
/// These names are locally-defined reactive inputs — they must not be
/// treated as free variables by the dependency propagation pass.
fn extract_param_names(params: &[Param]) -> Vec<String> {
    let mut names = Vec::new();
    for param in params {
        match param {
            Param::Identifier(place) | Param::Spread(place) => {
                if let Some(name) = &place.identifier.name {
                    names.push(name.clone());
                }
            }
        }
    }
    names
}

fn extract_param_ids(params: &[Param]) -> Vec<IdentifierId> {
    params
        .iter()
        .map(|p| match p {
            Param::Identifier(place) | Param::Spread(place) => place.identifier.id,
        })
        .collect()
}

/// Inline LoadLocal emit-temps into their consumers.
///
/// For each `LoadLocal { place: x } → temp` where `x` is a named variable,
/// replace all references to `temp.id` with `x`'s Place in subsequent
/// instructions and terminals. This eliminates the emit-temp indirection
/// so that scope analysis tracks the named variable directly.
///
/// Must run after SSA (the LoadLocal instruction defines a new SSA version
/// of the emit-temp, which would conflict with the named variable's versions
/// if done before SSA).
fn inline_load_local_temps(hir: &mut HIR) {
    use crate::hir::types::{InstructionValue, Place};
    use rustc_hash::FxHashMap;

    // Phase 1: Collect IDs that originate from LoadGlobal (hooks, builtins,
    // imports). Track through StoreLocal chains: if a global result is stored
    // into a named variable, that variable should NOT be inlined.
    let mut global_ids: rustc_hash::FxHashSet<IdentifierId> = rustc_hash::FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if matches!(&instr.value, InstructionValue::LoadGlobal { .. }) {
                global_ids.insert(instr.lvalue.identifier.id);
            }
            // Propagate: if StoreLocal stores a global-origin value, mark the target
            if let InstructionValue::StoreLocal { lvalue, value, .. }
            | InstructionValue::StoreContext { lvalue, value } = &instr.value
                && global_ids.contains(&value.identifier.id)
            {
                global_ids.insert(lvalue.identifier.id);
            }
        }
    }

    // Phase 2: Build substitution map (emit-temp ID → named place).
    // Substitute ALL LoadLocal of named variables, excluding globals/imports.
    // This is the broadest possible inline that maximizes conformance.
    let mut subs: FxHashMap<IdentifierId, Place> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } =
                &instr.value
                && place.identifier.name.is_some()
                && instr.lvalue.identifier.name.is_none()
                && !global_ids.contains(&place.identifier.id)
            {
                subs.insert(instr.lvalue.identifier.id, place.clone());
            }
        }
    }

    if subs.is_empty() {
        return;
    }

    // Phase 2: Apply substitutions to all operand places
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            substitute_operands(&mut instr.value, &subs);
        }
        substitute_terminal(&mut block.terminal, &subs);
    }
}

fn substitute_operands(
    value: &mut crate::hir::types::InstructionValue,
    subs: &rustc_hash::FxHashMap<IdentifierId, crate::hir::types::Place>,
) {
    use crate::hir::types::{ArrayElement, InstructionValue, ObjectPropertyKey};

    let sub = |place: &mut crate::hir::types::Place| {
        if let Some(replacement) = subs.get(&place.identifier.id) {
            *place = replacement.clone();
        }
    };

    match value {
        InstructionValue::StoreLocal { value, .. }
        | InstructionValue::StoreContext { value, .. } => sub(value),
        InstructionValue::CallExpression { callee, args, .. }
        | InstructionValue::NewExpression { callee, args } => {
            sub(callee);
            for arg in args {
                sub(arg);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            sub(receiver);
            for arg in args {
                sub(arg);
            }
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => sub(object),
        InstructionValue::PropertyStore { object, value, .. } => {
            sub(object);
            sub(value);
        }
        InstructionValue::ComputedLoad { object, property, .. }
        | InstructionValue::ComputedDelete { object, property } => {
            sub(object);
            sub(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            sub(object);
            sub(property);
            sub(value);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            sub(left);
            sub(right);
        }
        InstructionValue::UnaryExpression { value, .. }
        | InstructionValue::Await { value }
        | InstructionValue::TypeCastExpression { value, .. }
        | InstructionValue::StoreGlobal { value, .. }
        | InstructionValue::GetIterator { collection: value }
        | InstructionValue::NextPropertyOf { value } => sub(value),
        InstructionValue::IteratorNext { iterator, .. } => sub(iterator),
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => sub(lvalue),
        InstructionValue::Destructure { value, .. } => sub(value),
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                sub(&mut prop.value);
                if let ObjectPropertyKey::Computed(k) = &mut prop.key {
                    sub(k);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for el in elements {
                match el {
                    ArrayElement::Expression(p) | ArrayElement::Spread(p) => sub(p),
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            sub(tag);
            for a in props {
                sub(&mut a.value);
            }
            for c in children {
                sub(c);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for c in children {
                sub(c);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for s in subexpressions {
                sub(s);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value: tpl } => {
            sub(tag);
            for s in &mut tpl.subexpressions {
                sub(s);
            }
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            sub(decl);
            for d in deps {
                sub(d);
            }
        }
        // No operands to substitute
        InstructionValue::LoadLocal { .. }
        | InstructionValue::LoadContext { .. }
        | InstructionValue::DeclareLocal { .. }
        | InstructionValue::DeclareContext { .. }
        | InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

fn substitute_terminal(
    terminal: &mut crate::hir::types::Terminal,
    subs: &rustc_hash::FxHashMap<IdentifierId, crate::hir::types::Place>,
) {
    use crate::hir::types::Terminal;
    let sub = |place: &mut crate::hir::types::Place| {
        if let Some(replacement) = subs.get(&place.identifier.id) {
            *place = replacement.clone();
        }
    };
    match terminal {
        Terminal::Return { value, .. } | Terminal::Throw { value } => sub(value),
        Terminal::If { test, .. } | Terminal::Branch { test, .. } => sub(test),
        _ => {}
    }
}
