#![allow(dead_code)]

//! Compute which HIR blocks execute unconditionally.
//!
//! A block is **unconditional** if every execution path from the function entry
//! must pass through it — i.e., it post-dominates the entry block.
//!
//! This information is consumed by `CollectHoistablePropertyLoads` to determine
//! which property accesses are guaranteed to execute (and thus provide non-null
//! guarantees for hoisting).

use rustc_hash::{FxHashMap, FxHashSet};

use super::types::{BlockId, HIR, Terminal};

/// Set of blocks that execute unconditionally in the given HIR.
pub struct UnconditionalBlocks {
    /// Blocks that post-dominate the entry (always execute).
    pub unconditional: FxHashSet<BlockId>,
    /// Post-dominator map: block → immediate post-dominator.
    pub postdominators: FxHashMap<BlockId, BlockId>,
}

/// Compute the set of unconditional blocks in the HIR.
///
/// A block is unconditional if it post-dominates the entry block, meaning
/// every path from entry to any exit must pass through it.
pub fn compute_unconditional_blocks(hir: &HIR) -> UnconditionalBlocks {
    let block_ids: Vec<BlockId> = hir.blocks.iter().map(|(id, _)| *id).collect();
    if block_ids.is_empty() {
        return UnconditionalBlocks {
            unconditional: FxHashSet::default(),
            postdominators: FxHashMap::default(),
        };
    }

    // Build forward successor map
    let successors = build_successor_map(hir);

    // Find exit blocks (blocks with no successors: Return, Throw, Unreachable)
    let exit_blocks: Vec<BlockId> = block_ids
        .iter()
        .copied()
        .filter(|bid| successors.get(bid).map_or(true, |s| s.is_empty()))
        .collect();

    // If no exit blocks, all blocks are trivially unconditional (infinite loop)
    if exit_blocks.is_empty() {
        return UnconditionalBlocks {
            unconditional: block_ids.into_iter().collect(),
            postdominators: FxHashMap::default(),
        };
    }

    // Create a virtual exit block ID (one higher than max)
    let max_id = block_ids.iter().map(|b| b.0).max().unwrap_or(0);
    let virtual_exit = BlockId(max_id + 1);

    // Build predecessors for the reverse CFG.
    // In the reverse graph, predecessors of node A = forward successors of A.
    // (If A→B in forward, then B→A in reverse, so A has reverse-predecessor B?
    //  No: predecessor of A in reverse = {X : X→A in reverse} = {X : A→X in forward}
    //  = forward_successors(A).)
    let mut reverse_cfg_preds: FxHashMap<BlockId, Vec<BlockId>> = FxHashMap::default();
    reverse_cfg_preds.entry(virtual_exit).or_default(); // entry has no preds
    for &bid in &block_ids {
        reverse_cfg_preds.entry(bid).or_default();
    }
    for (&bid, succs) in &successors {
        reverse_cfg_preds.entry(bid).or_default().extend(succs.iter().copied());
    }
    // Add forward edge exit_block→VirtualExit for each exit block.
    // reverse_cfg_preds[exit_block] = forward_successors(exit_block), which now includes VirtualExit.
    for &exit_bid in &exit_blocks {
        reverse_cfg_preds.entry(exit_bid).or_default().push(virtual_exit);
    }

    // Build reverse CFG successors for DFS (successor of X in reverse = {Y : X→Y in reverse}
    // = {Y : Y→X in forward} = forward predecessors of X).
    let mut reverse_cfg_succs: FxHashMap<BlockId, Vec<BlockId>> = FxHashMap::default();
    for (&bid, succs) in &successors {
        for &s in succs {
            reverse_cfg_succs.entry(s).or_default().push(bid);
        }
    }
    // VirtualExit→exit_block in reverse graph
    for &exit_bid in &exit_blocks {
        reverse_cfg_succs.entry(virtual_exit).or_default().push(exit_bid);
    }

    // Compute reverse post-order of the reverse CFG via DFS from VirtualExit.
    // The Cooper-Harvey-Kennedy algorithm requires blocks in RPO so that
    // intersect() correctly finds the LCA.
    let all_ids = compute_rpo(virtual_exit, &reverse_cfg_succs);

    // Compute post-dominators = dominators on reverse CFG with virtual_exit as entry
    let postdoms = compute_dominators_generic(&all_ids, virtual_exit, &reverse_cfg_preds);

    // A block is unconditional if it post-dominates the entry block.
    // Check: does the original entry block's post-dominator chain include this block?
    let mut unconditional = FxHashSet::default();
    unconditional.insert(hir.entry); // Entry always executes

    // Walk post-dominator chain from entry
    let mut current = hir.entry;
    loop {
        match postdoms.get(&current) {
            Some(&pdom) if pdom != virtual_exit => {
                unconditional.insert(pdom);
                current = pdom;
            }
            _ => break,
        }
    }

    // Also check: any block that post-dominates entry is unconditional
    // We need to check ALL blocks, not just the chain from entry
    for &bid in &block_ids {
        if is_postdominated_by(hir.entry, bid, &postdoms) {
            unconditional.insert(bid);
        }
    }

    UnconditionalBlocks { unconditional, postdominators: postdoms }
}

/// Check if `block` is post-dominated by `candidate` (candidate appears
/// in the post-dominator chain of block).
fn is_postdominated_by(
    block: BlockId,
    candidate: BlockId,
    postdoms: &FxHashMap<BlockId, BlockId>,
) -> bool {
    if block == candidate {
        return true;
    }
    let mut current = block;
    let mut visited = FxHashSet::default();
    loop {
        if !visited.insert(current) {
            return false; // cycle detection
        }
        match postdoms.get(&current) {
            Some(&pdom) => {
                if pdom == candidate {
                    return true;
                }
                current = pdom;
            }
            None => return false,
        }
    }
}

/// Build successor map from HIR.
fn build_successor_map(hir: &HIR) -> FxHashMap<BlockId, Vec<BlockId>> {
    let mut succs = FxHashMap::default();
    for (bid, block) in &hir.blocks {
        succs.insert(*bid, terminal_successors(&block.terminal));
    }
    succs
}

/// Compute reverse post-order of a graph via iterative DFS from the given entry.
fn compute_rpo(entry: BlockId, succs: &FxHashMap<BlockId, Vec<BlockId>>) -> Vec<BlockId> {
    let mut visited = FxHashSet::default();
    let mut post_order = Vec::new();
    let mut stack: Vec<(BlockId, usize)> = vec![(entry, 0)];
    visited.insert(entry);

    while let Some((node, idx)) = stack.last_mut() {
        let children = succs.get(node).cloned().unwrap_or_default();
        if *idx < children.len() {
            let child = children[*idx];
            *idx += 1;
            if visited.insert(child) {
                stack.push((child, 0));
            }
        } else {
            post_order.push(*node);
            stack.pop();
        }
    }

    post_order.reverse();
    post_order
}

/// Extract successor blocks from a terminal.
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

// DIVERGENCE: Post-dominator computation also uses Cooper-Harvey-Kennedy (see
// enter_ssa.rs DIVERGENCE comment). This is consistent with our dominance
// computation choice and sufficient for the CFG sizes seen in React components.
/// Compute immediate dominators using the iterative algorithm (Cooper, Harvey, Kennedy).
/// Generic version that works with any block ID set and predecessor map.
fn compute_dominators_generic(
    block_ids: &[BlockId],
    entry: BlockId,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
) -> FxHashMap<BlockId, BlockId> {
    let id_to_idx: FxHashMap<BlockId, usize> =
        block_ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let n = block_ids.len();

    let mut doms: Vec<Option<usize>> = vec![None; n];
    let entry_idx = id_to_idx[&entry];
    doms[entry_idx] = Some(entry_idx);

    let mut changed = true;
    while changed {
        changed = false;
        for &bid in block_ids {
            let b = id_to_idx[&bid];
            if b == entry_idx {
                continue;
            }
            let pred_list = match preds.get(&bid) {
                Some(p) => p,
                None => continue,
            };
            let mut new_idom = None;
            for p in pred_list {
                if let Some(&pi) = id_to_idx.get(p) {
                    if doms[pi].is_some() {
                        new_idom = Some(pi);
                        break;
                    }
                }
            }
            if let Some(mut new_idom_val) = new_idom {
                for p in pred_list {
                    if let Some(&pi) = id_to_idx.get(p) {
                        if doms[pi].is_some() && pi != new_idom_val {
                            new_idom_val = intersect(&doms, pi, new_idom_val);
                        }
                    }
                }
                if doms[b] != Some(new_idom_val) {
                    doms[b] = Some(new_idom_val);
                    changed = true;
                }
            }
        }
    }

    let mut result = FxHashMap::default();
    for (i, dom) in doms.iter().enumerate() {
        if let Some(d) = dom {
            if i != entry_idx {
                result.insert(block_ids[i], block_ids[*d]);
            }
        }
    }
    result
}

fn intersect(doms: &[Option<usize>], mut a: usize, mut b: usize) -> usize {
    while a != b {
        while a > b {
            a = doms[a].unwrap();
        }
        while b > a {
            b = doms[b].unwrap();
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::types::{BasicBlock, BlockKind, Terminal};

    fn make_block(id: u32, terminal: Terminal) -> (BlockId, BasicBlock) {
        (
            BlockId(id),
            BasicBlock {
                kind: BlockKind::Block,
                id: BlockId(id),
                instructions: Vec::new(),
                terminal,
                preds: Vec::new(),
                phis: Vec::new(),
            },
        )
    }

    #[test]
    fn test_linear_cfg_all_unconditional() {
        // Entry(0) → Block(1) → Block(2) → Return
        let hir = HIR {
            entry: BlockId(0),
            blocks: vec![
                make_block(0, Terminal::Goto { block: BlockId(1) }),
                make_block(1, Terminal::Goto { block: BlockId(2) }),
                make_block(
                    2,
                    Terminal::Return {
                        value: crate::hir::types::Place {
                            identifier: crate::hir::types::Identifier {
                                id: crate::hir::types::IdentifierId(0),
                                declaration_id: None,
                                name: None,
                                mutable_range: crate::hir::types::MutableRange {
                                    start: crate::hir::types::InstructionId(0),
                                    end: crate::hir::types::InstructionId(0),
                                },
                                scope: None,
                                type_: crate::hir::types::Type::default(),
                                loc: oxc_span::Span::default(),
                            },
                            effect: crate::hir::types::Effect::Unknown,
                            reactive: false,
                            loc: oxc_span::Span::default(),
                        },
                    },
                ),
            ],
        };

        let result = compute_unconditional_blocks(&hir);
        assert!(result.unconditional.contains(&BlockId(0)));
        assert!(result.unconditional.contains(&BlockId(1)));
        assert!(result.unconditional.contains(&BlockId(2)));
    }

    #[test]
    fn test_conditional_branch() {
        // Entry(0) → If → Block(1) or Block(2) → Block(3) → Return
        // Block(1) and Block(2) are conditional, Block(3) is unconditional (fallthrough)
        let hir = HIR {
            entry: BlockId(0),
            blocks: vec![
                make_block(
                    0,
                    Terminal::Branch {
                        test: crate::hir::types::Place {
                            identifier: crate::hir::types::Identifier {
                                id: crate::hir::types::IdentifierId(0),
                                declaration_id: None,
                                name: None,
                                mutable_range: crate::hir::types::MutableRange {
                                    start: crate::hir::types::InstructionId(0),
                                    end: crate::hir::types::InstructionId(0),
                                },
                                scope: None,
                                type_: crate::hir::types::Type::default(),
                                loc: oxc_span::Span::default(),
                            },
                            effect: crate::hir::types::Effect::Unknown,
                            reactive: false,
                            loc: oxc_span::Span::default(),
                        },
                        consequent: BlockId(1),
                        alternate: BlockId(2),
                    },
                ),
                make_block(1, Terminal::Goto { block: BlockId(3) }),
                make_block(2, Terminal::Goto { block: BlockId(3) }),
                make_block(
                    3,
                    Terminal::Return {
                        value: crate::hir::types::Place {
                            identifier: crate::hir::types::Identifier {
                                id: crate::hir::types::IdentifierId(0),
                                declaration_id: None,
                                name: None,
                                mutable_range: crate::hir::types::MutableRange {
                                    start: crate::hir::types::InstructionId(0),
                                    end: crate::hir::types::InstructionId(0),
                                },
                                scope: None,
                                type_: crate::hir::types::Type::default(),
                                loc: oxc_span::Span::default(),
                            },
                            effect: crate::hir::types::Effect::Unknown,
                            reactive: false,
                            loc: oxc_span::Span::default(),
                        },
                    },
                ),
            ],
        };

        let result = compute_unconditional_blocks(&hir);
        assert!(result.unconditional.contains(&BlockId(0)), "entry is unconditional");
        assert!(!result.unconditional.contains(&BlockId(1)), "consequent is conditional");
        assert!(!result.unconditional.contains(&BlockId(2)), "alternate is conditional");
        assert!(result.unconditional.contains(&BlockId(3)), "fallthrough is unconditional");
    }
}
