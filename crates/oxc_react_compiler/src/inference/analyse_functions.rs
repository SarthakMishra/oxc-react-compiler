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
    let nested_sigs = analyse_functions(&mut func.body, errors);

    // Run inference passes on the nested function's HIR
    crate::inference::infer_types::infer_types(&mut func.body);
    crate::inference::infer_mutation_aliasing_effects::infer_mutation_aliasing_effects(
        &mut func.body,
        &nested_sigs,
    );
    crate::inference::infer_mutation_aliasing_ranges::infer_mutation_aliasing_ranges(
        &mut func.body,
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
