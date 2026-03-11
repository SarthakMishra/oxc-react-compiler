#![allow(dead_code)]

use rustc_hash::FxHashMap;

use crate::hir::types::{AliasingEffect, Effect, HIR, IdentifierId, InstructionId, MutableRange};

/// Compute mutable ranges for all identifiers.
///
/// Uses the effects computed by `infer_mutation_aliasing_effects` to determine
/// the instruction range during which each value is being mutated.
///
/// - `start`: instruction that creates the value
/// - `end`: last instruction that mutates the value (transitively through aliases)
pub fn infer_mutation_aliasing_ranges(hir: &mut HIR) {
    // Step 1: Build a map of each identifier to its creation point and all mutation sites.
    let mut creation_map: FxHashMap<IdentifierId, InstructionId> = FxHashMap::default();
    let mut mutation_map: FxHashMap<IdentifierId, Vec<InstructionId>> = FxHashMap::default();
    let mut alias_map: FxHashMap<IdentifierId, Vec<IdentifierId>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let instr_id = instr.id;

            // The lvalue's identifier is created at this instruction.
            let lvalue_id = instr.lvalue.identifier.id;
            creation_map.entry(lvalue_id).or_insert(instr_id);

            // Process effects to find mutations and aliases.
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    match effect {
                        AliasingEffect::Create { into, .. }
                        | AliasingEffect::CreateFrom { into, .. }
                        | AliasingEffect::CreateFunction { into, .. } => {
                            creation_map.entry(into.identifier.id).or_insert(instr_id);
                        }
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value } => {
                            mutation_map.entry(value.identifier.id).or_default().push(instr_id);
                        }
                        AliasingEffect::Alias { from, into }
                        | AliasingEffect::Assign { from, into }
                        | AliasingEffect::MaybeAlias { from, into } => {
                            alias_map
                                .entry(from.identifier.id)
                                .or_default()
                                .push(into.identifier.id);
                            alias_map
                                .entry(into.identifier.id)
                                .or_default()
                                .push(from.identifier.id);
                        }
                        AliasingEffect::Capture { from, into } => {
                            // Capture creates an indirect alias for mutation propagation.
                            alias_map
                                .entry(into.identifier.id)
                                .or_default()
                                .push(from.identifier.id);
                        }
                        _ => {}
                    }
                }
            }

            // Also check the lvalue's effect.
            if matches!(
                instr.lvalue.effect,
                Effect::Mutate | Effect::ConditionallyMutate | Effect::Store
            ) {
                mutation_map.entry(lvalue_id).or_default().push(instr_id);
            }
        }
    }

    // Step 2: Propagate mutation sites through aliases transitively.
    // If A aliases B and B is mutated at instruction N, A's last mutation extends to N.
    let mut all_ids: Vec<IdentifierId> = creation_map.keys().copied().collect();
    all_ids.sort();

    // Build transitive closure of aliases.
    let mut transitive_mutations: FxHashMap<IdentifierId, InstructionId> = FxHashMap::default();

    for &id in &all_ids {
        let mut last_mutation = InstructionId(0);

        // Direct mutations.
        if let Some(mutations) = mutation_map.get(&id) {
            for &m in mutations {
                if m.0 > last_mutation.0 {
                    last_mutation = m;
                }
            }
        }

        // Mutations through aliases (one level of transitivity).
        if let Some(aliases) = alias_map.get(&id) {
            for &alias_id in aliases {
                if let Some(mutations) = mutation_map.get(&alias_id) {
                    for &m in mutations {
                        if m.0 > last_mutation.0 {
                            last_mutation = m;
                        }
                    }
                }
            }
        }

        if last_mutation.0 > 0 {
            transitive_mutations.insert(id, last_mutation);
        }
    }

    // Step 3: Write mutable ranges back to identifiers.
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            let id = instr.lvalue.identifier.id;
            let start = creation_map.get(&id).copied().unwrap_or(instr.id);

            let end = transitive_mutations
                .get(&id)
                .map_or(InstructionId(start.0 + 1), |&last| InstructionId(last.0 + 1));

            instr.lvalue.identifier.mutable_range = MutableRange { start, end };
        }
    }
}
