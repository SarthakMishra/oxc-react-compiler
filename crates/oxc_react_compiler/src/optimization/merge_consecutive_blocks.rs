
use crate::hir::types::{BlockId, HIR, Terminal};
use rustc_hash::FxHashMap;

/// Merge basic blocks that have a single predecessor/successor relationship.
///
/// If block A's terminal is `Goto { block: B }` and B has only A as its
/// predecessor, merge B's instructions into A, replace A's terminal with
/// B's terminal, and remove B.
///
/// This simplifies the CFG after control flow lowering, which often creates
/// unnecessary intermediate blocks.
pub fn merge_consecutive_blocks(hir: &mut HIR) {
    loop {
        let merge = find_merge_candidate(hir);
        match merge {
            Some((from, into)) => apply_merge(hir, from, into),
            None => break,
        }
    }
}

/// Find a pair (A, B) where A's terminal is `Goto { block: B }` and B has
/// exactly one predecessor (A).
fn find_merge_candidate(hir: &HIR) -> Option<(BlockId, BlockId)> {
    // Build predecessor counts
    let pred_counts = compute_predecessor_counts(hir);

    for (block_id, block) in &hir.blocks {
        if let Terminal::Goto { block: target } = &block.terminal {
            // B must have exactly one predecessor
            if pred_counts.get(target).copied().unwrap_or(0) == 1 {
                // Don't merge a block into itself
                if block_id != target {
                    return Some((*block_id, *target));
                }
            }
        }
    }
    None
}

/// Count how many times each block appears as a successor.
fn compute_predecessor_counts(hir: &HIR) -> FxHashMap<BlockId, usize> {
    let mut counts: FxHashMap<BlockId, usize> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for succ in terminal_successors(&block.terminal) {
            *counts.entry(succ).or_insert(0) += 1;
        }
    }

    counts
}

/// Merge block `into` into block `from`: append `into`'s instructions to
/// `from`, replace `from`'s terminal with `into`'s terminal, then remove
/// `into` from the HIR.
fn apply_merge(hir: &mut HIR, from: BlockId, into: BlockId) {
    // Extract the target block's data
    let target_pos = hir.blocks.iter().position(|(id, _)| *id == into);
    let target_block = match target_pos {
        Some(pos) => hir.blocks.remove(pos).1,
        None => return,
    };

    // Find the source block and merge
    if let Some((_, source_block)) = hir.blocks.iter_mut().find(|(id, _)| *id == from) {
        source_block.instructions.extend(target_block.instructions);
        source_block.terminal = target_block.terminal;
    }

    // Update the entry block if needed
    if hir.entry == into {
        hir.entry = from;
    }

    // Update any remaining references to `into` to point to `from`
    for (_, block) in &mut hir.blocks {
        rewrite_terminal_target(&mut block.terminal, into, from);
        // Update predecessor lists
        for pred in &mut block.preds {
            if *pred == into {
                *pred = from;
            }
        }
    }
}

/// Rewrite all occurrences of `old` to `new` within a terminal.
fn rewrite_terminal_target(terminal: &mut Terminal, old: BlockId, new: BlockId) {
    let remap = |bid: &mut BlockId| {
        if *bid == old {
            *bid = new;
        }
    };

    match terminal {
        Terminal::Goto { block } => remap(block),
        Terminal::If { consequent, alternate, fallthrough, .. } => {
            remap(consequent);
            remap(alternate);
            remap(fallthrough);
        }
        Terminal::Branch { consequent, alternate, .. } => {
            remap(consequent);
            remap(alternate);
        }
        Terminal::Switch { cases, fallthrough, .. } => {
            for case in cases {
                remap(&mut case.block);
            }
            remap(fallthrough);
        }
        Terminal::Return { .. } | Terminal::Throw { .. } | Terminal::Unreachable => {}
        Terminal::For { init, test, update, body, fallthrough } => {
            remap(init);
            remap(test);
            if let Some(u) = update {
                remap(u);
            }
            remap(body);
            remap(fallthrough);
        }
        Terminal::ForOf { init, test, body, fallthrough }
        | Terminal::ForIn { init, test, body, fallthrough } => {
            remap(init);
            remap(test);
            remap(body);
            remap(fallthrough);
        }
        Terminal::DoWhile { body, test, fallthrough } => {
            remap(body);
            remap(test);
            remap(fallthrough);
        }
        Terminal::While { test, body, fallthrough } => {
            remap(test);
            remap(body);
            remap(fallthrough);
        }
        Terminal::Logical { left, right, fallthrough, .. } => {
            remap(left);
            remap(right);
            remap(fallthrough);
        }
        Terminal::Ternary { consequent, alternate, fallthrough, .. } => {
            remap(consequent);
            remap(alternate);
            remap(fallthrough);
        }
        Terminal::Optional { consequent, fallthrough, .. } => {
            remap(consequent);
            remap(fallthrough);
        }
        Terminal::Sequence { blocks, fallthrough } => {
            for b in blocks {
                remap(b);
            }
            remap(fallthrough);
        }
        Terminal::Label { block, fallthrough, .. } => {
            remap(block);
            remap(fallthrough);
        }
        Terminal::MaybeThrow { continuation, handler } => {
            remap(continuation);
            remap(handler);
        }
        Terminal::Try { block, handler, fallthrough } => {
            remap(block);
            remap(handler);
            remap(fallthrough);
        }
        Terminal::Scope { block, fallthrough, .. }
        | Terminal::PrunedScope { block, fallthrough, .. } => {
            remap(block);
            remap(fallthrough);
        }
    }
}

/// Returns all successor block IDs for a given terminal.
fn terminal_successors(terminal: &Terminal) -> Vec<BlockId> {
    match terminal {
        Terminal::Goto { block } => vec![*block],
        Terminal::If { consequent, alternate, fallthrough, .. } => {
            vec![*consequent, *alternate, *fallthrough]
        }
        Terminal::Branch { consequent, alternate, .. } => vec![*consequent, *alternate],
        Terminal::Switch { cases, fallthrough, .. } => {
            let mut succs: Vec<BlockId> = cases.iter().map(|c| c.block).collect();
            succs.push(*fallthrough);
            succs
        }
        Terminal::Return { .. } | Terminal::Throw { .. } | Terminal::Unreachable => vec![],
        Terminal::For { init, test, update, body, fallthrough } => {
            let mut succs = vec![*init, *test, *body, *fallthrough];
            if let Some(u) = update {
                succs.push(*u);
            }
            succs
        }
        Terminal::ForOf { init, test, body, fallthrough }
        | Terminal::ForIn { init, test, body, fallthrough } => {
            vec![*init, *test, *body, *fallthrough]
        }
        Terminal::DoWhile { body, test, fallthrough } => vec![*body, *test, *fallthrough],
        Terminal::While { test, body, fallthrough } => vec![*test, *body, *fallthrough],
        Terminal::Logical { left, right, fallthrough, .. } => vec![*left, *right, *fallthrough],
        Terminal::Ternary { consequent, alternate, fallthrough, .. } => {
            vec![*consequent, *alternate, *fallthrough]
        }
        Terminal::Optional { consequent, fallthrough, .. } => vec![*consequent, *fallthrough],
        Terminal::Sequence { blocks, fallthrough } => {
            let mut succs = blocks.clone();
            succs.push(*fallthrough);
            succs
        }
        Terminal::Label { block, fallthrough, .. } => vec![*block, *fallthrough],
        Terminal::MaybeThrow { continuation, handler } => vec![*continuation, *handler],
        Terminal::Try { block, handler, fallthrough } => vec![*block, *handler, *fallthrough],
        Terminal::Scope { block, fallthrough, .. }
        | Terminal::PrunedScope { block, fallthrough, .. } => vec![*block, *fallthrough],
    }
}
