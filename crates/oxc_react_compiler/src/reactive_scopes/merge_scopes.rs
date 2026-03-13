use crate::hir::types::{DependencyPathEntry, HIR, InstructionId, ReactiveFunction, ScopeId};
use std::collections::BTreeSet;

/// Merge overlapping reactive scopes in the HIR.
///
/// Two scopes overlap if their MutableRanges intersect.
/// Overlapping scopes are merged into a single scope.
pub fn merge_overlapping_reactive_scopes_hir(hir: &mut HIR) {
    // Collect all scopes from instructions
    let mut scopes: Vec<(ScopeId, u32, u32)> = Vec::new();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                scopes.push((scope.id, scope.range.start.0, scope.range.end.0));
            }
        }
    }

    if scopes.is_empty() {
        return;
    }

    // Sort by start position
    scopes.sort_by_key(|s| s.1);

    // Find merge groups: overlapping ranges get merged
    let mut merge_map: rustc_hash::FxHashMap<ScopeId, ScopeId> = rustc_hash::FxHashMap::default();
    let mut merged_ranges: Vec<(ScopeId, u32, u32)> = Vec::new();

    for (scope_id, start, end) in &scopes {
        if let Some(last) = merged_ranges.last_mut() {
            if *start <= last.2 {
                // Overlapping — merge into the existing range
                last.2 = last.2.max(*end);
                merge_map.insert(*scope_id, last.0);
            } else {
                merged_ranges.push((*scope_id, *start, *end));
            }
        } else {
            merged_ranges.push((*scope_id, *start, *end));
        }
    }

    // Apply the merge map: update scopes that were merged
    if merge_map.is_empty() {
        return;
    }

    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(ref mut scope) = instr.lvalue.identifier.scope
                && let Some(&target) = merge_map.get(&scope.id)
            {
                // Find the merged range for the target
                if let Some(merged) = merged_ranges.iter().find(|m| m.0 == target) {
                    scope.id = target;
                    scope.range.start = InstructionId(merged.1);
                    scope.range.end = InstructionId(merged.2);
                }
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
