use rustc_hash::FxHashMap;

use crate::error::ErrorCollector;
use crate::hir::types::{
    AliasingEffect, Effect, FunctionSignature, HIR, HIRFunction, IdentifierId, InstructionValue,
    Param, ParamEffect, ValueKind, ValueReason,
};

/// Recursively analyze nested functions within the HIR.
///
/// For each FunctionExpression/ObjectMethod instruction, analyze the
/// nested function's effects to determine how it affects captured variables.
/// This information is used by InferMutationAliasingEffects to properly
/// track mutations through closures.
///
/// Returns a map from the lvalue IdentifierId of each FunctionExpression
/// to its derived FunctionSignature.
pub fn analyse_functions(
    hir: &mut HIR,
    errors: &mut ErrorCollector,
) -> FxHashMap<IdentifierId, FunctionSignature> {
    let mut signatures: FxHashMap<IdentifierId, FunctionSignature> = FxHashMap::default();

    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            match &mut instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. } => {
                    analyse_nested_function(lowered_func, errors);
                    let sig = extract_signature(lowered_func);
                    signatures.insert(instr.lvalue.identifier.id, sig);
                }
                InstructionValue::ObjectMethod { lowered_func } => {
                    analyse_nested_function(lowered_func, errors);
                    let sig = extract_signature(lowered_func);
                    signatures.insert(instr.lvalue.identifier.id, sig);
                }
                _ => {}
            }
        }
    }

    // Propagate signatures through StoreLocal/LoadLocal alias chains
    // so that callee identifiers at call sites can resolve to the signature.
    propagate_signatures(hir, &mut signatures);

    signatures
}

fn analyse_nested_function(func: &mut HIRFunction, errors: &mut ErrorCollector) {
    // Recursively analyze functions within this function
    let mut nested_sigs = analyse_functions(&mut func.body, errors);

    // Add built-in signatures for globals used within this function expression
    populate_builtin_signatures(&func.body, &mut nested_sigs);

    // Run the sub-pipeline on the nested function's HIR.
    // Upstream: AnalyseFunctions recursively calls this sub-pipeline:
    //   0. InferTypes (OXC-specific: needed to set identifier types for scope inference)
    //   1. InferMutationAliasingEffects
    //   2. DeadCodeElimination
    //   3. InferMutationAliasingRanges
    //   4. RewriteInstructionKinds
    //   5. InferReactiveScopeVariables
    crate::inference::infer_types::infer_types(&mut func.body);

    crate::inference::infer_mutation_aliasing_effects::infer_mutation_aliasing_effects(
        &mut func.body,
        &nested_sigs,
    );

    crate::optimization::dead_code_elimination::dead_code_elimination(&mut func.body);

    crate::inference::infer_mutation_aliasing_ranges::infer_mutation_aliasing_ranges(
        &mut func.body,
    );

    // Annotate last_use for scope inference (feeds effective_range computation)
    crate::inference::infer_mutation_aliasing_ranges::annotate_last_use(&mut func.body);

    crate::inference::rewrite_instruction_kinds::rewrite_instruction_kinds_based_on_reassignment(
        &mut func.body,
    );

    // Extract param IDs for scope inference
    let param_ids: Vec<IdentifierId> = func
        .params
        .iter()
        .map(|p| match p {
            Param::Identifier(place) | Param::Spread(place) => place.identifier.id,
        })
        .collect();

    crate::reactive_scopes::infer_reactive_scope_variables::infer_reactive_scope_variables(
        &mut func.body,
        &param_ids,
    );

    // Compute externally-visible aliasing effects for this function expression.
    // This enables callers to use precise effect-based resolution instead of
    // the conservative fallback.
    func.aliasing_effects = Some(compute_aliasing_effects(func));
}

/// Extract a FunctionSignature from an analyzed HIRFunction.
///
/// After inference passes have run on the nested function, we look at the
/// effect annotations on parameter places to derive per-parameter effects.
/// The return effect is derived from the function's return place.
fn extract_signature(func: &HIRFunction) -> FunctionSignature {
    let params: Vec<ParamEffect> = func
        .params
        .iter()
        .map(|param| {
            let place = match param {
                Param::Identifier(p) | Param::Spread(p) => p,
            };
            ParamEffect {
                effect: place.effect,
                // A parameter aliases to the return value if it has a Store effect,
                // meaning the function stores/returns a reference to this parameter.
                alias_to_return: place.effect == Effect::Store,
            }
        })
        .collect();

    // DIVERGENCE: callee_effect is hardcoded to Read. Upstream derives it from whether the
    // function captures and mutates outer scope variables. This simplification means we won't
    // detect when calling a closure causes side-effects on captured variables.
    FunctionSignature {
        params,
        return_effect: func.returns.effect,
        callee_effect: Effect::Read,
        mutable_only_if_operands_are_mutable: false,
    }
}

/// Compute the externally-visible aliasing effects of a function expression.
///
/// Upstream: "Part 3" of `inferMutationAliasingRanges()` — function effect inference.
///
/// For each parameter and context variable, check if it is mutated within the
/// function body. If so, emit a `Mutate` or `MutateConditionally` effect. This
/// tells the caller's abstract interpreter how calling this function affects
/// values passed as arguments.
///
/// Also emits a `Create` for the return value based on the return place's effect,
/// and `Alias` effects for any parameters that alias to the return value.
fn compute_aliasing_effects(func: &HIRFunction) -> Vec<AliasingEffect> {
    let mut effects = Vec::new();

    // Create the return value with appropriate kind
    let return_kind = match func.returns.effect {
        Effect::Freeze => ValueKind::Frozen,
        _ => ValueKind::Mutable,
    };
    effects.push(AliasingEffect::Create {
        into: func.returns.clone(),
        value: return_kind,
        reason: ValueReason::Other,
    });

    // Check each parameter for mutation
    for param in &func.params {
        let place = match param {
            Param::Identifier(p) | Param::Spread(p) => p,
        };

        match place.effect {
            Effect::Store | Effect::Mutate => {
                // Parameter is definitely mutated
                effects.push(AliasingEffect::Mutate { value: place.clone(), reason: None });
            }
            Effect::ConditionallyMutate | Effect::ConditionallyMutateIterator => {
                // Parameter is conditionally mutated
                effects.push(AliasingEffect::MutateConditionally { value: place.clone() });
            }
            Effect::Capture => {
                // Parameter is captured (may alias to return)
                effects.push(AliasingEffect::Capture {
                    from: place.clone(),
                    into: func.returns.clone(),
                });
            }
            Effect::Freeze => {
                effects.push(AliasingEffect::Freeze {
                    value: place.clone(),
                    reason: ValueReason::Other,
                });
            }
            _ => {
                // Read or Unknown — no externally-visible effect
            }
        }
    }

    // Check each context variable (captured outer-scope vars) for mutation
    for ctx_place in &func.context {
        let is_mutated = is_context_var_mutated(&func.body, ctx_place.identifier.id);
        if is_mutated {
            effects.push(AliasingEffect::MutateConditionally { value: ctx_place.clone() });
        }
    }

    effects
}

/// Check if a context variable is mutated within a function body.
///
/// A context variable is mutated if any StoreContext instruction writes to it.
fn is_context_var_mutated(hir: &HIR, ctx_id: IdentifierId) -> bool {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreContext { lvalue, .. } = &instr.value
                && lvalue.identifier.id == ctx_id
            {
                return true;
            }
            // Also check if the context var has a mutated mutable range
            // by seeing if any effect directly mutates it
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    match effect {
                        AliasingEffect::Mutate { value, .. }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value } => {
                            if value.identifier.id == ctx_id {
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    false
}

/// Propagate function signatures through StoreLocal/LoadLocal alias chains.
///
/// When a function is defined and then stored to a variable:
///   $0 = FunctionExpression ...  (signature known for $0)
///   StoreLocal x = $0            (x should also map to the signature)
///   $1 = LoadLocal x             ($1 should also map to the signature)
///   $2 = CallExpression $1(...)  (need signature for $1's id)
///
/// We use a fixpoint loop to handle multi-hop chains across blocks.
fn propagate_signatures(hir: &HIR, signatures: &mut FxHashMap<IdentifierId, FunctionSignature>) {
    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 10;

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        for (_, block) in &hir.blocks {
            for instr in &block.instructions {
                match &instr.value {
                    InstructionValue::StoreLocal { lvalue, value, .. }
                    | InstructionValue::StoreContext { lvalue, value } => {
                        if !signatures.contains_key(&lvalue.identifier.id)
                            && let Some(sig) = signatures.get(&value.identifier.id).cloned()
                        {
                            signatures.insert(lvalue.identifier.id, sig);
                            changed = true;
                        }
                    }
                    InstructionValue::LoadLocal { place }
                    | InstructionValue::LoadContext { place } => {
                        if !signatures.contains_key(&instr.lvalue.identifier.id)
                            && let Some(sig) = signatures.get(&place.identifier.id).cloned()
                        {
                            signatures.insert(instr.lvalue.identifier.id, sig);
                            changed = true;
                        }
                    }
                    InstructionValue::TypeCastExpression { value, .. } => {
                        if !signatures.contains_key(&instr.lvalue.identifier.id)
                            && let Some(sig) = signatures.get(&value.identifier.id).cloned()
                        {
                            signatures.insert(instr.lvalue.identifier.id, sig);
                            changed = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in function signatures (Phase 2e)
// ---------------------------------------------------------------------------

/// Populate the function signatures map with built-in signatures for known
/// global functions.
///
/// Scans the HIR for `LoadGlobal` instructions and, for each recognized
/// global name, inserts a `FunctionSignature` describing how the function
/// affects its arguments and return value. This enables the abstract
/// interpreter to reason precisely about calls to known functions instead
/// of falling back to the conservative "assume everything is conditionally
/// mutated" behavior.
///
/// Must be called after `analyse_functions` and before
/// `infer_mutation_aliasing_effects`.
#[expect(clippy::implicit_hasher)]
pub fn populate_builtin_signatures(
    hir: &HIR,
    signatures: &mut FxHashMap<IdentifierId, FunctionSignature>,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::LoadGlobal { binding } = &instr.value {
                let name = &binding.name;
                let id = instr.lvalue.identifier.id;
                // Don't overwrite signatures from local function analysis
                if signatures.contains_key(&id) {
                    continue;
                }
                if let Some(sig) = get_builtin_signature(name) {
                    signatures.insert(id, sig);
                }
            }
        }
    }

    // Propagate built-in signatures through alias chains
    propagate_signatures(hir, signatures);
}

/// Helper to create a read-only param effect.
fn read_param() -> ParamEffect {
    ParamEffect { effect: Effect::Read, alias_to_return: false }
}

/// Helper to create a freeze param effect.
fn freeze_param() -> ParamEffect {
    ParamEffect { effect: Effect::Freeze, alias_to_return: false }
}

/// Helper to create a capture param effect (value captured, may alias return).
#[expect(dead_code)]
fn capture_param() -> ParamEffect {
    ParamEffect { effect: Effect::Capture, alias_to_return: true }
}

/// Helper to create a mutate param effect.
#[expect(dead_code)]
fn mutate_param() -> ParamEffect {
    ParamEffect { effect: Effect::Mutate, alias_to_return: false }
}

/// Helper to create a conditionally-mutate param effect.
#[expect(dead_code)]
fn conditional_mutate_param() -> ParamEffect {
    ParamEffect { effect: Effect::ConditionallyMutate, alias_to_return: false }
}

/// Return a `FunctionSignature` for a known global function name, or `None`
/// if the name is not recognized.
///
/// This covers:
/// - React hooks (useState, useRef, useEffect, useMemo, useCallback, etc.)
/// - Pure global functions (parseInt, parseFloat, isNaN, etc.)
/// - String/Number/Boolean constructors called as functions
fn get_builtin_signature(name: &str) -> Option<FunctionSignature> {
    match name {
        // ---------------------------------------------------------------
        // React hooks
        // ---------------------------------------------------------------

        // useState(initialValue): returns frozen [state, setState]
        // The initial value is read (not mutated or captured).
        "useState" => Some(FunctionSignature {
            params: vec![read_param()],
            return_effect: Effect::Freeze,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useReducer(reducer, initialState, init?): returns frozen [state, dispatch]
        "useReducer" => Some(FunctionSignature {
            params: vec![read_param(), read_param(), read_param()],
            return_effect: Effect::Freeze,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useRef(initialValue): returns a mutable ref object { current }
        "useRef" => Some(FunctionSignature {
            params: vec![read_param()],
            return_effect: Effect::Read,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useEffect(callback, deps?): captures the callback, reads deps
        // Returns void (no meaningful return).
        "useEffect" | "useLayoutEffect" | "useInsertionEffect" => Some(FunctionSignature {
            params: vec![freeze_param(), read_param()],
            return_effect: Effect::Read,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useMemo(factory, deps): captures factory, reads deps, returns frozen
        "useMemo" => Some(FunctionSignature {
            params: vec![freeze_param(), read_param()],
            return_effect: Effect::Freeze,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useCallback(callback, deps): captures callback, reads deps, returns frozen
        "useCallback" => Some(FunctionSignature {
            params: vec![freeze_param(), read_param()],
            return_effect: Effect::Freeze,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useContext(context): reads the context, returns frozen
        "useContext" => Some(FunctionSignature {
            params: vec![read_param()],
            return_effect: Effect::Freeze,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useTransition(): returns frozen [isPending, startTransition]
        "useTransition" => Some(FunctionSignature {
            params: vec![],
            return_effect: Effect::Freeze,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useDeferredValue(value): reads value, returns frozen
        "useDeferredValue" => Some(FunctionSignature {
            params: vec![read_param()],
            return_effect: Effect::Freeze,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useId(): returns frozen string
        "useId" => Some(FunctionSignature {
            params: vec![],
            return_effect: Effect::Freeze,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot?): returns frozen
        "useSyncExternalStore" => Some(FunctionSignature {
            params: vec![read_param(), read_param(), read_param()],
            return_effect: Effect::Freeze,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useImperativeHandle(ref, create, deps?): reads all args
        "useImperativeHandle" => Some(FunctionSignature {
            params: vec![read_param(), freeze_param(), read_param()],
            return_effect: Effect::Read,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // useDebugValue(value, formatter?): reads args, no return
        "useDebugValue" => Some(FunctionSignature {
            params: vec![read_param(), read_param()],
            return_effect: Effect::Read,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // ---------------------------------------------------------------
        // Pure global functions (return primitive, read all args)
        // ---------------------------------------------------------------
        "parseInt" | "parseFloat" | "isNaN" | "isFinite" | "encodeURI" | "decodeURI"
        | "encodeURIComponent" | "decodeURIComponent" | "atob" | "btoa" => {
            Some(FunctionSignature {
                params: vec![read_param()],
                return_effect: Effect::Read,
                callee_effect: Effect::Read,
                mutable_only_if_operands_are_mutable: false,
            })
        }

        // String/Number/Boolean called as functions (type coercion)
        "String" | "Number" | "Boolean" => Some(FunctionSignature {
            params: vec![read_param()],
            return_effect: Effect::Read,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // Array.isArray, Object.is — but these are usually called as methods, not globals.
        // structuredClone: creates a new mutable value from the input
        "structuredClone" => Some(FunctionSignature {
            params: vec![read_param()],
            return_effect: Effect::Read,
            callee_effect: Effect::Read,
            mutable_only_if_operands_are_mutable: false,
        }),

        // Unknown hooks: do NOT provide a signature. The conservative fallback
        // in the abstract interpreter (MutateTransitiveConditionally on all args) is
        // more correct for unknown hooks, because they might mutate or capture their
        // arguments in ways we can't predict. Only well-known React hooks above get
        // explicit signatures.
        _ => None,
    }
}
