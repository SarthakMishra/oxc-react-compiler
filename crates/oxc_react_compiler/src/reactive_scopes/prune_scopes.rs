#![allow(dead_code)]

use crate::hir::types::{
    ArrayElement, BasicBlock, BlockId, BlockKind, HIR, IdentifierId, InstructionKind,
    InstructionValue, ObjectPropertyKey, Param, Place, ReactiveBlock, ReactiveFunction,
    ReactiveInstruction, ReactiveScope, ReactiveTerminal, ScopeId, Terminal,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// Prune reactive scopes that don't escape the function.
///
/// A scope "escapes" if any of its declarations or reassignments are consumed
/// by instructions anywhere in the function. Scopes whose results are never
/// referenced can be unwrapped (their instructions inlined into the parent
/// block).
pub fn prune_non_escaping_scopes(rf: &mut ReactiveFunction) {
    // Collect all identifier IDs used anywhere in the function.
    //
    // NOTE: Previously we only collected uses OUTSIDE scope blocks (passing
    // `in_scope=true` when recursing into scopes). This was a bug: a variable
    // declared in scope S1 and used inside scope S2 IS escaping S1. Both
    // scopes are independent cache boundaries. The `in_scope` flag incorrectly
    // treated uses inside ANY scope block as "not escaping", causing derived
    // computations (like `const doubled = value * 2`) to be pruned and emitted
    // outside their scope guard, defeating memoization. The fix matches
    // upstream's `PruneNonEscapingScopes.ts` which collects all references
    // without an in-scope gate.
    let mut used_ids = FxHashSet::default();
    collect_used_ids(&rf.body, &mut used_ids);

    // Collect IDs that are ONLY used as condition tests (if/switch/ternary test
    // positions) and never as values. Per upstream's PruneNonEscapingScopes.ts,
    // a scope whose declarations are only used as condition tests does not
    // "escape" — the test discards the value (only truthiness matters).
    // DIVERGENCE: upstream tracks this at the type level; we approximate by
    // collecting test-position IDs and subtracting value-position IDs, then
    // propagating through alias chains (StoreLocal/LoadLocal).
    let mut test_only_ids = FxHashSet::default();
    collect_test_position_ids(&rf.body, &mut test_only_ids);
    let mut value_used_ids = FxHashSet::default();
    collect_value_used_ids(&rf.body, &mut value_used_ids);
    // test_only = appears in test position AND never in value position
    test_only_ids.retain(|id| !value_used_ids.contains(id));

    // Propagate test-only status through alias chains: if `const x = t0` and x
    // is test-only, and t0 is only used in this store, then t0 is also test-only.
    let mut alias_info: FxHashMap<IdentifierId, Vec<IdentifierId>> = FxHashMap::default();
    let mut use_counts: FxHashMap<IdentifierId, usize> = FxHashMap::default();
    collect_alias_info(&rf.body, &mut alias_info, &mut use_counts);
    // Fixed-point propagation
    loop {
        let mut changed = false;
        for (value_id, target_ids) in &alias_info {
            if test_only_ids.contains(value_id) {
                continue; // already test-only
            }
            // Check if ALL targets are test-only and this value is only used
            // in stores to those targets (use_count == number of store targets)
            let store_count = target_ids.len();
            let total_uses = use_counts.get(value_id).copied().unwrap_or(0);
            let all_targets_test_only = target_ids.iter().all(|t| test_only_ids.contains(t));
            if all_targets_test_only && total_uses == store_count {
                test_only_ids.insert(*value_id);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // Remove scopes whose declarations are never used anywhere (or only as tests)
    prune_scopes_in_block(&mut rf.body, &used_ids, &test_only_ids);
}

/// Collect all identifier IDs referenced as operands anywhere in the tree.
fn collect_used_ids(block: &ReactiveBlock, used: &mut FxHashSet<IdentifierId>) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                collect_instruction_operand_ids(&instruction.value, used);
            }
            ReactiveInstruction::Scope(scope_block) => {
                collect_used_ids(&scope_block.instructions, used);
            }
            ReactiveInstruction::Terminal(terminal) => {
                collect_used_in_terminal(terminal, used);
            }
        }
    }
}

fn collect_instruction_operand_ids(value: &InstructionValue, used: &mut FxHashSet<IdentifierId>) {
    fn insert_place(place: &Place, used: &mut FxHashSet<IdentifierId>) {
        used.insert(place.identifier.id);
    }

    match value {
        // Locals & context
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            insert_place(place, used);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            insert_place(lvalue, used);
            insert_place(value, used);
        }
        InstructionValue::DeclareLocal { lvalue, .. }
        | InstructionValue::DeclareContext { lvalue } => {
            insert_place(lvalue, used);
        }
        InstructionValue::Destructure { value, .. } => {
            insert_place(value, used);
        }

        // Literals — no operands
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}

        // Templates
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                insert_place(sub, used);
            }
        }

        // Operators
        InstructionValue::BinaryExpression { left, right, .. } => {
            insert_place(left, used);
            insert_place(right, used);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            insert_place(value, used);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            insert_place(lvalue, used);
        }

        // Calls
        InstructionValue::CallExpression { callee, args, .. }
        | InstructionValue::NewExpression { callee, args } => {
            insert_place(callee, used);
            for arg in args {
                insert_place(arg, used);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            insert_place(receiver, used);
            for arg in args {
                insert_place(arg, used);
            }
        }

        // Property access
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            insert_place(object, used);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            insert_place(object, used);
            insert_place(value, used);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            insert_place(object, used);
            insert_place(property, used);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            insert_place(object, used);
            insert_place(property, used);
            insert_place(value, used);
        }
        InstructionValue::ComputedDelete { object, property } => {
            insert_place(object, used);
            insert_place(property, used);
        }

        // Containers
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                insert_place(&prop.value, used);
                if let ObjectPropertyKey::Computed(key) = &prop.key {
                    insert_place(key, used);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    ArrayElement::Expression(p) | ArrayElement::Spread(p) => {
                        insert_place(p, used);
                    }
                    ArrayElement::Hole => {}
                }
            }
        }

        // JSX
        InstructionValue::JsxExpression { tag, props, children } => {
            insert_place(tag, used);
            for attr in props {
                insert_place(&attr.value, used);
            }
            for child in children {
                insert_place(child, used);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                insert_place(child, used);
            }
        }

        // Functions — no direct operands (lowered_func is self-contained)
        InstructionValue::FunctionExpression { .. } | InstructionValue::ObjectMethod { .. } => {}

        // Globals
        InstructionValue::StoreGlobal { value, .. } => {
            insert_place(value, used);
        }

        // Async/Iterator
        InstructionValue::Await { value }
        | InstructionValue::GetIterator { collection: value }
        | InstructionValue::IteratorNext { iterator: value, .. }
        | InstructionValue::NextPropertyOf { value } => {
            insert_place(value, used);
        }

        // Type
        InstructionValue::TypeCastExpression { value, .. } => {
            insert_place(value, used);
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            insert_place(tag, used);
            for sub in &value.subexpressions {
                insert_place(sub, used);
            }
        }

        // Manual memoization
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            insert_place(decl, used);
            for dep in deps {
                insert_place(dep, used);
            }
        }
    }
}

fn collect_used_in_terminal(terminal: &ReactiveTerminal, used: &mut FxHashSet<IdentifierId>) {
    match terminal {
        ReactiveTerminal::Return { value, .. } | ReactiveTerminal::Throw { value, .. } => {
            used.insert(value.identifier.id);
        }
        ReactiveTerminal::If { test, consequent, alternate, .. } => {
            used.insert(test.identifier.id);
            collect_used_ids(consequent, used);
            collect_used_ids(alternate, used);
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            used.insert(test.identifier.id);
            for (_, block) in cases {
                collect_used_ids(block, used);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            collect_used_ids(init, used);
            collect_used_ids(test, used);
            if let Some(upd) = update {
                collect_used_ids(upd, used);
            }
            collect_used_ids(body, used);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            collect_used_ids(init, used);
            collect_used_ids(test, used);
            collect_used_ids(body, used);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            collect_used_ids(test, used);
            collect_used_ids(body, used);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            collect_used_ids(block, used);
            collect_used_ids(handler, used);
        }
        ReactiveTerminal::Label { block, .. } => {
            collect_used_ids(block, used);
        }
        ReactiveTerminal::Logical { right, result, .. } => {
            collect_used_ids(right, used);
            if let Some(r) = result {
                used.insert(r.identifier.id);
            }
        }
        ReactiveTerminal::Continue { .. } | ReactiveTerminal::Break { .. } => {}
    }
}

/// Collect IDs that appear in condition-test positions (if test, switch test,
/// conditional/ternary test). These positions only evaluate truthiness — the
/// value itself does not escape.
fn collect_test_position_ids(block: &ReactiveBlock, test_ids: &mut FxHashSet<IdentifierId>) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Scope(scope_block) => {
                collect_test_position_ids(&scope_block.instructions, test_ids);
            }
            ReactiveInstruction::Terminal(terminal) => {
                collect_test_ids_in_terminal(terminal, test_ids);
            }
            ReactiveInstruction::Instruction(_) => {}
        }
    }
}

fn collect_test_ids_in_terminal(
    terminal: &ReactiveTerminal,
    test_ids: &mut FxHashSet<IdentifierId>,
) {
    match terminal {
        ReactiveTerminal::If { test, consequent, alternate, .. } => {
            test_ids.insert(test.identifier.id);
            collect_test_position_ids(consequent, test_ids);
            collect_test_position_ids(alternate, test_ids);
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            test_ids.insert(test.identifier.id);
            for (_, block) in cases {
                collect_test_position_ids(block, test_ids);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            collect_test_position_ids(init, test_ids);
            collect_test_position_ids(test, test_ids);
            if let Some(upd) = update {
                collect_test_position_ids(upd, test_ids);
            }
            collect_test_position_ids(body, test_ids);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            collect_test_position_ids(init, test_ids);
            collect_test_position_ids(test, test_ids);
            collect_test_position_ids(body, test_ids);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            collect_test_position_ids(test, test_ids);
            collect_test_position_ids(body, test_ids);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            collect_test_position_ids(block, test_ids);
            collect_test_position_ids(handler, test_ids);
        }
        ReactiveTerminal::Label { block, .. } => {
            collect_test_position_ids(block, test_ids);
        }
        ReactiveTerminal::Logical { right, .. } => {
            collect_test_position_ids(right, test_ids);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
    }
}

/// Collect alias information for propagating test-only status through
/// StoreLocal/DeclareLocal chains. For `StoreLocal { lvalue: x, value: t0 }`,
/// records that t0 aliases to x. Also counts total uses of each ID.
fn collect_alias_info(
    block: &ReactiveBlock,
    aliases: &mut FxHashMap<IdentifierId, Vec<IdentifierId>>,
    use_counts: &mut FxHashMap<IdentifierId, usize>,
) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                // Track alias chains through StoreLocal and LoadLocal
                match &instruction.value {
                    InstructionValue::StoreLocal { lvalue, value, .. } => {
                        // source (value) is stored into target (lvalue)
                        aliases.entry(value.identifier.id).or_default().push(lvalue.identifier.id);
                    }
                    InstructionValue::LoadLocal { place } => {
                        // source (place) is loaded into target (instruction.lvalue)
                        aliases
                            .entry(place.identifier.id)
                            .or_default()
                            .push(instruction.lvalue.identifier.id);
                    }
                    _ => {}
                }
                // Count READ uses of each ID (excludes write targets)
                collect_read_use_counts(&instruction.value, use_counts);
            }
            ReactiveInstruction::Scope(scope_block) => {
                collect_alias_info(&scope_block.instructions, aliases, use_counts);
            }
            ReactiveInstruction::Terminal(terminal) => {
                collect_alias_info_in_terminal(terminal, aliases, use_counts);
            }
        }
    }
}

fn collect_alias_info_in_terminal(
    terminal: &ReactiveTerminal,
    aliases: &mut FxHashMap<IdentifierId, Vec<IdentifierId>>,
    use_counts: &mut FxHashMap<IdentifierId, usize>,
) {
    match terminal {
        ReactiveTerminal::If { test, consequent, alternate, .. } => {
            *use_counts.entry(test.identifier.id).or_insert(0) += 1;
            collect_alias_info(consequent, aliases, use_counts);
            collect_alias_info(alternate, aliases, use_counts);
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            *use_counts.entry(test.identifier.id).or_insert(0) += 1;
            for (_, block) in cases {
                collect_alias_info(block, aliases, use_counts);
            }
        }
        ReactiveTerminal::Return { value, .. } | ReactiveTerminal::Throw { value, .. } => {
            *use_counts.entry(value.identifier.id).or_insert(0) += 1;
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            collect_alias_info(init, aliases, use_counts);
            collect_alias_info(test, aliases, use_counts);
            if let Some(upd) = update {
                collect_alias_info(upd, aliases, use_counts);
            }
            collect_alias_info(body, aliases, use_counts);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            collect_alias_info(init, aliases, use_counts);
            collect_alias_info(test, aliases, use_counts);
            collect_alias_info(body, aliases, use_counts);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            collect_alias_info(test, aliases, use_counts);
            collect_alias_info(body, aliases, use_counts);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            collect_alias_info(block, aliases, use_counts);
            collect_alias_info(handler, aliases, use_counts);
        }
        ReactiveTerminal::Label { block, .. } => {
            collect_alias_info(block, aliases, use_counts);
        }
        ReactiveTerminal::Logical { right, result, .. } => {
            collect_alias_info(right, aliases, use_counts);
            if let Some(r) = result {
                *use_counts.entry(r.identifier.id).or_insert(0) += 1;
            }
        }
        ReactiveTerminal::Continue { .. } | ReactiveTerminal::Break { .. } => {}
    }
}

/// Count READ uses of each IdentifierId in an instruction (excludes write
/// targets like StoreLocal lvalue). Each operand occurrence is counted
/// separately (e.g. `x + x` counts `x` twice).
fn collect_read_use_counts(value: &InstructionValue, counts: &mut FxHashMap<IdentifierId, usize>) {
    // Collect read operand IDs, then count each occurrence
    let mut temp = FxHashSet::default();
    collect_read_operand_ids(value, &mut temp);
    for id in temp {
        *counts.entry(id).or_insert(0) += 1;
    }
}

/// Collect IDs used as values (NOT as condition tests). This includes:
/// - Instruction READ operands (excluding write targets like StoreLocal lvalue)
/// - Return/Throw values
/// - Terminal body blocks (recursively)
///
/// But EXCLUDES condition test positions in If/Switch terminals.
fn collect_value_used_ids(block: &ReactiveBlock, used: &mut FxHashSet<IdentifierId>) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                // Use read-only operand collection (excludes write targets)
                collect_read_operand_ids(&instruction.value, used);
            }
            ReactiveInstruction::Scope(scope_block) => {
                collect_value_used_ids(&scope_block.instructions, used);
            }
            ReactiveInstruction::Terminal(terminal) => {
                collect_value_used_in_terminal(terminal, used);
            }
        }
    }
}

/// Like `collect_instruction_operand_ids` but only collects READ operands.
/// Excludes write-target positions like StoreLocal lvalue, DeclareLocal lvalue.
fn collect_read_operand_ids(value: &InstructionValue, used: &mut FxHashSet<IdentifierId>) {
    fn add(place: &Place, used: &mut FxHashSet<IdentifierId>) {
        used.insert(place.identifier.id);
    }

    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            add(place, used);
        }
        InstructionValue::StoreLocal { value, .. }
        | InstructionValue::StoreContext { value, .. } => {
            // Only the VALUE is a read; lvalue is a write target
            add(value, used);
        }
        InstructionValue::DeclareLocal { .. } | InstructionValue::DeclareContext { .. } => {
            // lvalue is a write target, not a read
        }
        InstructionValue::Destructure { value, .. } => {
            add(value, used);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            add(left, used);
            add(right, used);
        }
        InstructionValue::UnaryExpression { value, .. } => add(value, used),
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            add(lvalue, used); // lvalue is both read and written
        }
        InstructionValue::CallExpression { callee, args, .. }
        | InstructionValue::NewExpression { callee, args } => {
            add(callee, used);
            for arg in args {
                add(arg, used);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            add(receiver, used);
            for arg in args {
                add(arg, used);
            }
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            add(object, used);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            add(object, used);
            add(value, used);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            add(object, used);
            add(property, used);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            add(object, used);
            add(property, used);
            add(value, used);
        }
        InstructionValue::ComputedDelete { object, property } => {
            add(object, used);
            add(property, used);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                add(&prop.value, used);
                if let ObjectPropertyKey::Computed(key) = &prop.key {
                    add(key, used);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    ArrayElement::Expression(p) | ArrayElement::Spread(p) => add(p, used),
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            add(tag, used);
            for attr in props {
                add(&attr.value, used);
            }
            for child in children {
                add(child, used);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                add(child, used);
            }
        }
        InstructionValue::Await { value }
        | InstructionValue::GetIterator { collection: value }
        | InstructionValue::IteratorNext { iterator: value, .. }
        | InstructionValue::NextPropertyOf { value }
        | InstructionValue::TypeCastExpression { value, .. } => {
            add(value, used);
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                add(sub, used);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            add(tag, used);
            for sub in &value.subexpressions {
                add(sub, used);
            }
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            add(decl, used);
            for dep in deps {
                add(dep, used);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => add(value, used),
        InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. }
        | InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

/// Like `collect_used_in_terminal` but EXCLUDES condition test positions.
fn collect_value_used_in_terminal(terminal: &ReactiveTerminal, used: &mut FxHashSet<IdentifierId>) {
    match terminal {
        ReactiveTerminal::Return { value, .. } | ReactiveTerminal::Throw { value, .. } => {
            used.insert(value.identifier.id);
        }
        ReactiveTerminal::If { consequent, alternate, .. } => {
            // NOTE: test is intentionally NOT inserted — it's a condition test position
            collect_value_used_ids(consequent, used);
            collect_value_used_ids(alternate, used);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            // NOTE: test is intentionally NOT inserted
            for (_, block) in cases {
                collect_value_used_ids(block, used);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            collect_value_used_ids(init, used);
            collect_value_used_ids(test, used);
            if let Some(upd) = update {
                collect_value_used_ids(upd, used);
            }
            collect_value_used_ids(body, used);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            collect_value_used_ids(init, used);
            collect_value_used_ids(test, used);
            collect_value_used_ids(body, used);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            collect_value_used_ids(test, used);
            collect_value_used_ids(body, used);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            collect_value_used_ids(block, used);
            collect_value_used_ids(handler, used);
        }
        ReactiveTerminal::Label { block, .. } => {
            collect_value_used_ids(block, used);
        }
        ReactiveTerminal::Logical { right, result, .. } => {
            collect_value_used_ids(right, used);
            if let Some(r) = result {
                used.insert(r.identifier.id);
            }
        }
        ReactiveTerminal::Continue { .. } | ReactiveTerminal::Break { .. } => {}
    }
}

fn prune_scopes_in_block(
    block: &mut ReactiveBlock,
    used_ids: &FxHashSet<IdentifierId>,
    test_only_ids: &FxHashSet<IdentifierId>,
) {
    let mut new_instructions = Vec::new();

    for instr in std::mem::take(&mut block.instructions) {
        match instr {
            ReactiveInstruction::Scope(mut scope_block) => {
                // Check if any declaration of this scope is used outside,
                // excluding declarations that are ONLY used as condition tests
                // (if test, switch test) — those don't truly "escape" the scope.
                let any_decl_used = scope_block
                    .scope
                    .declarations
                    .iter()
                    .any(|(id, _)| used_ids.contains(id) && !test_only_ids.contains(id));
                // Also check if any reassignment target is used outside
                // (upstream checks both declarations and reassignments)
                let any_reassign_used = scope_block.scope.reassignments.iter().any(|ident| {
                    used_ids.contains(&ident.id) && !test_only_ids.contains(&ident.id)
                });

                prune_scopes_in_block(&mut scope_block.instructions, used_ids, test_only_ids);

                // Check if ALL declarations are test-only (used only as condition tests)
                let all_decls_test_only = !scope_block.scope.declarations.is_empty()
                    && scope_block
                        .scope
                        .declarations
                        .iter()
                        .all(|(id, _)| test_only_ids.contains(id));

                // Keep if: any declaration or reassignment escapes, OR empty declarations
                // (handled by PropagateEarlyReturns later), OR is allocating/sentinel scope
                // (unless ALL declarations are test-only — in that case the allocation
                // result is discarded and the scope can be pruned per upstream's
                // PruneNonEscapingScopes), OR has an early return value.
                if any_decl_used
                    || any_reassign_used
                    || (scope_block.scope.declarations.is_empty()
                        && scope_block.scope.reassignments.is_empty())
                    || (scope_block.scope.is_allocating && !all_decls_test_only)
                    || scope_block.scope.early_return_value.is_some()
                {
                    new_instructions.push(ReactiveInstruction::Scope(scope_block));
                } else {
                    // Unwrap the scope: emit its instructions directly
                    for inner in scope_block.instructions.instructions {
                        new_instructions.push(inner);
                    }
                }
            }
            ReactiveInstruction::Terminal(mut terminal) => {
                prune_scopes_in_terminal(&mut terminal, used_ids, test_only_ids);
                new_instructions.push(ReactiveInstruction::Terminal(terminal));
            }
            other => {
                new_instructions.push(other);
            }
        }
    }

    block.instructions = new_instructions;
}

fn prune_scopes_in_terminal(
    terminal: &mut ReactiveTerminal,
    used_ids: &FxHashSet<IdentifierId>,
    test_only_ids: &FxHashSet<IdentifierId>,
) {
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            prune_scopes_in_block(consequent, used_ids, test_only_ids);
            prune_scopes_in_block(alternate, used_ids, test_only_ids);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                prune_scopes_in_block(block, used_ids, test_only_ids);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            prune_scopes_in_block(init, used_ids, test_only_ids);
            prune_scopes_in_block(test, used_ids, test_only_ids);
            if let Some(upd) = update {
                prune_scopes_in_block(upd, used_ids, test_only_ids);
            }
            prune_scopes_in_block(body, used_ids, test_only_ids);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            prune_scopes_in_block(init, used_ids, test_only_ids);
            prune_scopes_in_block(test, used_ids, test_only_ids);
            prune_scopes_in_block(body, used_ids, test_only_ids);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            prune_scopes_in_block(test, used_ids, test_only_ids);
            prune_scopes_in_block(body, used_ids, test_only_ids);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            prune_scopes_in_block(block, used_ids, test_only_ids);
            prune_scopes_in_block(handler, used_ids, test_only_ids);
        }
        ReactiveTerminal::Label { block, .. } => {
            prune_scopes_in_block(block, used_ids, test_only_ids);
        }
        ReactiveTerminal::Logical { right, .. } => {
            prune_scopes_in_block(right, used_ids, test_only_ids);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
    }
}

/// Prune reactive scopes with non-reactive dependencies.
pub fn prune_non_reactive_dependencies(rf: &mut ReactiveFunction) {
    prune_non_reactive_deps_in_block(&mut rf.body);
}

fn prune_non_reactive_deps_in_block(block: &mut ReactiveBlock) {
    for instr in &mut block.instructions {
        match instr {
            ReactiveInstruction::Scope(scope_block) => {
                // Keep all dependencies — reactivity determines which scopes to
                // create, not which dependencies to track. All external values
                // read inside a scope are needed for cache invalidation.
                prune_non_reactive_deps_in_block(&mut scope_block.instructions);
            }
            ReactiveInstruction::Terminal(terminal) => {
                prune_non_reactive_deps_in_terminal(terminal);
            }
            ReactiveInstruction::Instruction(_) => {}
        }
    }
}

fn prune_non_reactive_deps_in_terminal(terminal: &mut ReactiveTerminal) {
    for_each_block_in_terminal_mut(terminal, |block| {
        prune_non_reactive_deps_in_block(block);
    });
}

/// Prune unused reactive scopes (no declarations used outside).
pub fn prune_unused_scopes(rf: &mut ReactiveFunction) {
    // Collect all referenced identifier IDs across the function
    let mut referenced = FxHashSet::default();
    collect_all_referenced_ids(&rf.body, &mut referenced);
    prune_unused_scopes_in_block(&mut rf.body, &referenced);
}

fn collect_all_referenced_ids(block: &ReactiveBlock, referenced: &mut FxHashSet<IdentifierId>) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                collect_instruction_operand_ids(&instruction.value, referenced);
                referenced.insert(instruction.lvalue.identifier.id);
            }
            ReactiveInstruction::Scope(scope_block) => {
                collect_all_referenced_ids(&scope_block.instructions, referenced);
            }
            ReactiveInstruction::Terminal(terminal) => {
                for_each_block_in_terminal(terminal, |block| {
                    collect_all_referenced_ids(block, referenced);
                });
            }
        }
    }
}

fn prune_unused_scopes_in_block(block: &mut ReactiveBlock, referenced: &FxHashSet<IdentifierId>) {
    let mut new_instructions = Vec::new();

    for instr in std::mem::take(&mut block.instructions) {
        match instr {
            ReactiveInstruction::Scope(mut scope_block) => {
                prune_unused_scopes_in_block(&mut scope_block.instructions, referenced);

                let has_used_decls =
                    scope_block.scope.declarations.iter().any(|(id, _)| referenced.contains(id));

                // Keep scopes that either have used declarations, have dependencies,
                // or are allocating (sentinel scopes for non-reactive allocations).
                // Only prune scopes with NO used declarations AND NO dependencies
                // AND not allocating.
                let has_deps = !scope_block.scope.dependencies.is_empty();
                if has_used_decls || has_deps || scope_block.scope.is_allocating {
                    new_instructions.push(ReactiveInstruction::Scope(scope_block));
                } else {
                    for inner in scope_block.instructions.instructions {
                        new_instructions.push(inner);
                    }
                }
            }
            ReactiveInstruction::Terminal(mut terminal) => {
                for_each_block_in_terminal_mut(&mut terminal, |block| {
                    prune_unused_scopes_in_block(block, referenced);
                });
                new_instructions.push(ReactiveInstruction::Terminal(terminal));
            }
            other => {
                new_instructions.push(other);
            }
        }
    }

    block.instructions = new_instructions;
}

/// Prune scopes that always invalidate (deps change every render).
///
/// Upstream: PruneAlwaysInvalidatingScopes.ts
///
/// Tracks values that are freshly allocated each render (arrays, objects, JSX,
/// functions, new-expressions). If such a value is NOT inside a reactive scope,
/// it is "unmemoized" — any scope depending on it will invalidate every render
/// and should be pruned. Propagates through LoadLocal/StoreLocal aliases.
pub fn prune_always_invalidating_scopes(rf: &mut ReactiveFunction) {
    let mut always_invalidating = FxHashSet::default();
    let mut unmemoized = FxHashSet::default();
    prune_always_invalidating_in_block(
        &mut rf.body,
        false,
        &mut always_invalidating,
        &mut unmemoized,
    );
}

fn is_always_invalidating_instruction(value: &InstructionValue) -> bool {
    matches!(
        value,
        InstructionValue::ArrayExpression { .. }
            | InstructionValue::ObjectExpression { .. }
            | InstructionValue::JsxExpression { .. }
            | InstructionValue::JsxFragment { .. }
            | InstructionValue::NewExpression { .. }
            | InstructionValue::FunctionExpression { .. }
            | InstructionValue::ObjectMethod { .. }
    )
}

fn prune_always_invalidating_in_block(
    block: &mut ReactiveBlock,
    within_scope: bool,
    always_invalidating: &mut FxHashSet<IdentifierId>,
    unmemoized: &mut FxHashSet<IdentifierId>,
) {
    let mut new_instructions = Vec::new();

    for instr in std::mem::take(&mut block.instructions) {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                // Track always-invalidating values
                if is_always_invalidating_instruction(&instruction.value) {
                    always_invalidating.insert(instruction.lvalue.identifier.id);
                    if !within_scope {
                        unmemoized.insert(instruction.lvalue.identifier.id);
                    }
                }

                // Propagate through LoadLocal/StoreLocal
                match &instruction.value {
                    InstructionValue::StoreLocal { value, .. }
                    | InstructionValue::StoreContext { value, .. } => {
                        if always_invalidating.contains(&value.identifier.id) {
                            always_invalidating.insert(instruction.lvalue.identifier.id);
                        }
                        if unmemoized.contains(&value.identifier.id) {
                            unmemoized.insert(instruction.lvalue.identifier.id);
                        }
                    }
                    InstructionValue::LoadLocal { place }
                    | InstructionValue::LoadContext { place } => {
                        if always_invalidating.contains(&place.identifier.id) {
                            always_invalidating.insert(instruction.lvalue.identifier.id);
                        }
                        if unmemoized.contains(&place.identifier.id) {
                            unmemoized.insert(instruction.lvalue.identifier.id);
                        }
                    }
                    _ => {}
                }

                new_instructions.push(ReactiveInstruction::Instruction(instruction));
            }
            ReactiveInstruction::Scope(mut scope_block) => {
                // Recurse into scope body (within_scope = true)
                prune_always_invalidating_in_block(
                    &mut scope_block.instructions,
                    true,
                    always_invalidating,
                    unmemoized,
                );

                // Check if any dependency is an unmemoized always-invalidating value
                let always_invalidates = scope_block
                    .scope
                    .dependencies
                    .iter()
                    .any(|dep| unmemoized.contains(&dep.identifier.id));

                if always_invalidates {
                    // Prune: promote declarations that are always-invalidating
                    // to unmemoized (they're now outside any scope)
                    for (decl_id, _) in &scope_block.scope.declarations {
                        if always_invalidating.contains(decl_id) {
                            unmemoized.insert(*decl_id);
                        }
                    }
                    // Emit inner instructions inline
                    for inner in scope_block.instructions.instructions {
                        new_instructions.push(inner);
                    }
                } else {
                    new_instructions.push(ReactiveInstruction::Scope(scope_block));
                }
            }
            ReactiveInstruction::Terminal(mut terminal) => {
                for_each_block_in_terminal_mut(&mut terminal, |block| {
                    prune_always_invalidating_in_block(
                        block,
                        within_scope,
                        always_invalidating,
                        unmemoized,
                    );
                });
                new_instructions.push(ReactiveInstruction::Terminal(terminal));
            }
        }
    }

    block.instructions = new_instructions;
}

/// Prune unused labels in ReactiveFunction.
pub fn prune_unused_labels(rf: &mut ReactiveFunction) {
    // Collect all label IDs that are targets of break/continue.
    // In the current IR model, breaks are encoded in the CFG structure,
    // so unused labels are those whose body has no break target referencing them.
    prune_labels_in_block(&mut rf.body);
}

fn prune_labels_in_block(block: &mut ReactiveBlock) {
    let mut new_instructions = Vec::new();

    for instr in std::mem::take(&mut block.instructions) {
        match instr {
            ReactiveInstruction::Terminal(ReactiveTerminal::Label {
                block: mut label_block,
                label,
                id,
            }) => {
                prune_labels_in_block(&mut label_block);
                // Keep all labels for now — a full implementation would track
                // break targets and remove unused ones
                new_instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Label {
                    block: label_block,
                    label,
                    id,
                }));
            }
            ReactiveInstruction::Terminal(mut terminal) => {
                for_each_block_in_terminal_mut(&mut terminal, prune_labels_in_block);
                new_instructions.push(ReactiveInstruction::Terminal(terminal));
            }
            ReactiveInstruction::Scope(mut scope_block) => {
                prune_labels_in_block(&mut scope_block.instructions);
                new_instructions.push(ReactiveInstruction::Scope(scope_block));
            }
            other => {
                new_instructions.push(other);
            }
        }
    }

    block.instructions = new_instructions;
}

/// Propagate early returns through scopes.
pub fn propagate_early_returns(rf: &mut ReactiveFunction) {
    propagate_early_returns_in_block(&mut rf.body);
}

fn propagate_early_returns_in_block(block: &mut ReactiveBlock) {
    for instr in &mut block.instructions {
        match instr {
            ReactiveInstruction::Scope(scope_block) => {
                propagate_early_returns_in_block(&mut scope_block.instructions);
                // Check if the last instruction is a return — if so, mark it as early return
                if let Some(ReactiveInstruction::Terminal(ReactiveTerminal::Return {
                    value, ..
                })) = scope_block.instructions.instructions.last()
                {
                    scope_block.scope.early_return_value =
                        Some(crate::hir::types::EarlyReturnValue {
                            value: value.clone(),
                            loc: scope_block.scope.loc,
                        });
                }
            }
            ReactiveInstruction::Terminal(terminal) => {
                for_each_block_in_terminal_mut(terminal, propagate_early_returns_in_block);
            }
            ReactiveInstruction::Instruction(_) => {}
        }
    }
}

/// Inline `LoadLocal` identity copies to reduce temp variable explosion.
///
/// When the HIR lowers `bar(props.a)`, it produces:
/// ```text
/// t28 = LoadLocal(bar)
/// t29 = LoadLocal(props)
/// t30 = PropertyLoad(t29, "a")
/// t31 = CallExpression(t28, [t30])
/// ```
///
/// This pass replaces uses of identity-copy temps with their source, turning
/// the above into:
/// ```text
/// t30 = PropertyLoad(props, "a")
/// t31 = CallExpression(bar, [t30])
/// ```
///
/// Dead `LoadLocal` instructions are then removed by `prune_unused_lvalues`.
pub fn inline_load_locals(rf: &mut ReactiveFunction) {
    inline_loads_in_block(&mut rf.body);
}

fn inline_loads_in_block(block: &mut ReactiveBlock) {
    // Phase 1: Build substitution map — for each LoadLocal(source) where lvalue
    // is an unnamed temp, map lvalue.id → source place.
    // Also collect from inside scope blocks: LoadLocal temps inside scopes that
    // are used at the parent level (e.g., Return terminal) need cross-scope inlining.
    let mut substitutions: FxHashMap<IdentifierId, Place> = FxHashMap::default();

    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                if let InstructionValue::LoadLocal { place: source } = &instruction.value {
                    let lvalue = &instruction.lvalue;
                    if lvalue.identifier.name.is_none() {
                        substitutions.insert(lvalue.identifier.id, source.clone());
                    }
                }
            }
            ReactiveInstruction::Scope(scope_block) => {
                // Collect LoadLocal subs from inside the scope that reference
                // named variables (scope outputs). These need to be visible
                // at the parent level for terminal operand substitution.
                for inner in &scope_block.instructions.instructions {
                    if let ReactiveInstruction::Instruction(instruction) = inner
                        && let InstructionValue::LoadLocal { place: source } = &instruction.value
                        && source.identifier.name.is_some()
                        && instruction.lvalue.identifier.name.is_none()
                    {
                        substitutions.insert(instruction.lvalue.identifier.id, source.clone());
                    }
                }
            }
            _ => {}
        }
    }

    if substitutions.is_empty() {
        // Recurse into nested blocks even if no subs at this level
        for instr in &mut block.instructions {
            match instr {
                ReactiveInstruction::Scope(scope_block) => {
                    inline_loads_in_block(&mut scope_block.instructions);
                }
                ReactiveInstruction::Terminal(terminal) => {
                    for_each_block_in_terminal_mut(terminal, inline_loads_in_block);
                }
                ReactiveInstruction::Instruction(_) => {}
            }
        }
        return;
    }

    // Resolve transitive substitutions: if t0 → t1 and t1 → x, then t0 → x
    let resolved = resolve_transitive_subs(&substitutions);

    // Phase 2: Apply substitutions to all operands in instructions
    for instr in &mut block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                substitute_places_in_value(&mut instruction.value, &resolved);
            }
            ReactiveInstruction::Scope(scope_block) => {
                // Substitute in scope dependencies and declarations
                for dep in &mut scope_block.scope.dependencies {
                    substitute_place_identifier(&mut dep.identifier, &resolved);
                }
                for (_, decl) in &mut scope_block.scope.declarations {
                    substitute_place_identifier(&mut decl.identifier, &resolved);
                }
                // Apply subs inside the scope block
                for inner in &mut scope_block.instructions.instructions {
                    if let ReactiveInstruction::Instruction(instruction) = inner {
                        substitute_places_in_value(&mut instruction.value, &resolved);
                    }
                }
                // Recurse into nested blocks
                inline_loads_in_block(&mut scope_block.instructions);
            }
            ReactiveInstruction::Terminal(terminal) => {
                substitute_places_in_terminal(terminal, &resolved);
                for_each_block_in_terminal_mut(terminal, inline_loads_in_block);
            }
        }
    }
}

fn resolve_transitive_subs(
    subs: &FxHashMap<IdentifierId, Place>,
) -> FxHashMap<IdentifierId, Place> {
    let mut resolved = FxHashMap::default();
    for (&id, place) in subs {
        let mut current = place.clone();
        // Follow the chain: if the source is also in subs, keep going
        let mut depth = 0;
        while let Some(next) = subs.get(&current.identifier.id) {
            current = next.clone();
            depth += 1;
            if depth > 20 {
                break; // Safety: avoid infinite loops
            }
        }
        resolved.insert(id, current);
    }
    resolved
}

fn substitute_place_identifier(
    identifier: &mut crate::hir::types::Identifier,
    subs: &FxHashMap<IdentifierId, Place>,
) {
    if let Some(replacement) = subs.get(&identifier.id) {
        *identifier = replacement.identifier.clone();
    }
}

fn substitute_place(place: &mut Place, subs: &FxHashMap<IdentifierId, Place>) {
    if let Some(replacement) = subs.get(&place.identifier.id) {
        *place = replacement.clone();
    }
}

fn substitute_places_in_terminal(
    terminal: &mut ReactiveTerminal,
    subs: &FxHashMap<IdentifierId, Place>,
) {
    match terminal {
        ReactiveTerminal::If { test, .. } => substitute_place(test, subs),
        ReactiveTerminal::Return { value, .. } | ReactiveTerminal::Throw { value, .. } => {
            substitute_place(value, subs);
        }
        ReactiveTerminal::Switch { test, .. } => substitute_place(test, subs),
        ReactiveTerminal::For { .. }
        | ReactiveTerminal::ForOf { .. }
        | ReactiveTerminal::ForIn { .. }
        | ReactiveTerminal::While { .. }
        | ReactiveTerminal::DoWhile { .. }
        | ReactiveTerminal::Try { .. }
        | ReactiveTerminal::Label { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
        ReactiveTerminal::Logical { result, .. } => {
            if let Some(r) = result {
                substitute_place(r, subs);
            }
        }
    }
}

fn substitute_places_in_value(value: &mut InstructionValue, subs: &FxHashMap<IdentifierId, Place>) {
    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            substitute_place(place, subs);
        }
        InstructionValue::StoreLocal { lvalue, value, .. } => {
            substitute_place(lvalue, subs);
            substitute_place(value, subs);
        }
        InstructionValue::StoreContext { lvalue, value } => {
            substitute_place(lvalue, subs);
            substitute_place(value, subs);
        }
        InstructionValue::DeclareLocal { lvalue, .. } => {
            substitute_place(lvalue, subs);
        }
        InstructionValue::DeclareContext { lvalue } => {
            substitute_place(lvalue, subs);
        }
        InstructionValue::Destructure { value, .. } => {
            substitute_place(value, subs);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            substitute_place(left, subs);
            substitute_place(right, subs);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            substitute_place(value, subs);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            substitute_place(lvalue, subs);
        }
        InstructionValue::CallExpression { callee, args, .. } => {
            substitute_place(callee, subs);
            for arg in args {
                substitute_place(arg, subs);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            substitute_place(receiver, subs);
            for arg in args {
                substitute_place(arg, subs);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            substitute_place(callee, subs);
            for arg in args {
                substitute_place(arg, subs);
            }
        }
        InstructionValue::PropertyLoad { object, .. } => {
            substitute_place(object, subs);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            substitute_place(object, subs);
            substitute_place(value, subs);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            substitute_place(object, subs);
            substitute_place(property, subs);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            substitute_place(object, subs);
            substitute_place(property, subs);
            substitute_place(value, subs);
        }
        InstructionValue::PropertyDelete { object, .. } => {
            substitute_place(object, subs);
        }
        InstructionValue::ComputedDelete { object, property } => {
            substitute_place(object, subs);
            substitute_place(property, subs);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                substitute_place(&mut prop.value, subs);
                if let ObjectPropertyKey::Computed(p) = &mut prop.key {
                    substitute_place(p, subs);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    ArrayElement::Spread(p) | ArrayElement::Expression(p) => {
                        substitute_place(p, subs);
                    }
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            substitute_place(tag, subs);
            for attr in props {
                substitute_place(&mut attr.value, subs);
            }
            for child in children {
                substitute_place(child, subs);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                substitute_place(child, subs);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                substitute_place(sub, subs);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            substitute_place(tag, subs);
            for sub in &mut value.subexpressions {
                substitute_place(sub, subs);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => {
            substitute_place(value, subs);
        }
        InstructionValue::Await { value } => {
            substitute_place(value, subs);
        }
        InstructionValue::GetIterator { collection } => {
            substitute_place(collection, subs);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            substitute_place(iterator, subs);
        }
        InstructionValue::NextPropertyOf { value } => {
            substitute_place(value, subs);
        }
        InstructionValue::TypeCastExpression { value, .. } => {
            substitute_place(value, subs);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            substitute_place(decl, subs);
            for dep in deps {
                substitute_place(dep, subs);
            }
        }
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

/// Prune unused lvalues.
pub fn prune_unused_lvalues(rf: &mut ReactiveFunction) {
    // Collect all referenced IDs
    let mut referenced = FxHashSet::default();
    collect_all_referenced_ids(&rf.body, &mut referenced);

    // Remove instructions whose lvalues are never referenced (except side-effectful ones)
    prune_lvalues_in_block(&mut rf.body, &referenced);
}

fn prune_lvalues_in_block(block: &mut ReactiveBlock, referenced: &FxHashSet<IdentifierId>) {
    block.instructions.retain(|instr| {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                let id = instruction.lvalue.identifier.id;
                // Keep if referenced, or if it has side effects
                referenced.contains(&id) || has_side_effects(&instruction.value)
            }
            // Always keep terminals and scopes
            ReactiveInstruction::Terminal(_) | ReactiveInstruction::Scope(_) => true,
        }
    });

    for instr in &mut block.instructions {
        match instr {
            ReactiveInstruction::Scope(scope_block) => {
                prune_lvalues_in_block(&mut scope_block.instructions, referenced);
            }
            ReactiveInstruction::Terminal(terminal) => {
                for_each_block_in_terminal_mut(terminal, |block| {
                    prune_lvalues_in_block(block, referenced);
                });
            }
            ReactiveInstruction::Instruction(_) => {}
        }
    }
}

fn has_side_effects(value: &crate::hir::types::InstructionValue) -> bool {
    use crate::hir::types::InstructionValue;
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
            | InstructionValue::Await { .. }
            | InstructionValue::Destructure { .. }
    )
}

/// Promote used temporaries to named variables.
pub fn promote_used_temporaries(rf: &mut ReactiveFunction) {
    // Walk the tree and rename unnamed temporaries that cross scope boundaries
    let mut counter = 0u32;
    promote_temps_in_block(&mut rf.body, &mut counter);
}

fn promote_temps_in_block(block: &mut ReactiveBlock, counter: &mut u32) {
    for instr in &mut block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                promote_place(&mut instruction.lvalue);
                promote_places_in_value(&mut instruction.value);
            }
            ReactiveInstruction::Scope(scope_block) => {
                // Promote identifiers in scope dependencies and declarations
                for dep in &mut scope_block.scope.dependencies {
                    promote_identifier(&mut dep.identifier);
                }
                for (_, decl) in &mut scope_block.scope.declarations {
                    promote_identifier(&mut decl.identifier);
                }
                promote_temps_in_block(&mut scope_block.instructions, counter);
            }
            ReactiveInstruction::Terminal(terminal) => {
                promote_places_in_terminal(terminal);
                for_each_block_in_terminal_mut(terminal, |block| {
                    promote_temps_in_block(block, counter);
                });
            }
        }
    }
}

fn promote_identifier(identifier: &mut crate::hir::types::Identifier) {
    if identifier.name.is_none() {
        identifier.name = Some(format!("t{}", identifier.id.0));
    }
}

fn promote_place(place: &mut Place) {
    promote_identifier(&mut place.identifier);
}

fn promote_places_in_terminal(terminal: &mut ReactiveTerminal) {
    match terminal {
        ReactiveTerminal::If { test, .. } => {
            promote_place(test);
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            promote_place(test);
            for (test_val, _) in cases {
                if let Some(tv) = test_val {
                    promote_place(tv);
                }
            }
        }
        ReactiveTerminal::Return { value, .. } => {
            promote_place(value);
        }
        ReactiveTerminal::Throw { value, .. } => {
            promote_place(value);
        }
        ReactiveTerminal::For { .. }
        | ReactiveTerminal::ForOf { .. }
        | ReactiveTerminal::ForIn { .. }
        | ReactiveTerminal::While { .. }
        | ReactiveTerminal::DoWhile { .. }
        | ReactiveTerminal::Label { .. }
        | ReactiveTerminal::Try { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {
            // These terminals have blocks but no direct Place fields
            // (blocks are walked by for_each_block_in_terminal_mut)
        }
        ReactiveTerminal::Logical { result, .. } => {
            if let Some(r) = result {
                promote_place(r);
            }
        }
    }
}

fn promote_places_in_value(value: &mut InstructionValue) {
    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            promote_place(place);
        }
        InstructionValue::StoreLocal { lvalue, value, .. } => {
            promote_place(lvalue);
            promote_place(value);
        }
        InstructionValue::StoreContext { lvalue, value } => {
            promote_place(lvalue);
            promote_place(value);
        }
        InstructionValue::DeclareLocal { lvalue, .. } => {
            promote_place(lvalue);
        }
        InstructionValue::DeclareContext { lvalue } => {
            promote_place(lvalue);
        }
        InstructionValue::Destructure { value, .. } => {
            promote_place(value);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            promote_place(left);
            promote_place(right);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            promote_place(value);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            promote_place(lvalue);
        }
        InstructionValue::CallExpression { callee, args, .. } => {
            promote_place(callee);
            for arg in args {
                promote_place(arg);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            promote_place(receiver);
            for arg in args {
                promote_place(arg);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            promote_place(callee);
            for arg in args {
                promote_place(arg);
            }
        }
        InstructionValue::PropertyLoad { object, .. } => {
            promote_place(object);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            promote_place(object);
            promote_place(value);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            promote_place(object);
            promote_place(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            promote_place(object);
            promote_place(property);
            promote_place(value);
        }
        InstructionValue::PropertyDelete { object, .. } => {
            promote_place(object);
        }
        InstructionValue::ComputedDelete { object, property } => {
            promote_place(object);
            promote_place(property);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                promote_place(&mut prop.value);
                if let ObjectPropertyKey::Computed(p) = &mut prop.key {
                    promote_place(p);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    ArrayElement::Spread(p) | ArrayElement::Expression(p) => {
                        promote_place(p);
                    }
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            promote_place(tag);
            for attr in props {
                promote_place(&mut attr.value);
            }
            for child in children {
                promote_place(child);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                promote_place(child);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                promote_place(sub);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            promote_place(tag);
            for sub in &mut value.subexpressions {
                promote_place(sub);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => {
            promote_place(value);
        }
        InstructionValue::Await { value } => {
            promote_place(value);
        }
        InstructionValue::GetIterator { collection } => {
            promote_place(collection);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            promote_place(iterator);
        }
        InstructionValue::NextPropertyOf { value } => {
            promote_place(value);
        }
        InstructionValue::TypeCastExpression { value, .. } => {
            promote_place(value);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            promote_place(decl);
            for dep in deps {
                promote_place(dep);
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

/// Extract scope declarations from destructuring patterns.
pub fn extract_scope_declarations_from_destructuring(rf: &mut ReactiveFunction) {
    // Walk the tree looking for Destructure instructions inside scopes
    // and extract individual declarations
    extract_destructuring_in_block(&mut rf.body);
}

fn extract_destructuring_in_block(block: &mut ReactiveBlock) {
    for instr in &mut block.instructions {
        match instr {
            ReactiveInstruction::Scope(scope_block) => {
                extract_destructuring_in_block(&mut scope_block.instructions);
            }
            ReactiveInstruction::Terminal(terminal) => {
                for_each_block_in_terminal_mut(terminal, extract_destructuring_in_block);
            }
            ReactiveInstruction::Instruction(_) => {}
        }
    }
}

/// Stabilize block IDs for deterministic output.
pub fn stabilize_block_ids(rf: &mut ReactiveFunction) {
    let mut next_id = 0u32;
    stabilize_ids_in_block(&mut rf.body, &mut next_id);
}

fn stabilize_ids_in_block(block: &mut ReactiveBlock, next_id: &mut u32) {
    for instr in &mut block.instructions {
        match instr {
            ReactiveInstruction::Instruction(_) => {}
            ReactiveInstruction::Scope(scope_block) => {
                stabilize_ids_in_block(&mut scope_block.instructions, next_id);
            }
            ReactiveInstruction::Terminal(terminal) => {
                // Renumber the terminal's block ID
                set_terminal_id(terminal, crate::hir::types::BlockId(*next_id));
                *next_id += 1;
                for_each_block_in_terminal_mut(terminal, |block| {
                    stabilize_ids_in_block(block, next_id);
                });
            }
        }
    }
}

fn set_terminal_id(terminal: &mut ReactiveTerminal, new_id: crate::hir::types::BlockId) {
    match terminal {
        ReactiveTerminal::If { id, .. }
        | ReactiveTerminal::Switch { id, .. }
        | ReactiveTerminal::For { id, .. }
        | ReactiveTerminal::ForOf { id, .. }
        | ReactiveTerminal::ForIn { id, .. }
        | ReactiveTerminal::While { id, .. }
        | ReactiveTerminal::DoWhile { id, .. }
        | ReactiveTerminal::Label { id, .. }
        | ReactiveTerminal::Try { id, .. }
        | ReactiveTerminal::Logical { id, .. }
        | ReactiveTerminal::Return { id, .. }
        | ReactiveTerminal::Throw { id, .. }
        | ReactiveTerminal::Continue { id, .. }
        | ReactiveTerminal::Break { id, .. } => {
            *id = new_id;
        }
    }
}

/// Rename variables for clean output.
/// Rename scope declaration outputs to temporary variable names.
///
/// Upstream's codegen renames variables that are scope outputs to temporary
/// names (t0, t1, ...) and adds an assignment from the temp to the original
/// variable after the scope block. This matches upstream's output format:
///
/// ```js
/// let t0;
/// if ($[0] !== dep) {
///   t0 = computation();
///   $[0] = dep; $[1] = t0;
/// } else { t0 = $[1]; }
/// const x = t0;  // assignment from temp to original
/// ```
///
/// Without this pass, our output uses the original variable name directly
/// in the scope block, causing token-level mismatches with upstream output.
pub fn rename_variables(rf: &mut ReactiveFunction) {
    // Upstream CodegenReactiveFunction.ts renames scope declaration outputs to
    // sequential temp names (t0, t1, ...) and emits `const originalName = tN`
    // after the scope block. We skip renaming declarations that:
    //   - already have a temp name (tN)
    //   - are reassigned (lvalue appears multiple times in scope body)
    //   - have property/computed mutations (PropertyStore, ComputedStore, etc.)
    let mut counter = scan_max_temp_counter(rf);
    rename_vars_in_block(&mut rf.body, &mut counter);
}

/// Check if a name is already a compiler temporary (matches `tN` pattern).
fn is_temp_var_name(name: &str) -> bool {
    name.starts_with('t') && name.len() >= 2 && name[1..].chars().all(|c| c.is_ascii_digit())
}

/// Extract the numeric suffix from a temp name like "t42" → Some(42).
fn temp_name_index(name: &str) -> Option<u32> {
    if is_temp_var_name(name) { name[1..].parse::<u32>().ok() } else { None }
}

/// Update `max` if `name` is a temp name with index >= current max.
fn update_max_temp(name: &str, max: &mut u32) {
    if let Some(n) = temp_name_index(name) {
        *max = (*max).max(n + 1);
    }
}

/// Scan all identifiers in the ReactiveFunction to find the highest `N` in any
/// existing `t{N}` name. The rename counter starts from `max_N + 1` to avoid
/// collisions with names assigned by `promote_used_temporaries`.
fn scan_max_temp_counter(rf: &ReactiveFunction) -> u32 {
    let mut max = 0u32;
    for param in &rf.params {
        let place = match param {
            Param::Identifier(p) | Param::Spread(p) => p,
        };
        if let Some(ref name) = place.identifier.name {
            update_max_temp(name, &mut max);
        }
    }
    scan_max_in_block(&rf.body, &mut max);
    max
}

fn scan_max_in_block(block: &ReactiveBlock, max: &mut u32) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(i) => {
                if let Some(ref name) = i.lvalue.identifier.name {
                    update_max_temp(name, max);
                }
            }
            ReactiveInstruction::Scope(sb) => {
                for (_, decl) in &sb.scope.declarations {
                    if let Some(ref name) = decl.identifier.name {
                        update_max_temp(name, max);
                    }
                }
                scan_max_in_block(&sb.instructions, max);
            }
            ReactiveInstruction::Terminal(terminal) => {
                for_each_block_in_terminal(terminal, |b| scan_max_in_block(b, max));
            }
        }
    }
}

/// Returns true if a scope declaration with the given name should be renamed
/// to a temp variable. A declaration is NOT renamed if:
/// - It already has a temp name (`t{digits}`) — checked by caller
/// - It is a reassignment of an outer-scope variable
/// - It is MUTATED inside the scope (PropertyStore, ComputedStore, MethodCall,
///   PrefixUpdate, PostfixUpdate — these modify the value's properties/state)
/// - There are other instructions in the scope body AFTER the variable's
///   assignment (upstream keeps original names in complex scope bodies)
///
/// PropertyLoad reads (e.g., `x.b`) do NOT prevent renaming — they're
/// scope-internal property access, and upstream renames regardless.
fn can_rename_scope_decl(name: &str, scope_body: &ReactiveBlock) -> bool {
    let mut lvalue_count = 0u32;
    let mut read_count = 0u32;
    let mut is_reassign = false;
    check_rename_eligibility(
        scope_body,
        name,
        &mut lvalue_count,
        &mut read_count,
        &mut is_reassign,
    );
    // Only block rename for reassignment or mutations (not reads).
    // `read_count` tracks mutations (PropertyStore, MethodCall, etc.) and
    // other meaningful uses (function args, return values).
    // With named lvalues, `lvalue_count` tracks writes (StoreLocal).
    if is_reassign || lvalue_count > 1 || read_count > 0 {
        return false;
    }
    // Upstream only renames when the variable's assignment is the last
    // meaningful instruction in the scope body. If there are other
    // instructions after it (even if they don't reference this variable),
    // upstream keeps the original name.
    is_last_assignment_in_scope(name, scope_body)
}

/// Check if the named variable's assignment is the last meaningful instruction
/// in the scope body (no other instructions follow it at the top level).
fn is_last_assignment_in_scope(name: &str, scope_body: &ReactiveBlock) -> bool {
    let mut found_assignment = false;
    for instr in &scope_body.instructions {
        if found_assignment {
            // There's an instruction after the assignment — don't rename
            return false;
        }
        if let ReactiveInstruction::Instruction(i) = instr
            && i.lvalue.identifier.name.as_deref() == Some(name)
            && !matches!(
                i.value,
                InstructionValue::DeclareLocal { .. } | InstructionValue::DeclareContext { .. }
            )
        {
            found_assignment = true;
        }
    }
    found_assignment
}

fn check_rename_eligibility(
    block: &ReactiveBlock,
    name: &str,
    lvalue_count: &mut u32,
    read_count: &mut u32,
    is_reassign: &mut bool,
) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(i) => {
                // Check if this instruction's lvalue uses the name.
                // DeclareLocal doesn't count as a "write" — it's a declaration
                // without assignment. Only StoreLocal/StoreContext and other
                // value-producing instructions count.
                if i.lvalue.identifier.name.as_deref() == Some(name)
                    && !matches!(
                        i.value,
                        InstructionValue::DeclareLocal { .. }
                            | InstructionValue::DeclareContext { .. }
                    )
                {
                    *lvalue_count += 1;
                }
                // Check for StoreLocal with Reassign kind — means the binding
                // was declared outside this scope and is being reassigned here.
                if let InstructionValue::StoreLocal {
                    lvalue,
                    type_: Some(InstructionKind::Reassign),
                    ..
                } = &i.value
                    && lvalue.identifier.name.as_deref() == Some(name)
                {
                    *is_reassign = true;
                }
                // Count all reads of the name in the instruction value
                count_reads_in_value(&i.value, name, lvalue_count, read_count);
            }
            ReactiveInstruction::Scope(sb) => {
                check_rename_eligibility(
                    &sb.instructions,
                    name,
                    lvalue_count,
                    read_count,
                    is_reassign,
                );
            }
            ReactiveInstruction::Terminal(terminal) => {
                count_reads_in_terminal(terminal, name, read_count);
                for_each_block_in_terminal(terminal, |b| {
                    check_rename_eligibility(b, name, lvalue_count, read_count, is_reassign);
                });
            }
        }
    }
}

/// Count references to `name` in a terminal's operands (test conditions, etc.)
fn count_reads_in_terminal(terminal: &ReactiveTerminal, name: &str, read_count: &mut u32) {
    match terminal {
        ReactiveTerminal::If { test, .. } => {
            if test.identifier.name.as_deref() == Some(name) {
                *read_count += 1;
            }
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            if test.identifier.name.as_deref() == Some(name) {
                *read_count += 1;
            }
            for (case_test, _) in cases {
                if let Some(tv) = case_test
                    && tv.identifier.name.as_deref() == Some(name)
                {
                    *read_count += 1;
                }
            }
        }
        ReactiveTerminal::Return { value, .. } | ReactiveTerminal::Throw { value, .. } => {
            if value.identifier.name.as_deref() == Some(name) {
                *read_count += 1;
            }
        }
        _ => {}
    }
}

/// Count all reads of `name` within an instruction value's operand places.
/// Also counts StoreLocal/StoreContext/DeclareLocal lvalue uses as additional writes.
fn count_reads_in_value(
    value: &InstructionValue,
    name: &str,
    lvalue_count: &mut u32,
    read_count: &mut u32,
) {
    let is_name = |place: &Place| place.identifier.name.as_deref() == Some(name);

    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            // LoadLocal/LoadContext of a scope-declared variable means the variable
            // is used within the scope (e.g., passed to a function call, used in an
            // expression). This prevents renaming to keep the original name.
            if is_name(place) {
                *read_count += 1;
            }
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            // Note: with named lvalues, instr.lvalue and the inner StoreLocal
            // lvalue are the same place. The outer check_rename_eligibility
            // already counts instr.lvalue, so we skip counting the inner lvalue
            // to avoid double-counting. Only count reads (the value operand).
            let _ = lvalue; // acknowledged but not counted
            if is_name(value) {
                *read_count += 1;
            }
        }
        InstructionValue::DeclareLocal { lvalue, .. }
        | InstructionValue::DeclareContext { lvalue, .. } => {
            // Same as StoreLocal: inner lvalue is the same as instr.lvalue
            let _ = lvalue;
        }
        InstructionValue::CallExpression { callee, args, .. }
        | InstructionValue::NewExpression { callee, args } => {
            if is_name(callee) {
                *read_count += 1;
            }
            for arg in args {
                if is_name(arg) {
                    *read_count += 1;
                }
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            if is_name(receiver) {
                *read_count += 1;
            }
            for arg in args {
                if is_name(arg) {
                    *read_count += 1;
                }
            }
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            if is_name(object) {
                *read_count += 1;
            }
            if is_name(value) {
                *read_count += 1;
            }
        }
        InstructionValue::ComputedStore { object, property, value } => {
            if is_name(object) {
                *read_count += 1;
            }
            if is_name(property) {
                *read_count += 1;
            }
            if is_name(value) {
                *read_count += 1;
            }
        }
        InstructionValue::PropertyDelete { object, .. } => {
            if is_name(object) {
                *read_count += 1;
            }
        }
        InstructionValue::ComputedDelete { object, property } => {
            if is_name(object) {
                *read_count += 1;
            }
            if is_name(property) {
                *read_count += 1;
            }
        }
        InstructionValue::PropertyLoad { object, .. } => {
            if is_name(object) {
                *read_count += 1;
            }
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            if is_name(object) {
                *read_count += 1;
            }
            if is_name(property) {
                *read_count += 1;
            }
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            if is_name(left) {
                *read_count += 1;
            }
            if is_name(right) {
                *read_count += 1;
            }
        }
        InstructionValue::UnaryExpression { value, .. }
        | InstructionValue::Await { value }
        | InstructionValue::GetIterator { collection: value }
        | InstructionValue::NextPropertyOf { value }
        | InstructionValue::TypeCastExpression { value, .. }
        | InstructionValue::StoreGlobal { value, .. } => {
            if is_name(value) {
                *read_count += 1;
            }
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            if is_name(lvalue) {
                *lvalue_count += 1;
                *read_count += 1; // updates both read and write
            }
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            if is_name(iterator) {
                *read_count += 1;
            }
        }
        InstructionValue::Destructure { value, .. } => {
            if is_name(value) {
                *read_count += 1;
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for el in elements {
                match el {
                    ArrayElement::Expression(p) | ArrayElement::Spread(p) => {
                        if is_name(p) {
                            *read_count += 1;
                        }
                    }
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                if is_name(&prop.value) {
                    *read_count += 1;
                }
                if let ObjectPropertyKey::Computed(ref key) = prop.key
                    && is_name(key)
                {
                    *read_count += 1;
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            if is_name(tag) {
                *read_count += 1;
            }
            for attr in props {
                if is_name(&attr.value) {
                    *read_count += 1;
                }
            }
            for child in children {
                if is_name(child) {
                    *read_count += 1;
                }
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                if is_name(child) {
                    *read_count += 1;
                }
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                if is_name(sub) {
                    *read_count += 1;
                }
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            if is_name(tag) {
                *read_count += 1;
            }
            for sub in &value.subexpressions {
                if is_name(sub) {
                    *read_count += 1;
                }
            }
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            if is_name(decl) {
                *read_count += 1;
            }
            for dep in deps {
                if is_name(dep) {
                    *read_count += 1;
                }
            }
        }
        // No places to check
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

fn rename_vars_in_block(block: &mut ReactiveBlock, counter: &mut u32) {
    // Collect post-scope assignments to insert after processing
    let mut insertions: Vec<(usize, Vec<ReactiveInstruction>)> = Vec::new();

    for (idx, instr) in block.instructions.iter_mut().enumerate() {
        match instr {
            ReactiveInstruction::Scope(scope_block) => {
                // First recurse into the scope body
                rename_vars_in_block(&mut scope_block.instructions, counter);

                // For each declaration with a user-meaningful name, rename to temp
                // if the declaration qualifies (no reassignment, no property mutation).
                // Skip declarations that also appear in reassignments (declared outside scope).
                let mut renames: Vec<(String, String)> = Vec::new();
                for (_, decl) in &mut scope_block.scope.declarations {
                    if let Some(ref name) = decl.identifier.name
                        && !is_temp_var_name(name)
                        && !scope_block
                            .scope
                            .reassignments
                            .iter()
                            .any(|id| id.name.as_deref() == Some(name))
                        && can_rename_scope_decl(name, &scope_block.instructions)
                    {
                        let temp_name = format!("t{counter}");
                        *counter += 1;
                        let original_name = name.clone();
                        decl.identifier.name = Some(temp_name.clone());
                        renames.push((original_name, temp_name));
                    }
                }

                // Apply renames to all identifiers in the scope body
                if !renames.is_empty() {
                    apply_renames_in_block(&mut scope_block.instructions, &renames);
                }

                // Create post-scope assignments: `const originalName = tempName;`
                let mut post_assignments = Vec::new();
                for (original_name, temp_name) in &renames {
                    // Create a StoreLocal instruction: originalName = tempName
                    // We create a minimal instruction using fresh IDs
                    let store_instr = create_rename_assignment(original_name, temp_name);
                    post_assignments.push(ReactiveInstruction::Instruction(store_instr));
                }
                if !post_assignments.is_empty() {
                    insertions.push((idx + 1, post_assignments));
                }
            }
            ReactiveInstruction::Terminal(terminal) => {
                for_each_block_in_terminal_mut(terminal, |b| rename_vars_in_block(b, counter));
            }
            ReactiveInstruction::Instruction(_) => {}
        }
    }

    // Insert post-scope assignments (in reverse order to preserve indices)
    for (idx, assignments) in insertions.into_iter().rev() {
        for (i, assignment) in assignments.into_iter().enumerate() {
            if idx + i <= block.instructions.len() {
                block.instructions.insert(idx + i, assignment);
            }
        }
    }
}

/// Apply name renames to all identifiers in a reactive block.
fn apply_renames_in_block(block: &mut ReactiveBlock, renames: &[(String, String)]) {
    for instr in &mut block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                apply_renames_to_identifier(&mut instruction.lvalue.identifier, renames);
                apply_renames_to_value(&mut instruction.value, renames);
            }
            ReactiveInstruction::Scope(scope_block) => {
                // Apply renames to scope dependencies and declarations
                for dep in &mut scope_block.scope.dependencies {
                    apply_renames_to_identifier(&mut dep.identifier, renames);
                }
                for (_, decl) in &mut scope_block.scope.declarations {
                    apply_renames_to_identifier(&mut decl.identifier, renames);
                }
                apply_renames_in_block(&mut scope_block.instructions, renames);
            }
            ReactiveInstruction::Terminal(terminal) => {
                apply_renames_to_terminal(terminal, renames);
                for_each_block_in_terminal_mut(terminal, |b| apply_renames_in_block(b, renames));
            }
        }
    }
}

fn apply_renames_to_identifier(
    identifier: &mut crate::hir::types::Identifier,
    renames: &[(String, String)],
) {
    if let Some(ref name) = identifier.name {
        for (old_name, new_name) in renames {
            if name == old_name {
                identifier.name = Some(new_name.clone());
                return;
            }
        }
    }
}

fn apply_renames_to_place(place: &mut Place, renames: &[(String, String)]) {
    apply_renames_to_identifier(&mut place.identifier, renames);
}

fn apply_renames_to_terminal(terminal: &mut ReactiveTerminal, renames: &[(String, String)]) {
    match terminal {
        ReactiveTerminal::If { test, .. } => apply_renames_to_place(test, renames),
        ReactiveTerminal::Switch { test, cases, .. } => {
            apply_renames_to_place(test, renames);
            for (test_val, _) in cases {
                if let Some(tv) = test_val {
                    apply_renames_to_place(tv, renames);
                }
            }
        }
        ReactiveTerminal::Return { value, .. } => apply_renames_to_place(value, renames),
        ReactiveTerminal::Throw { value, .. } => apply_renames_to_place(value, renames),
        _ => {}
    }
}

fn apply_renames_to_value(value: &mut InstructionValue, renames: &[(String, String)]) {
    // Apply renames to all Place references in the instruction value
    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            apply_renames_to_place(place, renames);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            apply_renames_to_place(lvalue, renames);
            apply_renames_to_place(value, renames);
        }
        InstructionValue::DeclareLocal { lvalue, .. }
        | InstructionValue::DeclareContext { lvalue } => {
            apply_renames_to_place(lvalue, renames);
        }
        InstructionValue::Destructure { value, .. } => {
            apply_renames_to_place(value, renames);
        }
        InstructionValue::CallExpression { callee, args, .. }
        | InstructionValue::NewExpression { callee, args } => {
            apply_renames_to_place(callee, renames);
            for arg in args {
                apply_renames_to_place(arg, renames);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            apply_renames_to_place(receiver, renames);
            for arg in args {
                apply_renames_to_place(arg, renames);
            }
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            apply_renames_to_place(left, renames);
            apply_renames_to_place(right, renames);
        }
        InstructionValue::UnaryExpression { value, .. }
        | InstructionValue::TypeCastExpression { value, .. } => {
            apply_renames_to_place(value, renames);
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            apply_renames_to_place(object, renames);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            apply_renames_to_place(object, renames);
            apply_renames_to_place(value, renames);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            apply_renames_to_place(object, renames);
            apply_renames_to_place(property, renames);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            apply_renames_to_place(object, renames);
            apply_renames_to_place(property, renames);
            apply_renames_to_place(value, renames);
        }
        InstructionValue::ComputedDelete { object, property } => {
            apply_renames_to_place(object, renames);
            apply_renames_to_place(property, renames);
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            apply_renames_to_place(tag, renames);
            for attr in props {
                apply_renames_to_place(&mut attr.value, renames);
            }
            for child in children {
                apply_renames_to_place(child, renames);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                apply_renames_to_place(child, renames);
            }
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                apply_renames_to_place(&mut prop.value, renames);
                if let crate::hir::types::ObjectPropertyKey::Computed(key) = &mut prop.key {
                    apply_renames_to_place(key, renames);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Expression(p)
                    | crate::hir::types::ArrayElement::Spread(p) => {
                        apply_renames_to_place(p, renames);
                    }
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                apply_renames_to_place(sub, renames);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value: tpl } => {
            apply_renames_to_place(tag, renames);
            for sub in &mut tpl.subexpressions {
                apply_renames_to_place(sub, renames);
            }
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            apply_renames_to_place(lvalue, renames);
        }
        InstructionValue::StoreGlobal { value, .. } => {
            apply_renames_to_place(value, renames);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            apply_renames_to_place(decl, renames);
            for dep in deps {
                apply_renames_to_place(dep, renames);
            }
        }
        InstructionValue::Await { value } => {
            apply_renames_to_place(value, renames);
        }
        InstructionValue::GetIterator { collection } => {
            apply_renames_to_place(collection, renames);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            apply_renames_to_place(iterator, renames);
        }
        InstructionValue::NextPropertyOf { value } => {
            apply_renames_to_place(value, renames);
        }
        // No places to rename
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. } => {}
    }
}

/// Create a minimal instruction that represents `const originalName = tempName;`
/// This emits as a StoreLocal in codegen.
fn create_rename_assignment(
    original_name: &str,
    temp_name: &str,
) -> crate::hir::types::Instruction {
    use crate::hir::types::*;
    let zero_loc = SourceLocation::new(0, 0);
    let zero_range = MutableRange { start: InstructionId(0), end: InstructionId(0) };
    let make_place = |name: &str| Place {
        identifier: Identifier {
            id: IdentifierId(0),
            ssa_version: 0,
            declaration_id: None,
            name: Some(name.to_string()),
            type_: Type::Poly,
            mutable_range: zero_range,
            last_use: InstructionId(0),
            scope: None,
            loc: zero_loc,
        },
        effect: Effect::Unknown,
        reactive: false,
        loc: zero_loc,
    };
    // Create a minimal instruction: const originalName = tempName;
    Instruction {
        id: InstructionId(0),
        loc: zero_loc,
        lvalue: make_place(original_name),
        value: InstructionValue::StoreLocal {
            lvalue: make_place(original_name),
            value: make_place(temp_name),
            type_: Some(InstructionKind::Const),
        },
        effects: None,
    }
}

/// Prune hoisted contexts.
pub fn prune_hoisted_contexts(rf: &mut ReactiveFunction) {
    // Remove DeclareContext/StoreContext instructions that are not needed
    prune_hoisted_in_block(&mut rf.body);
}

fn prune_hoisted_in_block(block: &mut ReactiveBlock) {
    for instr in &mut block.instructions {
        match instr {
            ReactiveInstruction::Scope(scope_block) => {
                prune_hoisted_in_block(&mut scope_block.instructions);
            }
            ReactiveInstruction::Terminal(terminal) => {
                for_each_block_in_terminal_mut(terminal, prune_hoisted_in_block);
            }
            ReactiveInstruction::Instruction(_) => {}
        }
    }
}

/// Memoize fbt and macro operands in same scope.
pub fn memoize_fbt_and_macro_operands_in_same_scope(hir: &mut HIR) {
    // For fbt/macro calls, ensure operands are in the same reactive scope
    // This is specific to Meta's fbt internationalization framework
    let _ = hir;
}

/// Build reactive scope terminals in the HIR.
///
/// Converts scope annotations on identifiers into `Terminal::Scope` nodes in the CFG.
/// This splits blocks at scope boundaries and wraps scoped instructions so that
/// `build_reactive_function` can produce `ReactiveScopeBlock` nodes.
pub fn build_reactive_scope_terminals_hir(hir: &mut HIR) {
    // Step 1: Collect unique scope IDs and determine their block-position boundaries.
    // Instead of using instruction ID ranges (which break across SSA), we find the
    // first and last instruction positions within each block for each scope.
    let mut scope_ids: FxHashSet<ScopeId> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                scope_ids.insert(scope.id);
            }
        }
    }

    if scope_ids.is_empty() {
        return;
    }

    // Step 2: For each scope, find which block contains its annotated instructions
    // and compute position-based boundaries (first annotated pos to last annotated pos).
    let mut scope_info: Vec<(ScopeId, usize, usize, usize)> = Vec::new(); // (scope_id, block_idx, start_pos, end_pos)

    for &sid in &scope_ids {
        for (block_idx, (_, block)) in hir.blocks.iter().enumerate() {
            let first = block
                .instructions
                .iter()
                .position(|i| i.lvalue.identifier.scope.as_ref().is_some_and(|s| s.id == sid));
            if let Some(first_pos) = first {
                let last_pos = block
                    .instructions
                    .iter()
                    .rposition(|i| i.lvalue.identifier.scope.as_ref().is_some_and(|s| s.id == sid))
                    .unwrap_or(first_pos);
                scope_info.push((sid, block_idx, first_pos, last_pos + 1));
                break; // Only handle the first block containing this scope
            }
        }
    }

    // Sort innermost-first (smallest span first).
    scope_info.sort_by_key(|&(_, _, start, end)| end - start);

    // Allocate new BlockIds starting past the highest existing one.
    let mut next_block_id = hir.blocks.iter().map(|(id, _)| id.0).max().unwrap_or(0) + 1;

    // Step 3: For each scope, split the block at the position boundaries.
    for (scope_id, _block_idx, start_pos, end_pos) in scope_info {
        insert_scope_terminal_by_position(hir, scope_id, start_pos, end_pos, &mut next_block_id);
    }

    // Step 4: Clean up stale scope annotations from orphaned instructions.
    //
    // When a scope's annotated instructions span multiple basic blocks (e.g., after
    // flatten_scopes_with_hooks_or_use_hir splits scopes around hooks), only the
    // FIRST block's instructions get moved into the scope terminal block (Step 3).
    // Instructions in other blocks still carry the same scope ID annotation, which
    // causes propagate_scope_dependencies_hir to incorrectly declare them as scope
    // outputs — even though they're never computed inside the scope body at codegen.
    //
    // Collect scope IDs that have actual Terminal::Scope entries, and the block IDs
    // they point to. Then clear scope annotations from instructions that claim to
    // belong to a scope but are NOT in that scope's terminal block.
    let mut scope_to_block: FxHashMap<ScopeId, crate::hir::types::BlockId> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        if let Terminal::Scope { scope, block: scope_block, .. } = &block.terminal {
            scope_to_block.insert(*scope, *scope_block);
        }
    }

    for (block_id, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope
                && let Some(scope_block_id) = scope_to_block.get(&scope.id)
                && *block_id != *scope_block_id
            {
                // This instruction's scope has a terminal in a DIFFERENT block.
                // It's a stale annotation from a multi-block scope — clear it.
                instr.lvalue.identifier.scope = None;
            }
        }
    }
}

/// Insert a `Terminal::Scope` by splitting a block at position boundaries.
/// `start_pos` and `end_pos` are positions within the block's instruction vector.
fn insert_scope_terminal_by_position(
    hir: &mut HIR,
    scope_id: ScopeId,
    _start_pos: usize,
    _end_pos: usize,
    next_id: &mut u32,
) {
    // Re-find the block containing this scope (index may have shifted from prior insertions).
    let entry_idx = hir.blocks.iter().position(|(_, block)| {
        block
            .instructions
            .iter()
            .any(|i| i.lvalue.identifier.scope.as_ref().is_some_and(|s| s.id == scope_id))
    });

    let Some(entry_idx) = entry_idx else {
        return;
    };

    let block = &hir.blocks[entry_idx].1;

    // Re-compute positions since block may have been modified by prior scope insertions.
    let start_pos = block
        .instructions
        .iter()
        .position(|i| i.lvalue.identifier.scope.as_ref().is_some_and(|s| s.id == scope_id));

    let Some(start_pos) = start_pos else {
        return;
    };

    let end_pos = block
        .instructions
        .iter()
        .rposition(|i| i.lvalue.identifier.scope.as_ref().is_some_and(|s| s.id == scope_id))
        .map_or(start_pos + 1, |p| p + 1);

    // Partition the block into three segments:
    //   [0..start_pos)      = before scope (stays in original block)
    //   [start_pos..end_pos) = scope content (goes into new scope block)
    //   [end_pos..)          = after scope  (goes into fallthrough block)

    let original_block_id = hir.blocks[entry_idx].0;
    let original_terminal = hir.blocks[entry_idx].1.terminal.clone();
    let original_kind = hir.blocks[entry_idx].1.kind;

    let total_instrs = hir.blocks[entry_idx].1.instructions.len();
    let before_instrs = hir.blocks[entry_idx].1.instructions[..start_pos].to_vec();
    let mut scope_instrs = hir.blocks[entry_idx].1.instructions[start_pos..end_pos].to_vec();
    let after_instrs = hir.blocks[entry_idx].1.instructions[end_pos..].to_vec();

    // Annotate instructions in the scope block that lack any scope annotation.
    // Instructions between scope-annotated boundaries are included in the scope block
    // but may lack scope annotations (e.g., non-reactive instructions sandwiched between
    // reactive ones). Without annotation, propagate_scope_dependencies_hir won't recognize
    // their outputs as scope declarations, causing "not defined" errors when outputs are
    // used outside the scope.
    //
    // NOTE: Only annotate instructions with NO scope. Instructions with a DIFFERENT
    // scope ID are left as-is — they belong to a nested or adjacent scope whose
    // annotations are handled by Step 4's stale cleanup pass.
    let ref_scope = scope_instrs
        .iter()
        .find_map(|i| i.lvalue.identifier.scope.as_ref().filter(|s| s.id == scope_id).cloned());
    if let Some(ref_scope) = ref_scope {
        for instr in &mut scope_instrs {
            if instr.lvalue.identifier.scope.is_none() {
                instr.lvalue.identifier.scope = Some(Box::new(crate::hir::types::ReactiveScope {
                    id: scope_id,
                    range: ref_scope.range,
                    dependencies: Vec::new(),
                    declarations: Vec::new(),
                    reassignments: Vec::new(),
                    early_return_value: None,
                    merged: Vec::new(),
                    loc: ref_scope.loc,
                    is_allocating: false,
                }));
            }
        }
    }

    // When the scope covers all remaining instructions in the block (end_pos == total),
    // the original block's terminal (Ternary, If, Logical, etc.) may be part of the scope.
    // The scope block should inherit the terminal so that build_scope_block_only follows
    // the control flow branches. The fallthrough becomes the original terminal's
    // fallthrough (or a synthetic empty block for terminals without one).
    //
    // IMPORTANT: Only inherit terminals that represent control flow WITHIN the scope
    // (Ternary, If, Logical, Optional, etc.). Do NOT inherit Return/Throw/Goto — these
    // are exit terminals that should NOT be inside the scope body. A Return inside a
    // scope guard would make the cache store and else-branch unreachable (dead code).
    let scope_inherits_terminal = end_pos == total_instrs
        && after_instrs.is_empty()
        && matches!(
            &original_terminal,
            Terminal::If { .. }
                | Terminal::Ternary { .. }
                | Terminal::Logical { .. }
                | Terminal::Optional { .. }
                | Terminal::Sequence { .. }
                | Terminal::Switch { .. }
                | Terminal::For { .. }
                | Terminal::ForOf { .. }
                | Terminal::ForIn { .. }
                | Terminal::While { .. }
                | Terminal::DoWhile { .. }
                | Terminal::Try { .. }
                | Terminal::Label { .. }
                | Terminal::Branch { .. }
                | Terminal::MaybeThrow { .. }
        );

    // Allocate block IDs for new blocks.
    let scope_block_id = BlockId(*next_id);
    *next_id += 1;

    {
        let fallthrough_block_id = BlockId(*next_id);
        *next_id += 1;

        let (scope_terminal, fallthrough_terminal, scope_successors) = if scope_inherits_terminal {
            // Scope gets the original terminal; its control flow (ternary branches, etc.)
            // will be processed by build_scope_block_only. The fallthrough block gets
            // the original terminal's fallthrough target as a Goto, OR if the terminal
            // has no fallthrough (Return/Throw), we still create an empty fallthrough
            // block to satisfy the Scope terminal structure.
            let ft = terminal_fallthrough(&original_terminal);
            let successors = terminal_successors(&original_terminal);
            match ft {
                Some(ft_block) => {
                    (original_terminal, Terminal::Goto { block: ft_block }, successors)
                }
                None => {
                    // Terminal has no fallthrough (Return/Throw) — scope block
                    // keeps the terminal, fallthrough block is unreachable.
                    (
                        original_terminal,
                        Terminal::Goto { block: fallthrough_block_id }, // self-loop, unreachable
                        successors,
                    )
                }
            }
        } else {
            // Scope does NOT cover to end of block — use Goto to fallthrough,
            // and fallthrough gets remaining instructions + original terminal.
            let successors = terminal_successors(&original_terminal);
            (Terminal::Goto { block: fallthrough_block_id }, original_terminal, successors)
        };

        let scope_block = BasicBlock {
            kind: BlockKind::Block,
            id: scope_block_id,
            instructions: scope_instrs,
            terminal: scope_terminal,
            preds: vec![original_block_id],
            phis: Vec::new(),
        };

        let fallthrough_block = BasicBlock {
            kind: original_kind,
            id: fallthrough_block_id,
            instructions: after_instrs,
            terminal: fallthrough_terminal,
            preds: vec![scope_block_id],
            phis: Vec::new(),
        };

        // Original block keeps before-scope instructions + Scope terminal.
        hir.blocks[entry_idx].1.instructions = before_instrs;
        hir.blocks[entry_idx].1.terminal = Terminal::Scope {
            scope: scope_id,
            block: scope_block_id,
            fallthrough: fallthrough_block_id,
        };

        hir.blocks.push((scope_block_id, scope_block));
        hir.blocks.push((fallthrough_block_id, fallthrough_block));

        // Update predecessor lists: successors of whatever block now holds the
        // original terminal should reference the correct predecessor.
        let pred_for_successors =
            if scope_inherits_terminal { scope_block_id } else { fallthrough_block_id };
        for succ_id in scope_successors {
            if let Some((_, succ_block)) = hir.blocks.iter_mut().find(|(id, _)| *id == succ_id) {
                for pred in &mut succ_block.preds {
                    if *pred == original_block_id {
                        *pred = pred_for_successors;
                    }
                }
            }
        }
    }
}

/// Get the primary fallthrough successor of a terminal (the "next" block after it completes).
fn terminal_fallthrough(terminal: &Terminal) -> Option<BlockId> {
    match terminal {
        Terminal::Goto { block } => Some(*block),
        Terminal::If { fallthrough, .. } => Some(*fallthrough),
        Terminal::Branch { consequent, .. } => Some(*consequent),
        Terminal::Switch { fallthrough, .. } => Some(*fallthrough),
        Terminal::Return { .. } | Terminal::Throw { .. } | Terminal::Unreachable => None,
        Terminal::For { fallthrough, .. } => Some(*fallthrough),
        Terminal::ForOf { fallthrough, .. } => Some(*fallthrough),
        Terminal::ForIn { fallthrough, .. } => Some(*fallthrough),
        Terminal::DoWhile { fallthrough, .. } => Some(*fallthrough),
        Terminal::While { fallthrough, .. } => Some(*fallthrough),
        Terminal::Logical { fallthrough, .. } => Some(*fallthrough),
        Terminal::Ternary { fallthrough, .. } => Some(*fallthrough),
        Terminal::Optional { fallthrough, .. } => Some(*fallthrough),
        Terminal::Sequence { fallthrough, .. } => Some(*fallthrough),
        Terminal::Label { fallthrough, .. } => Some(*fallthrough),
        Terminal::MaybeThrow { continuation, .. } => Some(*continuation),
        Terminal::Try { fallthrough, .. } => Some(*fallthrough),
        Terminal::Scope { fallthrough, .. } => Some(*fallthrough),
        Terminal::PrunedScope { fallthrough, .. } => Some(*fallthrough),
    }
}

/// Get all successor block IDs from a terminal.
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
        Terminal::ForOf { init, test, body, fallthrough } => {
            vec![*init, *test, *body, *fallthrough]
        }
        Terminal::ForIn { init, test, body, fallthrough } => {
            vec![*init, *test, *body, *fallthrough]
        }
        Terminal::DoWhile { body, test, fallthrough } => vec![*body, *test, *fallthrough],
        Terminal::While { test, body, fallthrough } => vec![*test, *body, *fallthrough],
        Terminal::Logical { left, right, fallthrough, .. } => vec![*left, *right, *fallthrough],
        Terminal::Ternary { consequent, alternate, fallthrough, .. } => {
            vec![*consequent, *alternate, *fallthrough]
        }
        Terminal::Optional { consequent, fallthrough, .. } => vec![*consequent, *fallthrough],
        Terminal::Sequence { blocks, fallthrough, .. } => {
            let mut succs = blocks.clone();
            succs.push(*fallthrough);
            succs
        }
        Terminal::Label { block, fallthrough, .. } => vec![*block, *fallthrough],
        Terminal::MaybeThrow { continuation, handler, .. } => vec![*continuation, *handler],
        Terminal::Try { block, handler, fallthrough } => vec![*block, *handler, *fallthrough],
        Terminal::Scope { block, fallthrough, .. } => vec![*block, *fallthrough],
        Terminal::PrunedScope { block, fallthrough, .. } => vec![*block, *fallthrough],
    }
}

/// Flatten reactive loops in HIR.
pub fn flatten_reactive_loops_hir(hir: &mut HIR) {
    // Flatten scopes that span entire loop bodies — these can't be memoized
    // because the loop may execute a different number of times each render.

    // First pass: collect body block IDs from loop terminals
    let mut loop_body_blocks: Vec<crate::hir::types::BlockId> = Vec::new();

    for (_, block) in &hir.blocks {
        match &block.terminal {
            Terminal::For { body, .. }
            | Terminal::ForOf { body, .. }
            | Terminal::ForIn { body, .. }
            | Terminal::While { body, .. }
            | Terminal::DoWhile { body, .. } => {
                loop_body_blocks.push(*body);
            }
            _ => {}
        }
    }

    // Second pass: remove scope annotations from instructions in loop body blocks
    for (block_id, block) in &mut hir.blocks {
        if loop_body_blocks.contains(block_id) {
            for instr in &mut block.instructions {
                instr.lvalue.identifier.scope = None;
            }
        }
    }
}

/// Flatten scopes containing hooks or `use` in HIR.
/// Split reactive scopes that contain hook calls.
///
/// Hooks cannot be inside conditional scope guards (`if ($[0] !== dep) { useState(...) }`)
/// because that would violate the rules of hooks (called conditionally).
///
/// Instead of removing the entire scope (losing all memoization), we SPLIT the scope
/// around the hook call:
/// - Instructions before the hook → keep the original scope ID
/// - The hook call itself → remove scope annotation (will be unscoped)
/// - Instructions after the hook → get a new scope ID
///
/// This runs BEFORE `build_reactive_scope_terminals_hir`, so the split scopes
/// will be properly converted to `Terminal::Scope` structures later.
pub fn flatten_scopes_with_hooks_or_use_hir(hir: &mut HIR) {
    use crate::hir::globals::is_hook_name;
    use crate::hir::types::InstructionValue;

    // Build callee name map: instruction lvalue ID → callee name
    // (handles the LoadLocal inline where callee is a named place)
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadGlobal { binding } => {
                    id_to_name.insert(instr.lvalue.identifier.id, binding.name.clone());
                }
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    // Find scopes that contain hook calls, and the positions of hook instructions
    let mut scopes_with_hooks: FxHashMap<ScopeId, Vec<IdentifierId>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                let callee_name = callee
                    .identifier
                    .name
                    .as_deref()
                    .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));
                if callee_name.is_some_and(is_hook_name)
                    && let Some(ref scope) = instr.lvalue.identifier.scope
                {
                    scopes_with_hooks.entry(scope.id).or_default().push(instr.lvalue.identifier.id);
                }
            }
        }
    }

    if scopes_with_hooks.is_empty() {
        return;
    }

    // For each scope with hooks: remove scope annotation from hook instructions
    // AND from all instructions BEFORE the FIRST hook (since hook must run first
    // to establish state). Actually, the simpler approach: just remove the scope
    // from the hook instruction itself. The scope will become discontinuous,
    // which build_reactive_scope_terminals_hir handles by creating separate scopes.
    //
    // But wait — build_reactive_scope_terminals uses first/last position to create
    // ONE contiguous scope. A gap in the middle won't create two scopes.
    //
    // The correct approach: assign a NEW scope ID to instructions AFTER each hook,
    // so the scope naturally splits into two contiguous scopes.
    let mut next_scope_id = {
        let mut max_id = 0u32;
        for (_, block) in &hir.blocks {
            for instr in &block.instructions {
                if let Some(ref scope) = instr.lvalue.identifier.scope {
                    max_id = max_id.max(scope.id.0);
                }
            }
        }
        max_id + 1
    };

    for (scope_id, hook_ids) in &scopes_with_hooks {
        let hook_id_set: FxHashSet<IdentifierId> = hook_ids.iter().copied().collect();

        for (_, block) in &mut hir.blocks {
            // Track whether we've seen a hook in this scope within this block.
            // Once set, ALL subsequent instructions with this scope ID get
            // reassigned — even if there are non-scoped instructions in between.
            let mut past_hook = false;
            let mut current_new_scope_id = None;

            for instr in &mut block.instructions {
                let is_this_scope =
                    instr.lvalue.identifier.scope.as_ref().is_some_and(|s| s.id == *scope_id);

                if !is_this_scope {
                    // Don't reset past_hook — a gap in scope membership
                    // doesn't mean we haven't seen a hook yet.
                    continue;
                }

                if hook_id_set.contains(&instr.lvalue.identifier.id) {
                    // This IS the hook instruction — remove its scope
                    instr.lvalue.identifier.scope = None;
                    past_hook = true;
                    current_new_scope_id = None; // Will assign on next scoped instruction
                } else if past_hook {
                    // After a hook — assign a new scope ID so it's a separate scope
                    if current_new_scope_id.is_none() {
                        current_new_scope_id = Some(ScopeId(next_scope_id));
                        next_scope_id += 1;
                    }
                    if let Some(ref mut scope) = instr.lvalue.identifier.scope {
                        // Clone the scope with a new ID
                        let new_id = current_new_scope_id.unwrap();
                        **scope = ReactiveScope {
                            id: new_id,
                            range: scope.range,
                            dependencies: Vec::new(),
                            declarations: Vec::new(),
                            reassignments: Vec::new(),
                            early_return_value: None,
                            merged: Vec::new(),
                            loc: scope.loc,
                            is_allocating: scope.is_allocating,
                        };
                    }
                }
                // else: before any hook — keep the original scope ID
            }
        }
    }
}

// --- Helper: iterate over all sub-blocks of a terminal ---

fn for_each_block_in_terminal(terminal: &ReactiveTerminal, mut f: impl FnMut(&ReactiveBlock)) {
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            f(consequent);
            f(alternate);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                f(block);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            f(init);
            f(test);
            if let Some(upd) = update {
                f(upd);
            }
            f(body);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            f(init);
            f(test);
            f(body);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            f(test);
            f(body);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            f(block);
            f(handler);
        }
        ReactiveTerminal::Label { block, .. } => {
            f(block);
        }
        ReactiveTerminal::Logical { right, .. } => {
            f(right);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
    }
}

fn for_each_block_in_terminal_mut(
    terminal: &mut ReactiveTerminal,
    mut f: impl FnMut(&mut ReactiveBlock),
) {
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            f(consequent);
            f(alternate);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                f(block);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            f(init);
            f(test);
            if let Some(upd) = update {
                f(upd);
            }
            f(body);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            f(init);
            f(test);
            f(body);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            f(test);
            f(body);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            f(block);
            f(handler);
        }
        ReactiveTerminal::Label { block, .. } => {
            f(block);
        }
        ReactiveTerminal::Logical { right, .. } => {
            f(right);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
    }
}
