#![allow(dead_code)]

use crate::hir::types::{BlockId, HIR, IdentifierId, InstructionId, Phi, Place};
use rustc_hash::{FxHashMap, FxHashSet};

/// Enter SSA form: insert phi nodes and rename identifiers.
///
/// Standard algorithm (Cytron et al.):
/// 1. Compute dominance frontiers
/// 2. Insert phi nodes at dominance frontiers for each variable
/// 3. Rename identifiers using dominator tree walk
///
/// After the stable-IdentifierId refactor, all references to the same binding
/// share the same `IdentifierId`. The SSA pass distinguishes versions via the
/// `ssa_version` field on `Identifier` instead of creating fresh IDs.
pub fn enter_ssa(hir: &mut HIR) {
    let block_ids: Vec<BlockId> = hir.blocks.iter().map(|(id, _)| *id).collect();
    if block_ids.is_empty() {
        return;
    }

    let entry = hir.entry;

    // Step 1: Build predecessor map and compute dominators
    let preds = build_predecessor_map(hir);
    let dominators = compute_dominators(&block_ids, entry, &preds);
    let dom_tree = build_dominator_tree(&block_ids, &dominators);
    let dom_frontiers = compute_dominance_frontiers(&block_ids, &preds, &dominators);

    // Step 2: Find all variables (IdentifierIds) and their definition sites
    let defs = find_variable_definitions(hir);

    // Step 3: Insert phi nodes
    insert_phi_nodes(hir, &defs, &dom_frontiers);

    // Step 4: Rename variables (SSA versioning)
    rename_variables(hir, entry, &dom_tree, &preds);
}

/// Build a map from block -> predecessor blocks
fn build_predecessor_map(hir: &HIR) -> FxHashMap<BlockId, Vec<BlockId>> {
    let mut preds: FxHashMap<BlockId, Vec<BlockId>> = FxHashMap::default();
    for (id, _) in &hir.blocks {
        preds.entry(*id).or_default();
    }
    for (id, block) in &hir.blocks {
        for succ in terminal_successors(&block.terminal) {
            preds.entry(succ).or_default().push(*id);
        }
    }
    preds
}

/// Get successor block IDs from a terminal
fn terminal_successors(terminal: &crate::hir::types::Terminal) -> Vec<BlockId> {
    use crate::hir::types::Terminal;
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

// DIVERGENCE: Dominance computation uses the Cooper-Harvey-Kennedy iterative
// algorithm instead of Lengauer-Tarjan. CHK is simpler to implement and debug,
// and performs well on the small CFGs typical of React components (usually
// <100 blocks). Upstream babel-plugin-react-compiler uses a different approach.
/// Compute immediate dominators using the iterative algorithm (Cooper, Harvey, Kennedy).
/// Returns a map from block -> immediate dominator.
fn compute_dominators(
    block_ids: &[BlockId],
    entry: BlockId,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
) -> FxHashMap<BlockId, BlockId> {
    // Map block ID to index for efficient intersection
    let id_to_idx: FxHashMap<BlockId, usize> =
        block_ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let n = block_ids.len();

    // Initialize: entry dominates itself, everything else undefined
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
            let pred_list = &preds[&bid];
            // Find first processed predecessor
            let mut new_idom = None;
            for p in pred_list {
                if let Some(&pi) = id_to_idx.get(p)
                    && doms[pi].is_some()
                {
                    new_idom = Some(pi);
                    break;
                }
            }
            if let Some(mut new_idom_val) = new_idom {
                for p in pred_list {
                    if let Some(&pi) = id_to_idx.get(p)
                        && doms[pi].is_some()
                        && pi != new_idom_val
                    {
                        new_idom_val = intersect(&doms, pi, new_idom_val);
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
        if let Some(d) = dom
            && i != entry_idx
        {
            result.insert(block_ids[i], block_ids[*d]);
        }
    }
    result
}

fn intersect(doms: &[Option<usize>], mut a: usize, mut b: usize) -> usize {
    while a != b {
        while a > b {
            a = doms[a].unwrap_or_else(|| {
                panic!("dominator for block index {a} must be computed during intersect")
            });
        }
        while b > a {
            b = doms[b].unwrap_or_else(|| {
                panic!("dominator for block index {b} must be computed during intersect")
            });
        }
    }
    a
}

/// Build dominator tree: map from block -> children in dom tree
fn build_dominator_tree(
    block_ids: &[BlockId],
    dominators: &FxHashMap<BlockId, BlockId>,
) -> FxHashMap<BlockId, Vec<BlockId>> {
    let mut tree: FxHashMap<BlockId, Vec<BlockId>> = FxHashMap::default();
    for &bid in block_ids {
        tree.entry(bid).or_default();
    }
    for (&child, &parent) in dominators {
        tree.entry(parent).or_default().push(child);
    }
    tree
}

/// Compute dominance frontiers for all blocks
fn compute_dominance_frontiers(
    block_ids: &[BlockId],
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
    dominators: &FxHashMap<BlockId, BlockId>,
) -> FxHashMap<BlockId, FxHashSet<BlockId>> {
    let mut frontiers: FxHashMap<BlockId, FxHashSet<BlockId>> = FxHashMap::default();
    for &bid in block_ids {
        frontiers.entry(bid).or_default();
    }

    for &bid in block_ids {
        let pred_list = &preds[&bid];
        if pred_list.len() >= 2 {
            for &p in pred_list {
                let mut runner = p;
                while Some(&runner) != dominators.get(&bid) && runner != bid {
                    frontiers.entry(runner).or_default().insert(bid);
                    match dominators.get(&runner) {
                        Some(&d) => runner = d,
                        None => break,
                    }
                }
            }
        }
    }
    frontiers
}

/// Find all variables defined in each block (including phi defs)
fn find_variable_definitions(hir: &HIR) -> FxHashMap<IdentifierId, FxHashSet<BlockId>> {
    let mut defs: FxHashMap<IdentifierId, FxHashSet<BlockId>> = FxHashMap::default();
    for (block_id, block) in &hir.blocks {
        for instr in &block.instructions {
            defs.entry(instr.lvalue.identifier.id).or_default().insert(*block_id);
        }
    }
    defs
}

/// Insert phi nodes at dominance frontiers for each variable
fn insert_phi_nodes(
    hir: &mut HIR,
    defs: &FxHashMap<IdentifierId, FxHashSet<BlockId>>,
    dom_frontiers: &FxHashMap<BlockId, FxHashSet<BlockId>>,
) {
    for (var_id, def_blocks) in defs {
        let mut worklist: Vec<BlockId> = def_blocks.iter().copied().collect();
        let mut has_phi: FxHashSet<BlockId> = FxHashSet::default();

        while let Some(block_id) = worklist.pop() {
            if let Some(frontier) = dom_frontiers.get(&block_id) {
                for &df_block in frontier {
                    if has_phi.insert(df_block) {
                        // Find a representative place for this variable
                        if let Some(place) = find_place_for_var(hir, *var_id) {
                            let phi = Phi {
                                id: InstructionId(0), // Will be assigned during renaming
                                place: place.clone(),
                                operands: Vec::new(),
                            };
                            if let Some(block) =
                                hir.blocks.iter_mut().find(|(id, _)| *id == df_block)
                            {
                                block.1.phis.push(phi);
                            }
                            worklist.push(df_block);
                        }
                    }
                }
            }
        }
    }
}

/// Find a Place with the given IdentifierId in the HIR
fn find_place_for_var(hir: &HIR, var_id: IdentifierId) -> Option<&Place> {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id == var_id {
                return Some(&instr.lvalue);
            }
        }
    }
    None
}

/// Rename variables to SSA form by walking the dominator tree.
///
/// Instead of creating fresh IdentifierIds, this sets the `ssa_version` field
/// on each Place's Identifier. All references to the same binding keep the
/// same IdentifierId; only the version distinguishes them.
fn rename_variables(
    hir: &mut HIR,
    entry: BlockId,
    dom_tree: &FxHashMap<BlockId, Vec<BlockId>>,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
) {
    // Map from base IdentifierId -> stack of SSA version numbers.
    // The top of the stack is the "current" version for that variable.
    let mut stacks: FxHashMap<IdentifierId, Vec<u32>> = FxHashMap::default();

    // Per-variable counter for generating fresh version numbers.
    let mut version_counters: FxHashMap<IdentifierId, u32> = FxHashMap::default();

    // Build a quick lookup: block_id -> index in hir.blocks
    let block_index: FxHashMap<BlockId, usize> =
        hir.blocks.iter().enumerate().map(|(i, (id, _))| (*id, i)).collect();

    // Precompute successors per block
    let successors: FxHashMap<BlockId, Vec<BlockId>> =
        hir.blocks.iter().map(|(id, block)| (*id, terminal_successors(&block.terminal))).collect();

    rename_block(
        hir,
        entry,
        dom_tree,
        &mut stacks,
        &mut version_counters,
        &block_index,
        &successors,
        preds,
    );
}

/// Allocate the next SSA version for `base_id` and push it onto the stack.
fn fresh_ssa_version(
    base_id: IdentifierId,
    stacks: &mut FxHashMap<IdentifierId, Vec<u32>>,
    version_counters: &mut FxHashMap<IdentifierId, u32>,
) -> u32 {
    let counter = version_counters.entry(base_id).or_insert(0);
    let version = *counter;
    *counter += 1;
    stacks.entry(base_id).or_default().push(version);
    version
}

/// Return the current SSA version for `base_id` (top of stack), or 0 if none.
fn current_ssa_version(base_id: IdentifierId, stacks: &FxHashMap<IdentifierId, Vec<u32>>) -> u32 {
    stacks.get(&base_id).and_then(|stack| stack.last().copied()).unwrap_or(0)
}

fn rename_place_use(place: &mut Place, stacks: &FxHashMap<IdentifierId, Vec<u32>>) {
    let base_id = place.identifier.id;
    place.identifier.ssa_version = current_ssa_version(base_id, stacks);
}

fn rename_place_def(
    place: &mut Place,
    stacks: &mut FxHashMap<IdentifierId, Vec<u32>>,
    version_counters: &mut FxHashMap<IdentifierId, u32>,
    push_count: &mut FxHashMap<IdentifierId, u32>,
) {
    let base_id = place.identifier.id;
    let version = fresh_ssa_version(base_id, stacks, version_counters);
    *push_count.entry(base_id).or_insert(0) += 1;
    place.identifier.ssa_version = version;
}

/// Rename uses within an InstructionValue (all operand positions)
fn rename_instruction_value_uses(
    value: &mut crate::hir::types::InstructionValue,
    stacks: &FxHashMap<IdentifierId, Vec<u32>>,
) {
    use crate::hir::types::InstructionValue;
    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            rename_place_use(place, stacks);
        }
        InstructionValue::StoreLocal { lvalue, value, .. } => {
            rename_place_use(value, stacks);
            rename_place_use(lvalue, stacks);
        }
        InstructionValue::StoreContext { lvalue, value } => {
            rename_place_use(value, stacks);
            rename_place_use(lvalue, stacks);
        }
        InstructionValue::DeclareLocal { lvalue, .. } => {
            rename_place_use(lvalue, stacks);
        }
        InstructionValue::DeclareContext { lvalue } => {
            rename_place_use(lvalue, stacks);
        }
        InstructionValue::Destructure { value, .. } => {
            rename_place_use(value, stacks);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            rename_place_use(left, stacks);
            rename_place_use(right, stacks);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            rename_place_use(value, stacks);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            rename_place_use(lvalue, stacks);
        }
        InstructionValue::CallExpression { callee, args, .. } => {
            rename_place_use(callee, stacks);
            for arg in args {
                rename_place_use(arg, stacks);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            rename_place_use(receiver, stacks);
            for arg in args {
                rename_place_use(arg, stacks);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            rename_place_use(callee, stacks);
            for arg in args {
                rename_place_use(arg, stacks);
            }
        }
        InstructionValue::PropertyLoad { object, .. } => {
            rename_place_use(object, stacks);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            rename_place_use(object, stacks);
            rename_place_use(value, stacks);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            rename_place_use(object, stacks);
            rename_place_use(property, stacks);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            rename_place_use(object, stacks);
            rename_place_use(property, stacks);
            rename_place_use(value, stacks);
        }
        InstructionValue::PropertyDelete { object, .. } => {
            rename_place_use(object, stacks);
        }
        InstructionValue::ComputedDelete { object, property } => {
            rename_place_use(object, stacks);
            rename_place_use(property, stacks);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                rename_place_use(&mut prop.value, stacks);
                if let crate::hir::types::ObjectPropertyKey::Computed(p) = &mut prop.key {
                    rename_place_use(p, stacks);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Spread(p)
                    | crate::hir::types::ArrayElement::Expression(p) => {
                        rename_place_use(p, stacks);
                    }
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            rename_place_use(tag, stacks);
            for attr in props {
                rename_place_use(&mut attr.value, stacks);
            }
            for child in children {
                rename_place_use(child, stacks);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                rename_place_use(child, stacks);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                rename_place_use(sub, stacks);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            rename_place_use(tag, stacks);
            for sub in &mut value.subexpressions {
                rename_place_use(sub, stacks);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => {
            rename_place_use(value, stacks);
        }
        InstructionValue::Await { value } => {
            rename_place_use(value, stacks);
        }
        InstructionValue::GetIterator { collection } => {
            rename_place_use(collection, stacks);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            rename_place_use(iterator, stacks);
        }
        InstructionValue::NextPropertyOf { value } => {
            rename_place_use(value, stacks);
        }
        InstructionValue::TypeCastExpression { value, .. } => {
            rename_place_use(value, stacks);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            rename_place_use(decl, stacks);
            for dep in deps {
                rename_place_use(dep, stacks);
            }
        }
        // No places in these variants
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

/// Rename uses within a Terminal
fn rename_terminal_uses(
    terminal: &mut crate::hir::types::Terminal,
    stacks: &FxHashMap<IdentifierId, Vec<u32>>,
) {
    use crate::hir::types::Terminal;
    match terminal {
        Terminal::If { test, .. }
        | Terminal::Branch { test, .. }
        | Terminal::Ternary { test, .. }
        | Terminal::Optional { test, .. } => {
            rename_place_use(test, stacks);
        }
        Terminal::Switch { test, cases, .. } => {
            rename_place_use(test, stacks);
            for case in cases {
                if let Some(t) = &mut case.test {
                    rename_place_use(t, stacks);
                }
            }
        }
        Terminal::Return { value } | Terminal::Throw { value } => {
            rename_place_use(value, stacks);
        }
        _ => {}
    }
}

#[expect(clippy::too_many_arguments)]
fn rename_block(
    hir: &mut HIR,
    block_id: BlockId,
    dom_tree: &FxHashMap<BlockId, Vec<BlockId>>,
    stacks: &mut FxHashMap<IdentifierId, Vec<u32>>,
    version_counters: &mut FxHashMap<IdentifierId, u32>,
    block_index: &FxHashMap<BlockId, usize>,
    successors: &FxHashMap<BlockId, Vec<BlockId>>,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
) {
    // Track how many versions we push so we can pop them when backtracking
    let mut push_count: FxHashMap<IdentifierId, u32> = FxHashMap::default();

    let idx = match block_index.get(&block_id) {
        Some(&i) => i,
        None => return,
    };

    // 1. Rename phi node definitions: each phi defines a new SSA version
    let num_phis = hir.blocks[idx].1.phis.len();
    for i in 0..num_phis {
        let base_id = hir.blocks[idx].1.phis[i].place.identifier.id;
        let version = fresh_ssa_version(base_id, stacks, version_counters);
        *push_count.entry(base_id).or_insert(0) += 1;
        hir.blocks[idx].1.phis[i].place.identifier.ssa_version = version;
    }

    // 2. For each instruction: rename uses, then rename the def (lvalue)
    let num_instrs = hir.blocks[idx].1.instructions.len();
    for i in 0..num_instrs {
        // Rename uses in the instruction value
        let mut value = std::mem::replace(
            &mut hir.blocks[idx].1.instructions[i].value,
            crate::hir::types::InstructionValue::UnsupportedNode { node: String::new() },
        );
        rename_instruction_value_uses(&mut value, stacks);
        hir.blocks[idx].1.instructions[i].value = value;

        // Rename the definition (lvalue) — set ssa_version instead of replacing id
        let base_id = hir.blocks[idx].1.instructions[i].lvalue.identifier.id;
        let version = fresh_ssa_version(base_id, stacks, version_counters);
        *push_count.entry(base_id).or_insert(0) += 1;
        hir.blocks[idx].1.instructions[i].lvalue.identifier.ssa_version = version;
    }

    // 3. Rename uses in the terminal
    let mut terminal = std::mem::replace(
        &mut hir.blocks[idx].1.terminal,
        crate::hir::types::Terminal::Unreachable,
    );
    rename_terminal_uses(&mut terminal, stacks);
    hir.blocks[idx].1.terminal = terminal;

    // 4. Fill in phi operands in successor blocks.
    //    With stable IDs, all references to the same variable share the same
    //    IdentifierId, so we can match phi variables directly by base ID
    //    instead of searching by declaration_id or name.
    if let Some(succs) = successors.get(&block_id) {
        for &succ_id in succs {
            if let Some(&succ_idx) = block_index.get(&succ_id) {
                let num_succ_phis = hir.blocks[succ_idx].1.phis.len();
                for pi in 0..num_succ_phis {
                    let already_has_operand = hir.blocks[succ_idx].1.phis[pi]
                        .operands
                        .iter()
                        .any(|(bid, _)| *bid == block_id);
                    if already_has_operand {
                        continue;
                    }

                    // The phi's base IdentifierId identifies the variable.
                    // Look up the current SSA version from our stacks.
                    let phi_base_id = hir.blocks[succ_idx].1.phis[pi].place.identifier.id;

                    // Only add an operand if we have a version on the stack
                    // (meaning this variable is defined/visible from this block)
                    if stacks.get(&phi_base_id).is_some_and(|s| !s.is_empty()) {
                        let current_version = current_ssa_version(phi_base_id, stacks);
                        let mut operand_place = hir.blocks[succ_idx].1.phis[pi].place.clone();
                        operand_place.identifier.ssa_version = current_version;
                        hir.blocks[succ_idx].1.phis[pi].operands.push((block_id, operand_place));
                    }
                }
            }
        }
    }

    // 5. Recurse into dominator tree children
    let children: Vec<BlockId> = dom_tree.get(&block_id).cloned().unwrap_or_default();
    for child in children {
        rename_block(
            hir,
            child,
            dom_tree,
            stacks,
            version_counters,
            block_index,
            successors,
            preds,
        );
    }

    // 6. Pop all versions pushed in this block
    for (base_id, count) in &push_count {
        if let Some(stack) = stacks.get_mut(base_id) {
            for _ in 0..*count {
                stack.pop();
            }
        }
    }
}
