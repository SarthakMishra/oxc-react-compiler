use rustc_hash::FxHashMap;

use crate::error::ErrorCollector;
use crate::hir::types::{
    Effect, FunctionSignature, HIR, HIRFunction, IdentifierId, InstructionValue, Param, ParamEffect,
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
    FunctionSignature { params, return_effect: func.returns.effect, callee_effect: Effect::Read }
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
