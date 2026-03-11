#![allow(dead_code)]

use crate::hir::types::{BlockId, IdentifierId, InstructionId, Phi, Place, HIR};
use rustc_hash::{FxHashMap, FxHashSet};

/// Enter SSA form: insert phi nodes and rename identifiers.
///
/// Standard algorithm (Cytron et al.):
/// 1. Compute dominance frontiers
/// 2. Insert phi nodes at dominance frontiers for each variable
/// 3. Rename identifiers using dominator tree walk
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

    // Step 4: Rename variables (SSA numbering)
    let mut next_id = find_max_identifier_id(hir) + 1;
    rename_variables(hir, entry, &dom_tree, &preds, &mut next_id);
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
        Terminal::If {
            consequent,
            alternate,
            fallthrough,
            ..
        } => vec![*consequent, *alternate, *fallthrough],
        Terminal::Branch {
            consequent,
            alternate,
            ..
        } => vec![*consequent, *alternate],
        Terminal::Switch {
            cases, fallthrough, ..
        } => {
            let mut succs: Vec<BlockId> = cases.iter().map(|c| c.block).collect();
            succs.push(*fallthrough);
            succs
        }
        Terminal::Return { .. } | Terminal::Throw { .. } | Terminal::Unreachable => vec![],
        Terminal::For {
            init,
            test,
            update,
            body,
            fallthrough,
        } => {
            let mut succs = vec![*init, *test, *body, *fallthrough];
            if let Some(u) = update {
                succs.push(*u);
            }
            succs
        }
        Terminal::ForOf {
            init,
            test,
            body,
            fallthrough,
        }
        | Terminal::ForIn {
            init,
            test,
            body,
            fallthrough,
        } => vec![*init, *test, *body, *fallthrough],
        Terminal::DoWhile {
            body,
            test,
            fallthrough,
        } => vec![*body, *test, *fallthrough],
        Terminal::While {
            test,
            body,
            fallthrough,
        } => vec![*test, *body, *fallthrough],
        Terminal::Logical {
            left,
            right,
            fallthrough,
            ..
        } => vec![*left, *right, *fallthrough],
        Terminal::Ternary {
            consequent,
            alternate,
            fallthrough,
            ..
        } => vec![*consequent, *alternate, *fallthrough],
        Terminal::Optional {
            consequent,
            fallthrough,
            ..
        } => vec![*consequent, *fallthrough],
        Terminal::Sequence {
            blocks,
            fallthrough,
        } => {
            let mut succs = blocks.clone();
            succs.push(*fallthrough);
            succs
        }
        Terminal::Label {
            block, fallthrough, ..
        } => vec![*block, *fallthrough],
        Terminal::MaybeThrow {
            continuation,
            handler,
        } => vec![*continuation, *handler],
        Terminal::Try {
            block,
            handler,
            fallthrough,
        } => vec![*block, *handler, *fallthrough],
        Terminal::Scope {
            block, fallthrough, ..
        }
        | Terminal::PrunedScope {
            block, fallthrough, ..
        } => vec![*block, *fallthrough],
    }
}

/// Compute immediate dominators using the iterative algorithm (Cooper, Harvey, Kennedy).
/// Returns a map from block -> immediate dominator.
fn compute_dominators(
    block_ids: &[BlockId],
    entry: BlockId,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
) -> FxHashMap<BlockId, BlockId> {
    // Map block ID to index for efficient intersection
    let id_to_idx: FxHashMap<BlockId, usize> = block_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();
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
            defs.entry(instr.lvalue.identifier.id)
                .or_default()
                .insert(*block_id);
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

/// Find the maximum IdentifierId in the HIR for renaming
fn find_max_identifier_id(hir: &HIR) -> u32 {
    let mut max = 0u32;
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            max = max.max(instr.lvalue.identifier.id.0);
            collect_max_id_from_value(&instr.value, &mut max);
        }
        for phi in &block.phis {
            max = max.max(phi.place.identifier.id.0);
            for (_, op) in &phi.operands {
                max = max.max(op.identifier.id.0);
            }
        }
        collect_max_id_from_terminal(&block.terminal, &mut max);
    }
    max
}

fn collect_max_id_from_place(place: &Place, max: &mut u32) {
    *max = (*max).max(place.identifier.id.0);
}

fn collect_max_id_from_value(value: &crate::hir::types::InstructionValue, max: &mut u32) {
    use crate::hir::types::InstructionValue;
    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            collect_max_id_from_place(place, max);
        }
        InstructionValue::StoreLocal { lvalue, value, .. } => {
            collect_max_id_from_place(lvalue, max);
            collect_max_id_from_place(value, max);
        }
        InstructionValue::StoreContext { lvalue, value } => {
            collect_max_id_from_place(lvalue, max);
            collect_max_id_from_place(value, max);
        }
        InstructionValue::DeclareLocal { lvalue, .. } => {
            collect_max_id_from_place(lvalue, max);
        }
        InstructionValue::DeclareContext { lvalue } => {
            collect_max_id_from_place(lvalue, max);
        }
        InstructionValue::Destructure { value, .. } => {
            collect_max_id_from_place(value, max);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            collect_max_id_from_place(left, max);
            collect_max_id_from_place(right, max);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            collect_max_id_from_place(value, max);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            collect_max_id_from_place(lvalue, max);
        }
        InstructionValue::CallExpression { callee, args } => {
            collect_max_id_from_place(callee, max);
            for arg in args {
                collect_max_id_from_place(arg, max);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            collect_max_id_from_place(receiver, max);
            for arg in args {
                collect_max_id_from_place(arg, max);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            collect_max_id_from_place(callee, max);
            for arg in args {
                collect_max_id_from_place(arg, max);
            }
        }
        InstructionValue::PropertyLoad { object, .. } => {
            collect_max_id_from_place(object, max);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            collect_max_id_from_place(object, max);
            collect_max_id_from_place(value, max);
        }
        InstructionValue::ComputedLoad { object, property } => {
            collect_max_id_from_place(object, max);
            collect_max_id_from_place(property, max);
        }
        InstructionValue::ComputedStore {
            object,
            property,
            value,
        } => {
            collect_max_id_from_place(object, max);
            collect_max_id_from_place(property, max);
            collect_max_id_from_place(value, max);
        }
        InstructionValue::PropertyDelete { object, .. } => {
            collect_max_id_from_place(object, max);
        }
        InstructionValue::ComputedDelete { object, property } => {
            collect_max_id_from_place(object, max);
            collect_max_id_from_place(property, max);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                collect_max_id_from_place(&prop.value, max);
                if let crate::hir::types::ObjectPropertyKey::Computed(p) = &prop.key {
                    collect_max_id_from_place(p, max);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Spread(p)
                    | crate::hir::types::ArrayElement::Expression(p) => {
                        collect_max_id_from_place(p, max);
                    }
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression {
            tag,
            props,
            children,
        } => {
            collect_max_id_from_place(tag, max);
            for attr in props {
                collect_max_id_from_place(&attr.value, max);
            }
            for child in children {
                collect_max_id_from_place(child, max);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                collect_max_id_from_place(child, max);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                collect_max_id_from_place(sub, max);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            collect_max_id_from_place(tag, max);
            for sub in &value.subexpressions {
                collect_max_id_from_place(sub, max);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => {
            collect_max_id_from_place(value, max);
        }
        InstructionValue::Await { value } => {
            collect_max_id_from_place(value, max);
        }
        InstructionValue::GetIterator { collection } => {
            collect_max_id_from_place(collection, max);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            collect_max_id_from_place(iterator, max);
        }
        InstructionValue::NextPropertyOf { value } => {
            collect_max_id_from_place(value, max);
        }
        InstructionValue::TypeCastExpression { value, .. } => {
            collect_max_id_from_place(value, max);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            collect_max_id_from_place(decl, max);
            for dep in deps {
                collect_max_id_from_place(dep, max);
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

fn collect_max_id_from_terminal(terminal: &crate::hir::types::Terminal, max: &mut u32) {
    use crate::hir::types::Terminal;
    match terminal {
        Terminal::If { test, .. }
        | Terminal::Branch { test, .. }
        | Terminal::Ternary { test, .. }
        | Terminal::Optional { test, .. } => {
            collect_max_id_from_place(test, max);
        }
        Terminal::Switch { test, cases, .. } => {
            collect_max_id_from_place(test, max);
            for case in cases {
                if let Some(t) = &case.test {
                    collect_max_id_from_place(t, max);
                }
            }
        }
        Terminal::Return { value } | Terminal::Throw { value } => {
            collect_max_id_from_place(value, max);
        }
        _ => {}
    }
}

/// Rename variables to SSA form by walking the dominator tree
fn rename_variables(
    hir: &mut HIR,
    entry: BlockId,
    dom_tree: &FxHashMap<BlockId, Vec<BlockId>>,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
    next_id: &mut u32,
) {
    // Map from original IdentifierId -> stack of SSA-renamed IdentifierIds.
    // The top of the stack is the "current" SSA name for that variable.
    let mut stacks: FxHashMap<IdentifierId, Vec<IdentifierId>> = FxHashMap::default();

    // We need the block ordering (successor info) to fill phi operands.
    // Build a quick lookup: block_id -> index in hir.blocks
    let block_index: FxHashMap<BlockId, usize> = hir
        .blocks
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i))
        .collect();

    // Precompute successors per block
    let successors: FxHashMap<BlockId, Vec<BlockId>> = hir
        .blocks
        .iter()
        .map(|(id, block)| (*id, terminal_successors(&block.terminal)))
        .collect();

    rename_block(
        hir,
        entry,
        dom_tree,
        &mut stacks,
        next_id,
        &block_index,
        &successors,
        preds,
    );
}

fn fresh_ssa_name(
    original: IdentifierId,
    stacks: &mut FxHashMap<IdentifierId, Vec<IdentifierId>>,
    next_id: &mut u32,
) -> IdentifierId {
    let new_id = IdentifierId(*next_id);
    *next_id += 1;
    stacks.entry(original).or_default().push(new_id);
    new_id
}

fn current_ssa_name(
    original: IdentifierId,
    stacks: &FxHashMap<IdentifierId, Vec<IdentifierId>>,
) -> IdentifierId {
    stacks
        .get(&original)
        .and_then(|stack| stack.last().copied())
        .unwrap_or(original)
}

fn rename_place_use(place: &mut Place, stacks: &FxHashMap<IdentifierId, Vec<IdentifierId>>) {
    let original = place.identifier.id;
    place.identifier.id = current_ssa_name(original, stacks);
}

fn rename_place_def(
    place: &mut Place,
    stacks: &mut FxHashMap<IdentifierId, Vec<IdentifierId>>,
    next_id: &mut u32,
    push_count: &mut FxHashMap<IdentifierId, u32>,
) {
    let original = place.identifier.id;
    let new_id = fresh_ssa_name(original, stacks, next_id);
    *push_count.entry(original).or_insert(0) += 1;
    place.identifier.id = new_id;
}

/// Rename uses within an InstructionValue (all operand positions)
fn rename_instruction_value_uses(
    value: &mut crate::hir::types::InstructionValue,
    stacks: &FxHashMap<IdentifierId, Vec<IdentifierId>>,
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
        InstructionValue::CallExpression { callee, args } => {
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
        InstructionValue::ComputedLoad { object, property } => {
            rename_place_use(object, stacks);
            rename_place_use(property, stacks);
        }
        InstructionValue::ComputedStore {
            object,
            property,
            value,
        } => {
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
        InstructionValue::JsxExpression {
            tag,
            props,
            children,
        } => {
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
    stacks: &FxHashMap<IdentifierId, Vec<IdentifierId>>,
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

#[allow(clippy::too_many_arguments)]
fn rename_block(
    hir: &mut HIR,
    block_id: BlockId,
    dom_tree: &FxHashMap<BlockId, Vec<BlockId>>,
    stacks: &mut FxHashMap<IdentifierId, Vec<IdentifierId>>,
    next_id: &mut u32,
    block_index: &FxHashMap<BlockId, usize>,
    successors: &FxHashMap<BlockId, Vec<BlockId>>,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
) {
    // Track how many names we push so we can pop them when backtracking
    let mut push_count: FxHashMap<IdentifierId, u32> = FxHashMap::default();

    let idx = match block_index.get(&block_id) {
        Some(&i) => i,
        None => return,
    };

    // 1. Rename phi node definitions: each phi defines a new SSA name
    let num_phis = hir.blocks[idx].1.phis.len();
    for i in 0..num_phis {
        let original = hir.blocks[idx].1.phis[i].place.identifier.id;
        let new_id = fresh_ssa_name(original, stacks, next_id);
        *push_count.entry(original).or_insert(0) += 1;
        hir.blocks[idx].1.phis[i].place.identifier.id = new_id;
    }

    // 2. For each instruction: rename uses, then rename the def (lvalue)
    let num_instrs = hir.blocks[idx].1.instructions.len();
    for i in 0..num_instrs {
        // Rename uses in the instruction value
        // We need to temporarily take the value out to avoid borrow issues
        let mut value = std::mem::replace(
            &mut hir.blocks[idx].1.instructions[i].value,
            crate::hir::types::InstructionValue::UnsupportedNode {
                node: String::new(),
            },
        );
        rename_instruction_value_uses(&mut value, stacks);
        hir.blocks[idx].1.instructions[i].value = value;

        // Rename the definition (lvalue)
        let original = hir.blocks[idx].1.instructions[i].lvalue.identifier.id;
        let new_id = fresh_ssa_name(original, stacks, next_id);
        *push_count.entry(original).or_insert(0) += 1;
        hir.blocks[idx].1.instructions[i].lvalue.identifier.id = new_id;
    }

    // 3. Rename uses in the terminal
    let mut terminal = std::mem::replace(
        &mut hir.blocks[idx].1.terminal,
        crate::hir::types::Terminal::Unreachable,
    );
    rename_terminal_uses(&mut terminal, stacks);
    hir.blocks[idx].1.terminal = terminal;

    // 4. Fill in phi operands in successor blocks
    if let Some(succs) = successors.get(&block_id) {
        for &succ_id in succs {
            if let Some(&succ_idx) = block_index.get(&succ_id) {
                let num_succ_phis = hir.blocks[succ_idx].1.phis.len();
                for pi in 0..num_succ_phis {
                    // The phi's original variable is encoded in its place.
                    // We need to find what the current SSA name is for that variable
                    // from the perspective of block_id.
                    //
                    // The phi.place.identifier.id has already been renamed to a new SSA name.
                    // We need the *original* variable id to look up the current name.
                    // Since we renamed it, we look at the declaration_id or use a reverse map.
                    //
                    // Strategy: we use the phi's declaration_id to identify the original
                    // variable, or we look at the existing operands to determine the original.
                    // Simplest: the original variable is whatever was on the stack before
                    // we renamed it. We can detect it by looking at what the phi was
                    // for — we stored the original place during insert_phi_nodes.
                    //
                    // The phi operands tell us which block contributes which value.
                    // We check if this block_id already has an operand; if not, we add one.
                    let already_has_operand = hir.blocks[succ_idx].1.phis[pi]
                        .operands
                        .iter()
                        .any(|(bid, _)| *bid == block_id);
                    if already_has_operand {
                        continue;
                    }

                    // Find the original variable for this phi. We need to figure out
                    // what variable this phi node is for. We look at the phi's
                    // declaration_id as a stable identifier across SSA renames.
                    let phi_decl_id = hir.blocks[succ_idx].1.phis[pi]
                        .place
                        .identifier
                        .declaration_id;
                    let phi_name = hir.blocks[succ_idx].1.phis[pi]
                        .place
                        .identifier
                        .name
                        .clone();

                    // Find the original identifier id by searching stacks for a match
                    // based on declaration_id
                    let mut matched_original: Option<IdentifierId> = None;
                    for (&orig_id, stack) in stacks.iter() {
                        if !stack.is_empty() {
                            // Check if this is the right variable by comparing
                            // declaration IDs via the HIR
                            if let Some(orig_decl) = find_declaration_id_for_var(hir, orig_id) {
                                if orig_decl == phi_decl_id {
                                    matched_original = Some(orig_id);
                                    break;
                                }
                            }
                            // Fallback: match by name
                            if matched_original.is_none() {
                                if let Some(orig_name) = find_name_for_var(hir, orig_id) {
                                    if Some(&orig_name) == phi_name.as_ref() {
                                        matched_original = Some(orig_id);
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    // If we found a match, add the current SSA name as operand
                    if let Some(orig_id) = matched_original {
                        let current = current_ssa_name(orig_id, stacks);
                        let mut operand_place = hir.blocks[succ_idx].1.phis[pi].place.clone();
                        operand_place.identifier.id = current;
                        hir.blocks[succ_idx].1.phis[pi]
                            .operands
                            .push((block_id, operand_place));
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
            next_id,
            block_index,
            successors,
            preds,
        );
    }

    // 6. Pop all names pushed in this block
    for (orig_id, count) in &push_count {
        if let Some(stack) = stacks.get_mut(orig_id) {
            for _ in 0..*count {
                stack.pop();
            }
        }
    }
}

/// Find the declaration_id for a given IdentifierId in the HIR
fn find_declaration_id_for_var(
    hir: &HIR,
    var_id: IdentifierId,
) -> Option<Option<crate::hir::types::DeclarationId>> {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id == var_id {
                return Some(instr.lvalue.identifier.declaration_id);
            }
        }
    }
    None
}

/// Find the name for a given IdentifierId in the HIR
fn find_name_for_var(hir: &HIR, var_id: IdentifierId) -> Option<String> {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id == var_id {
                return instr.lvalue.identifier.name.clone();
            }
        }
    }
    None
}
