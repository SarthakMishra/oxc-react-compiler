#![allow(dead_code)]

use crate::hir::types::{HIR, InstructionId, ReactiveFunction, ScopeId};

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
                && let Some(&target) = merge_map.get(&scope.id) {
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

fn merge_scopes_in_block(block: &mut crate::hir::types::ReactiveBlock) {
    // Collect scope indices that share the same dependencies
    let mut scope_indices: Vec<(usize, Vec<crate::hir::types::IdentifierId>)> = Vec::new();

    for (i, instr) in block.instructions.iter().enumerate() {
        if let crate::hir::types::ReactiveInstruction::Scope(scope_block) = instr {
            let dep_ids: Vec<crate::hir::types::IdentifierId> =
                scope_block.scope.dependencies.iter().map(|d| d.identifier.id).collect();
            scope_indices.push((i, dep_ids));
        }
    }

    // Find pairs with identical deps and merge them
    // We merge from the end to avoid invalidating indices
    let mut to_merge: Vec<(usize, usize)> = Vec::new();
    for i in 0..scope_indices.len() {
        for j in (i + 1)..scope_indices.len() {
            if scope_indices[i].1 == scope_indices[j].1 && !scope_indices[i].1.is_empty() {
                to_merge.push((scope_indices[i].0, scope_indices[j].0));
            }
        }
    }

    // For now, just merge the instructions of adjacent scopes with same deps
    // A full implementation would restructure the tree more aggressively
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
                    // Merge the merged list
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
