use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::environment::{EnvironmentConfig, ExhaustiveDepsMode};
use crate::hir::types::{HIR, IdentifierId, InstructionValue, Place};
use rustc_hash::{FxHashMap, FxHashSet};

/// Known hooks that accept a dependency array as their second argument.
const HOOKS_WITH_DEPS: &[&str] = &["useMemo", "useCallback", "useEffect", "useLayoutEffect"];

/// Effect hooks (for distinguishing MemoDependency vs EffectDependency).
const EFFECT_HOOKS: &[&str] = &["useEffect", "useLayoutEffect"];

/// Memo hooks.
const MEMO_HOOKS: &[&str] = &["useMemo", "useCallback"];

/// Validate that dependency arrays for memoization/effect hooks are exhaustive.
///
/// Compares the reactive values actually used inside the callback against the
/// declared dependency array. Missing dependencies can cause stale closures;
/// extra dependencies cause unnecessary re-computations.
///
/// Upstream: ValidateExhaustiveDeps.ts
pub fn validate_exhaustive_dependencies(
    hir: &HIR,
    config: &EnvironmentConfig,
    errors: &mut ErrorCollector,
) {
    // Build id-to-name map for resolving SSA temporaries
    let id_to_name = build_name_map(hir);

    // Build reactive-names set from the parent HIR for filtering effect deps.
    // DIVERGENCE: Upstream checks reactivity on each identifier in the lowered func.
    // Our HIR doesn't propagate reactivity into lowered function bodies, so we
    // collect reactive names from the parent HIR's LoadLocal/LoadContext instructions.
    let reactive_names = build_reactive_names(hir);

    // Determine the effect deps mode
    let effect_mode = if config.validate_exhaustive_effect_dependencies {
        match config.validate_exhaustive_effect_dependencies_mode {
            ExhaustiveDepsMode::Off => ExhaustiveDepsMode::All,
            other => other,
        }
    } else {
        ExhaustiveDepsMode::Off
    };

    let validate_memo = config.validate_exhaustive_memo_dependencies;

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, args } = &instr.value {
                let name = callee
                    .identifier
                    .name
                    .as_deref()
                    .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));

                let Some(name) = name else { continue };

                if !HOOKS_WITH_DEPS.contains(&name) {
                    continue;
                }

                // Need exactly 2 args: callback and deps array
                if args.len() != 2 {
                    continue;
                }

                let is_effect = EFFECT_HOOKS.contains(&name);
                let is_memo = MEMO_HOOKS.contains(&name);

                // Skip if this hook type isn't being validated
                if is_effect && effect_mode == ExhaustiveDepsMode::Off {
                    continue;
                }
                if is_memo && !validate_memo {
                    continue;
                }

                let callback_place = &args[0];
                let deps_place = &args[1];

                // Always collect ALL captured variables from the callback.
                // Reactivity filtering is done at the reporting stage for effects.
                let all_callback_deps = collect_callback_dependencies(hir, callback_place);

                // Find the deps array and collect its elements
                let declared_deps = collect_declared_deps(hir, deps_place, &id_to_name);

                let kind = if is_effect {
                    DiagnosticKind::EffectDependency
                } else {
                    DiagnosticKind::MemoDependency
                };

                // Determine which checks to run based on mode
                let check_missing = if is_effect {
                    matches!(effect_mode, ExhaustiveDepsMode::All | ExhaustiveDepsMode::MissingOnly)
                } else {
                    true // memo always checks missing
                };
                let check_extra = if is_effect {
                    matches!(effect_mode, ExhaustiveDepsMode::All | ExhaustiveDepsMode::ExtraOnly)
                } else {
                    true // memo always checks extra
                };

                // Report missing dependencies
                if check_missing {
                    // Sort for deterministic error ordering
                    let mut sorted_deps: Vec<&String> = all_callback_deps.iter().collect();
                    sorted_deps.sort();

                    for dep_name in sorted_deps {
                        if declared_deps.contains(dep_name.as_str()) {
                            continue;
                        }

                        // For effect hooks: only report reactive deps as missing.
                        // For memo hooks: report ALL deps as missing (including non-reactive).
                        if is_effect && !reactive_names.contains(dep_name.as_str()) {
                            continue;
                        }

                        let category = if is_effect { "effect" } else { "memoization" };
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            format!(
                                "Found missing {category} dependencies. Missing \
                                 dependencies can cause a value to update less often \
                                 than it should. Dependency `{dep_name}` is used but \
                                 not listed in the dependency array."
                            ),
                            kind,
                        ));
                    }
                }

                // Report extra dependencies
                if check_extra {
                    // Sort for deterministic error ordering
                    let mut sorted_declared: Vec<&String> = declared_deps.iter().collect();
                    sorted_declared.sort();

                    for dep_name in sorted_declared {
                        // A declared dep is "extra" if it's not used in the callback at all
                        if !all_callback_deps.contains(dep_name) {
                            let category = if is_effect { "effect" } else { "memoization" };
                            errors.push(CompilerError::invalid_react_with_kind(
                                instr.loc,
                                format!(
                                    "Found extra {category} dependencies. Extra \
                                     dependencies can cause a value to update more often \
                                     than it should. Dependency `{dep_name}` is listed in \
                                     the dependency array but is not used."
                                ),
                                kind,
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Collect ALL variable names used inside a callback function body.
///
/// Collects every LoadLocal/LoadContext reference regardless of reactivity,
/// since reactivity filtering is done at the reporting stage.
fn collect_callback_dependencies(hir: &HIR, callback: &Place) -> FxHashSet<String> {
    let mut deps = FxHashSet::default();
    let callback_id = callback.identifier.id;

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }

            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        match &inner_instr.value {
                            InstructionValue::LoadLocal { place }
                            | InstructionValue::LoadContext { place } => {
                                if let Some(name) = &place.identifier.name {
                                    deps.insert(name.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    deps
}

/// Collect the names of variables declared in a dependency array expression.
///
/// Resolves SSA temporaries through the id_to_name map when the array element
/// has no direct name (e.g., it's a LoadLocal temp).
fn collect_declared_deps(
    hir: &HIR,
    deps_place: &Place,
    id_to_name: &FxHashMap<IdentifierId, String>,
) -> FxHashSet<String> {
    let mut declared = FxHashSet::default();
    let deps_id = deps_place.identifier.id;

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != deps_id {
                continue;
            }

            if let InstructionValue::ArrayExpression { elements } = &instr.value {
                for element in elements {
                    if let crate::hir::types::ArrayElement::Expression(place) = element {
                        // Try direct name first, then SSA resolution
                        if let Some(name) = &place.identifier.name {
                            declared.insert(name.clone());
                        } else if let Some(name) = id_to_name.get(&place.identifier.id) {
                            declared.insert(name.clone());
                        }
                    }
                }
            }
        }
    }

    declared
}

/// Build a map from identifier ID → name for SSA resolution.
fn build_name_map(hir: &HIR) -> FxHashMap<IdentifierId, String> {
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadGlobal { binding } => {
                    id_to_name.insert(instr.lvalue.identifier.id, binding.name.clone());
                }
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name.clone());
                    }
                }
                _ => {}
            }
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_name.entry(instr.lvalue.identifier.id).or_insert_with(|| name.clone());
            }
        }
    }

    id_to_name
}

/// Build a set of variable names that are reactive in the parent HIR.
///
/// DIVERGENCE: Upstream checks `place.reactive` inside lowered function bodies.
/// Our HIR doesn't propagate reactivity into lowered functions, so we collect
/// reactive names from the parent HIR and use those for effect dep filtering.
fn build_reactive_names(hir: &HIR) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.reactive
                        && let Some(name) = &place.identifier.name
                    {
                        names.insert(name.clone());
                    }
                }
                _ => {}
            }
        }
    }
    names
}
