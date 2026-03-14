use crate::hir::types::{
    AliasingEffect, DependencyPathEntry, HIR, IdentifierId, InstructionId, ReactiveFunction,
    ScopeId,
};
use crate::utils::disjoint_set::DisjointSet;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeSet;

/// Merge overlapping reactive scopes in the HIR.
///
/// Uses an active-scope-stack algorithm matching the upstream
/// `MergeOverlappingReactiveScopesHIR.ts`. Three phases:
///
/// 1. **Index** — Build maps of scope starts/ends per instruction ID,
///    place-to-scope ownership, and mutation sites.
/// 2. **Stack walk** — Walk instructions in ascending ID order maintaining
///    an active scope stack. Merge scopes on geometric overlap (a scope
///    ends while not at stack top) and cross-scope mutation (a mutation
///    targets a place owned by a scope different from the stack top).
/// 3. **Rewrite** — Compute merged ranges and update all identifier scope
///    annotations in-place.
pub fn merge_overlapping_reactive_scopes_hir(hir: &mut HIR) {
    // -----------------------------------------------------------------------
    // Phase 1: Build index
    // -----------------------------------------------------------------------

    // scope_data: canonical (start, end) for each ScopeId
    let mut scope_data: FxHashMap<ScopeId, (u32, u32)> = FxHashMap::default();
    // scope_starts[instr_id] = set of ScopeIds starting at that instruction
    let mut scope_starts: FxHashMap<u32, Vec<ScopeId>> = FxHashMap::default();
    // scope_ends[instr_id] = set of ScopeIds ending at that instruction
    let mut scope_ends: FxHashMap<u32, Vec<ScopeId>> = FxHashMap::default();
    // place_scope: which scope owns each identifier
    let mut place_scope: FxHashMap<IdentifierId, ScopeId> = FxHashMap::default();
    // mutated_at[instr_id] = list of IdentifierIds mutated at that instruction
    let mut mutated_at: FxHashMap<u32, Vec<IdentifierId>> = FxHashMap::default();
    // Track which ScopeIds we've already indexed (avoid duplicate entries)
    let mut seen_scopes: FxHashSet<ScopeId> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Index scope annotations
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                let sid = scope.id;
                let start = scope.range.start.0;
                let end = scope.range.end.0;

                // Map place to its owning scope
                place_scope.insert(instr.lvalue.identifier.id, sid);

                // Only index each scope once (many identifiers share the same scope)
                if seen_scopes.insert(sid) {
                    scope_data.insert(sid, (start, end));
                    scope_starts.entry(start).or_default().push(sid);
                    scope_ends.entry(end).or_default().push(sid);
                }
            }

            // Index mutation effects
            if let Some(ref effects) = instr.effects {
                let iid = instr.id.0;
                for effect in effects {
                    let mutated_place = match effect {
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value } => {
                            Some(&value.identifier.id)
                        }
                        _ => None,
                    };
                    if let Some(id) = mutated_place {
                        mutated_at.entry(iid).or_default().push(*id);
                    }
                }
            }
        }
    }

    if scope_data.is_empty() {
        return;
    }

    // -----------------------------------------------------------------------
    // Phase 2: Active-scope-stack walk with union-find
    // -----------------------------------------------------------------------

    let mut dsu: DisjointSet<ScopeId> = DisjointSet::new();
    for &sid in scope_data.keys() {
        dsu.make_set(sid);
    }

    // Collect all interesting instruction IDs and sort them
    let mut all_ids: BTreeSet<u32> = BTreeSet::new();
    for &id in scope_starts.keys() {
        all_ids.insert(id);
    }
    for &id in scope_ends.keys() {
        all_ids.insert(id);
    }
    for &id in mutated_at.keys() {
        all_ids.insert(id);
    }

    // Active scope stack: entries are (ScopeId, end_instr_id).
    // The stack top (last element) is the innermost/shallowest scope.
    let mut active: Vec<(ScopeId, u32)> = Vec::new();

    for instr_id in all_ids {
        // 1. Process scope ends at this instruction
        if let Some(ending_scopes) = scope_ends.get(&instr_id) {
            for &ending_scope in ending_scopes {
                let ending_rep = match dsu.find(ending_scope) {
                    Some(r) => r,
                    None => continue,
                };

                // Find this scope's representative in the active stack (rightmost)
                let stack_pos =
                    active.iter().rposition(|(sid, _)| dsu.find(*sid) == Some(ending_rep));

                if let Some(pos) = stack_pos {
                    // Merge everything above pos with ending_scope
                    for &(above_sid, _) in &active[(pos + 1)..] {
                        dsu.union(ending_scope, above_sid);
                    }
                    // Pop pos and everything above it
                    active.truncate(pos);
                }
            }
        }

        // 2. Process scope starts at this instruction
        if let Some(starting_scopes) = scope_starts.get(&instr_id) {
            // Sort descending by end time; when pushed in order, the earliest-ending
            // scope ends up at the stack top (last element = innermost active scope).
            let mut to_push: Vec<(ScopeId, u32)> = starting_scopes
                .iter()
                .filter_map(|&sid| scope_data.get(&sid).map(|&(_, end)| (sid, end)))
                .collect();
            to_push.sort_by(|a, b| b.1.cmp(&a.1));

            for (sid, end) in to_push {
                // Check for identical-range scope already on stack → union instead of push
                let existing = active.iter().find(|(s, e)| {
                    *e == end && scope_data.get(s).is_some_and(|&(start, _)| start == instr_id)
                });
                if let Some(&(existing_sid, _)) = existing {
                    dsu.union(sid, existing_sid);
                } else {
                    active.push((sid, end));
                }
            }
        }

        // 3. Process mutations at this instruction
        if let Some(mutated_ids) = mutated_at.get(&instr_id)
            && let Some(&(top_sid, _)) = active.last()
        {
            let Some(top_rep) = dsu.find(top_sid) else { continue };
            for &identifier_id in mutated_ids {
                if let Some(&mutated_scope_id) = place_scope.get(&identifier_id) {
                    let mutated_rep = match dsu.find(mutated_scope_id) {
                        Some(r) => r,
                        None => continue,
                    };
                    if mutated_rep != top_rep {
                        // Find the mutated scope's position in the stack
                        let stack_pos =
                            active.iter().rposition(|(sid, _)| dsu.find(*sid) == Some(mutated_rep));
                        if let Some(pos) = stack_pos {
                            // Merge everything above pos with the mutated scope
                            for &(above_sid, _) in &active[(pos + 1)..] {
                                dsu.union(mutated_scope_id, above_sid);
                            }
                            // Do NOT pop — the scope is still active; only merge
                        }
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 3: Compute merged ranges and rewrite
    // -----------------------------------------------------------------------

    let groups = dsu.sets();

    // For each group, compute merged (min_start, max_end)
    let mut group_range: FxHashMap<ScopeId, (u32, u32)> = FxHashMap::default();
    for (rep, members) in &groups {
        let (min_start, max_end) = members
            .iter()
            .filter_map(|sid| scope_data.get(sid))
            .fold((u32::MAX, 0u32), |(ms, me), &(s, e)| (ms.min(s), me.max(e)));
        group_range.insert(*rep, (min_start, max_end));
    }

    // Build remap: old ScopeId → (representative ScopeId, new_start, new_end)
    let mut remap: FxHashMap<ScopeId, (ScopeId, u32, u32)> = FxHashMap::default();
    for (rep, members) in &groups {
        if let Some(&(min_s, max_e)) = group_range.get(rep) {
            for &member in members {
                remap.insert(member, (*rep, min_s, max_e));
            }
        }
    }

    // Apply remap to all identifier scope annotations
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(ref mut scope) = instr.lvalue.identifier.scope
                && let Some(&(rep, new_start, new_end)) = remap.get(&scope.id)
            {
                scope.id = rep;
                scope.range.start = InstructionId(new_start);
                scope.range.end = InstructionId(new_end);
            }
        }
    }
}

/// Merge reactive scopes that invalidate together.
///
/// If two scopes have the same set of dependencies, they should be merged
/// because they'll always recompute at the same time.
pub fn merge_reactive_scopes_that_invalidate_together(reactive_fn: &mut ReactiveFunction) {
    // Walk the reactive function tree and find scopes with identical dependency sets
    merge_scopes_in_block(&mut reactive_fn.body);
}

/// Canonical dependency key for comparing scope deps by name + path,
/// not by IdentifierId (which is SSA-unique per Place reference).
/// DIVERGENCE: Upstream uses identifier name + property path for dep comparison.
/// Our HIR creates fresh IdentifierIds per Place, so ID comparison would
/// make almost all scopes appear to have different deps.
type DepKey = (Option<String>, Vec<DependencyPathEntry>);

fn dep_key_set(scope: &crate::hir::types::ReactiveScope) -> BTreeSet<DepKey> {
    scope.dependencies.iter().map(|d| (d.identifier.name.clone(), d.path.clone())).collect()
}

fn merge_scopes_in_block(block: &mut crate::hir::types::ReactiveBlock) {
    // Collect scope indices with their canonical dependency keys
    let mut scope_indices: Vec<(usize, BTreeSet<DepKey>)> = Vec::new();

    for (i, instr) in block.instructions.iter().enumerate() {
        if let crate::hir::types::ReactiveInstruction::Scope(scope_block) = instr {
            scope_indices.push((i, dep_key_set(&scope_block.scope)));
        }
    }

    // Find consecutive pairs with identical deps and merge them.
    // DIVERGENCE: Upstream MergeReactiveScopesThatInvalidateTogether.ts compares
    // scopes by named dependency path (identifier name + property path), not by
    // IdentifierId. Our HIR creates fresh IDs per Place, so ID comparison makes
    // almost all scopes appear to have different deps. Name-based comparison
    // correctly identifies scopes that invalidate together.
    //
    // We only merge scopes with strictly identical dep sets (not subsets) to
    // match upstream semantics: "invalidate together" means the same deps.
    let mut merged_indices: rustc_hash::FxHashSet<usize> = rustc_hash::FxHashSet::default();
    let mut to_merge: Vec<(usize, usize)> = Vec::new();
    for i in 0..scope_indices.len() {
        if merged_indices.contains(&i) {
            continue;
        }
        for j in (i + 1)..scope_indices.len() {
            if merged_indices.contains(&j) {
                continue;
            }
            let (_, ref a_deps) = scope_indices[i];
            let (_, ref b_deps) = scope_indices[j];
            // Merge only when deps are identical and non-empty
            if !a_deps.is_empty() && a_deps == b_deps {
                to_merge.push((scope_indices[i].0, scope_indices[j].0));
                merged_indices.insert(j); // Prevent double-merge of scope j
            }
        }
    }

    // Merge scopes: move second scope's instructions into first.
    // Process in reverse to preserve indices.
    for &(first_idx, second_idx) in to_merge.iter().rev() {
        if second_idx < block.instructions.len() && first_idx < block.instructions.len() {
            let second = block.instructions.remove(second_idx);
            if let crate::hir::types::ReactiveInstruction::Scope(second_scope) = second
                && let Some(crate::hir::types::ReactiveInstruction::Scope(first_scope)) =
                    block.instructions.get_mut(first_idx)
            {
                // Merge: extend first scope's instructions with second's
                first_scope
                    .instructions
                    .instructions
                    .extend(second_scope.instructions.instructions);
                // Union the dependency sets using a set for dedup
                let existing_keys = dep_key_set(&first_scope.scope);
                for dep in &second_scope.scope.dependencies {
                    let key = (dep.identifier.name.clone(), dep.path.clone());
                    if !existing_keys.contains(&key) {
                        first_scope.scope.dependencies.push(dep.clone());
                    }
                }
                // Merge declarations
                first_scope.scope.declarations.extend(second_scope.scope.declarations);
                // Track merged scope ID
                first_scope.scope.merged.push(second_scope.scope.id);
            }
        }
    }

    // Recurse into nested blocks
    for instr in &mut block.instructions {
        match instr {
            crate::hir::types::ReactiveInstruction::Scope(scope_block) => {
                merge_scopes_in_block(&mut scope_block.instructions);
            }
            crate::hir::types::ReactiveInstruction::Terminal(terminal) => {
                merge_scopes_in_terminal(terminal);
            }
            crate::hir::types::ReactiveInstruction::Instruction(_) => {}
        }
    }
}

fn merge_scopes_in_terminal(terminal: &mut crate::hir::types::ReactiveTerminal) {
    use crate::hir::types::ReactiveTerminal;
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            merge_scopes_in_block(consequent);
            merge_scopes_in_block(alternate);
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            merge_scopes_in_block(init);
            merge_scopes_in_block(test);
            if let Some(upd) = update {
                merge_scopes_in_block(upd);
            }
            merge_scopes_in_block(body);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            merge_scopes_in_block(init);
            merge_scopes_in_block(test);
            merge_scopes_in_block(body);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            merge_scopes_in_block(test);
            merge_scopes_in_block(body);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                merge_scopes_in_block(block);
            }
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            merge_scopes_in_block(block);
            merge_scopes_in_block(handler);
        }
        ReactiveTerminal::Label { block, .. } => {
            merge_scopes_in_block(block);
        }
        ReactiveTerminal::Return { .. } | ReactiveTerminal::Throw { .. } => {}
    }
}
