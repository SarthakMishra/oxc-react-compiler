use crate::hir::types::{BlockId, HIR, IdentifierId, InstructionValue, Terminal};
use rustc_hash::FxHashSet;

/// Remove instructions whose results are never used.
///
/// Algorithm:
/// 1. Mark all identifiers that are "used" (appear as operands)
/// 2. Remove instructions whose lvalue is not in the used set
///    (except instructions with side effects)
/// 3. Remove unreachable blocks
pub fn dead_code_elimination(hir: &mut HIR) {
    let used = collect_used_identifiers(hir);

    for (_, block) in &mut hir.blocks {
        block.instructions.retain(|instr| {
            // Keep instructions with side effects
            if has_side_effects(&instr.value) {
                return true;
            }
            // Keep instructions whose result is used
            used.contains(&instr.lvalue.identifier.id)
        });
    }

    // Remove unreachable blocks
    remove_unreachable_blocks(hir);
}

/// Collect all IdentifierIds that are used as operands anywhere.
fn collect_used_identifiers(hir: &HIR) -> FxHashSet<IdentifierId> {
    let mut used = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            collect_used_in_instruction_value(&instr.value, &mut used);
        }
        collect_used_in_terminal(&block.terminal, &mut used);
        for phi in &block.phis {
            for (_, operand) in &phi.operands {
                used.insert(operand.identifier.id);
            }
        }
    }

    used
}

fn collect_used_in_instruction_value(value: &InstructionValue, used: &mut FxHashSet<IdentifierId>) {
    let mut add = |place: &crate::hir::types::Place| {
        used.insert(place.identifier.id);
    };

    match value {
        InstructionValue::LoadLocal { place } => add(place),
        InstructionValue::StoreLocal { lvalue, value, .. } => {
            add(lvalue);
            add(value);
        }
        InstructionValue::LoadContext { place } => add(place),
        InstructionValue::StoreContext { lvalue, value } => {
            add(lvalue);
            add(value);
        }
        InstructionValue::DeclareLocal { lvalue, .. } => add(lvalue),
        InstructionValue::DeclareContext { lvalue } => add(lvalue),
        InstructionValue::Destructure { value, lvalue_pattern } => {
            add(value);
            // Mark default value temps as used so DCE doesn't remove them
            collect_default_value_uses(lvalue_pattern, used);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            add(left);
            add(right);
        }
        InstructionValue::UnaryExpression { value, .. } => add(value),
        InstructionValue::PrefixUpdate { lvalue, .. } => add(lvalue),
        InstructionValue::PostfixUpdate { lvalue, .. } => add(lvalue),
        InstructionValue::CallExpression { callee, args, .. } => {
            add(callee);
            for arg in args {
                add(arg);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            add(receiver);
            for arg in args {
                add(arg);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            add(callee);
            for arg in args {
                add(arg);
            }
        }
        InstructionValue::PropertyLoad { object, .. } => add(object),
        InstructionValue::PropertyStore { object, value, .. } => {
            add(object);
            add(value);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            add(object);
            add(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            add(object);
            add(property);
            add(value);
        }
        InstructionValue::PropertyDelete { object, .. } => add(object),
        InstructionValue::ComputedDelete { object, property } => {
            add(object);
            add(property);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                add(&prop.value);
                if let crate::hir::types::ObjectPropertyKey::Computed(p) = &prop.key {
                    add(p);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for el in elements {
                match el {
                    crate::hir::types::ArrayElement::Expression(p)
                    | crate::hir::types::ArrayElement::Spread(p) => add(p),
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            add(tag);
            for attr in props {
                add(&attr.value);
            }
            for child in children {
                add(child);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for c in children {
                add(c);
            }
        }
        InstructionValue::Await { value } => add(value),
        InstructionValue::GetIterator { collection } => add(collection),
        InstructionValue::IteratorNext { iterator, .. } => add(iterator),
        InstructionValue::NextPropertyOf { value } => add(value),
        InstructionValue::TypeCastExpression { value, .. } => add(value),
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            add(tag);
            for sub in &value.subexpressions {
                add(sub);
            }
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            add(decl);
            for d in deps {
                add(d);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                add(sub);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => add(value),
        // Function expressions capture places internally; the captured
        // variables are handled via the HIRFunction body, not directly here.
        InstructionValue::FunctionExpression { .. } | InstructionValue::ObjectMethod { .. } => {}
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

/// Collect IdentifierIds of default value places in a destructure pattern.
fn collect_default_value_uses(
    pattern: &crate::hir::types::DestructurePattern,
    used: &mut FxHashSet<IdentifierId>,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};
    match pattern {
        DestructurePattern::Object { properties, .. } => {
            for prop in properties {
                if let Some(ref default_place) = prop.default_value {
                    used.insert(default_place.identifier.id);
                }
                if let DestructureTarget::Pattern(nested) = &prop.value {
                    collect_default_value_uses(nested, used);
                }
            }
        }
        DestructurePattern::Array { items, .. } => {
            for item in items {
                if let DestructureArrayItem::Value(DestructureTarget::Pattern(nested)) = item {
                    collect_default_value_uses(nested, used);
                }
            }
        }
    }
}

fn collect_used_in_terminal(terminal: &Terminal, used: &mut FxHashSet<IdentifierId>) {
    match terminal {
        Terminal::If { test, .. }
        | Terminal::Branch { test, .. }
        | Terminal::Ternary { test, .. }
        | Terminal::Optional { test, .. } => {
            used.insert(test.identifier.id);
        }
        Terminal::Switch { test, cases, .. } => {
            used.insert(test.identifier.id);
            for case in cases {
                if let Some(t) = &case.test {
                    used.insert(t.identifier.id);
                }
            }
        }
        Terminal::Return { value, .. } | Terminal::Throw { value } => {
            used.insert(value.identifier.id);
        }
        Terminal::Goto { .. }
        | Terminal::For { .. }
        | Terminal::ForOf { .. }
        | Terminal::ForIn { .. }
        | Terminal::DoWhile { .. }
        | Terminal::While { .. }
        | Terminal::Logical { .. }
        | Terminal::Sequence { .. }
        | Terminal::Label { .. }
        | Terminal::MaybeThrow { .. }
        | Terminal::Try { .. }
        | Terminal::Scope { .. }
        | Terminal::PrunedScope { .. }
        | Terminal::Unreachable => {}
    }
}

/// Check if an instruction has side effects (should not be removed even if unused).
fn has_side_effects(value: &InstructionValue) -> bool {
    matches!(
        value,
        InstructionValue::CallExpression { .. }
            | InstructionValue::MethodCall { .. }
            | InstructionValue::NewExpression { .. }
            | InstructionValue::StoreLocal { .. }
            | InstructionValue::StoreContext { .. }
            | InstructionValue::StoreGlobal { .. }
            | InstructionValue::PropertyStore { .. }
            | InstructionValue::ComputedStore { .. }
            | InstructionValue::PropertyDelete { .. }
            | InstructionValue::ComputedDelete { .. }
            | InstructionValue::PrefixUpdate { .. }
            | InstructionValue::PostfixUpdate { .. }
            | InstructionValue::DeclareLocal { .. }
            | InstructionValue::DeclareContext { .. }
            | InstructionValue::Destructure { .. }
            | InstructionValue::Await { .. }
            | InstructionValue::StartMemoize { .. }
            | InstructionValue::FinishMemoize { .. }
            | InstructionValue::UnsupportedNode { .. }
    )
}

/// Remove blocks that are not reachable from the entry block.
fn remove_unreachable_blocks(hir: &mut HIR) {
    let reachable = find_reachable_blocks(hir);
    hir.blocks.retain(|(id, _)| reachable.contains(id));
}

fn find_reachable_blocks(hir: &HIR) -> FxHashSet<BlockId> {
    let mut reachable = FxHashSet::default();
    let mut worklist = vec![hir.entry];

    while let Some(bid) = worklist.pop() {
        if !reachable.insert(bid) {
            continue;
        }
        if let Some((_, block)) = hir.blocks.iter().find(|(id, _)| *id == bid) {
            for succ in terminal_successors(&block.terminal) {
                if !reachable.contains(&succ) {
                    worklist.push(succ);
                }
            }
        }
    }

    reachable
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
        Terminal::MaybeThrow { continuation, handler, .. } => vec![*continuation, *handler],
        Terminal::Try { block, handler, fallthrough } => vec![*block, *handler, *fallthrough],
        Terminal::Scope { block, fallthrough, .. }
        | Terminal::PrunedScope { block, fallthrough, .. } => vec![*block, *fallthrough],
    }
}
