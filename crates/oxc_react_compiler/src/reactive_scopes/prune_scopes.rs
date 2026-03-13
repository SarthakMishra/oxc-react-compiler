#![allow(dead_code)]

use crate::hir::types::{
    ArrayElement, BasicBlock, BlockId, BlockKind, HIR, IdentifierId, InstructionValue,
    ObjectPropertyKey, Place, ReactiveBlock, ReactiveFunction, ReactiveInstruction,
    ReactiveTerminal, ScopeId, Terminal,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// Prune reactive scopes that don't escape the function.
pub fn prune_non_escaping_scopes(rf: &mut ReactiveFunction) {
    // Collect all identifier IDs used outside of scopes
    let mut used_outside_scopes = FxHashSet::default();
    collect_used_outside_scopes(&rf.body, false, &mut used_outside_scopes);

    // Remove scopes whose declarations are never used outside
    prune_scopes_in_block(&mut rf.body, &used_outside_scopes);
}

fn collect_used_outside_scopes(
    block: &ReactiveBlock,
    in_scope: bool,
    used: &mut FxHashSet<IdentifierId>,
) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                if !in_scope {
                    // Collect all operand IDs used outside scopes
                    collect_instruction_operand_ids(&instruction.value, used);
                }
            }
            ReactiveInstruction::Scope(scope_block) => {
                collect_used_outside_scopes(&scope_block.instructions, true, used);
            }
            ReactiveInstruction::Terminal(terminal) => {
                collect_used_in_terminal(terminal, in_scope, used);
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
        InstructionValue::CallExpression { callee, args }
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
        InstructionValue::ComputedLoad { object, property } => {
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

fn collect_used_in_terminal(
    terminal: &ReactiveTerminal,
    in_scope: bool,
    used: &mut FxHashSet<IdentifierId>,
) {
    match terminal {
        ReactiveTerminal::Return { value, .. } | ReactiveTerminal::Throw { value, .. } => {
            if !in_scope {
                used.insert(value.identifier.id);
            }
        }
        ReactiveTerminal::If { test, consequent, alternate, .. } => {
            if !in_scope {
                used.insert(test.identifier.id);
            }
            collect_used_outside_scopes(consequent, in_scope, used);
            collect_used_outside_scopes(alternate, in_scope, used);
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            if !in_scope {
                used.insert(test.identifier.id);
            }
            for (_, block) in cases {
                collect_used_outside_scopes(block, in_scope, used);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            collect_used_outside_scopes(init, in_scope, used);
            collect_used_outside_scopes(test, in_scope, used);
            if let Some(upd) = update {
                collect_used_outside_scopes(upd, in_scope, used);
            }
            collect_used_outside_scopes(body, in_scope, used);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            collect_used_outside_scopes(init, in_scope, used);
            collect_used_outside_scopes(test, in_scope, used);
            collect_used_outside_scopes(body, in_scope, used);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            collect_used_outside_scopes(test, in_scope, used);
            collect_used_outside_scopes(body, in_scope, used);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            collect_used_outside_scopes(block, in_scope, used);
            collect_used_outside_scopes(handler, in_scope, used);
        }
        ReactiveTerminal::Label { block, .. } => {
            collect_used_outside_scopes(block, in_scope, used);
        }
    }
}

fn prune_scopes_in_block(block: &mut ReactiveBlock, used_outside: &FxHashSet<IdentifierId>) {
    let mut new_instructions = Vec::new();

    for instr in std::mem::take(&mut block.instructions) {
        match instr {
            ReactiveInstruction::Scope(mut scope_block) => {
                // Check if any declaration of this scope is used outside
                let any_decl_used =
                    scope_block.scope.declarations.iter().any(|(id, _)| used_outside.contains(id));
                // Also check if any reassignment target is used outside
                // (upstream checks both declarations and reassignments)
                let any_reassign_used = scope_block
                    .scope
                    .reassignments
                    .iter()
                    .any(|ident| used_outside.contains(&ident.id));

                prune_scopes_in_block(&mut scope_block.instructions, used_outside);

                // Keep if: any declaration or reassignment escapes, OR empty declarations
                // (handled by PropagateEarlyReturns later), OR is allocating/sentinel scope,
                // OR has an early return value.
                if any_decl_used
                    || any_reassign_used
                    || (scope_block.scope.declarations.is_empty()
                        && scope_block.scope.reassignments.is_empty())
                    || scope_block.scope.is_allocating
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
                prune_scopes_in_terminal(&mut terminal, used_outside);
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
    used_outside: &FxHashSet<IdentifierId>,
) {
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            prune_scopes_in_block(consequent, used_outside);
            prune_scopes_in_block(alternate, used_outside);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                prune_scopes_in_block(block, used_outside);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            prune_scopes_in_block(init, used_outside);
            prune_scopes_in_block(test, used_outside);
            if let Some(upd) = update {
                prune_scopes_in_block(upd, used_outside);
            }
            prune_scopes_in_block(body, used_outside);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            prune_scopes_in_block(init, used_outside);
            prune_scopes_in_block(test, used_outside);
            prune_scopes_in_block(body, used_outside);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            prune_scopes_in_block(test, used_outside);
            prune_scopes_in_block(body, used_outside);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            prune_scopes_in_block(block, used_outside);
            prune_scopes_in_block(handler, used_outside);
        }
        ReactiveTerminal::Label { block, .. } => {
            prune_scopes_in_block(block, used_outside);
        }
        ReactiveTerminal::Return { .. } | ReactiveTerminal::Throw { .. } => {}
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
    // is an unnamed temp, map lvalue.id → source place
    let mut substitutions: FxHashMap<IdentifierId, Place> = FxHashMap::default();

    for instr in &block.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr
            && let InstructionValue::LoadLocal { place: source } = &instruction.value
        {
            let lvalue = &instruction.lvalue;
            // Only inline unnamed temporaries (not user-declared variables)
            if lvalue.identifier.name.is_none() {
                substitutions.insert(lvalue.identifier.id, source.clone());
            }
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
        | ReactiveTerminal::Label { .. } => {}
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
        InstructionValue::CallExpression { callee, args } => {
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
        InstructionValue::ComputedLoad { object, property } => {
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
        | ReactiveTerminal::Try { .. } => {
            // These terminals have blocks but no direct Place fields
            // (blocks are walked by for_each_block_in_terminal_mut)
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
        InstructionValue::CallExpression { callee, args } => {
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
        InstructionValue::ComputedLoad { object, property } => {
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
        | ReactiveTerminal::Return { id, .. }
        | ReactiveTerminal::Throw { id, .. } => {
            *id = new_id;
        }
    }
}

/// Rename variables for clean output.
pub fn rename_variables(rf: &mut ReactiveFunction) {
    // For now, this is handled by promote_used_temporaries
    let _ = rf;
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

    let before_instrs = hir.blocks[entry_idx].1.instructions[..start_pos].to_vec();
    let scope_instrs = hir.blocks[entry_idx].1.instructions[start_pos..end_pos].to_vec();
    let after_instrs = hir.blocks[entry_idx].1.instructions[end_pos..].to_vec();

    // Allocate block IDs for new blocks.
    let scope_block_id = BlockId(*next_id);
    *next_id += 1;

    // Always create a fallthrough block. The scope block uses Goto to the fallthrough,
    // and the fallthrough gets the after-scope instructions + original terminal.
    // This avoids self-referential fallthrough when the original terminal is Return/Throw.
    {
        let fallthrough_block_id = BlockId(*next_id);
        *next_id += 1;

        // Scope block: holds the scope content, falls through to fallthrough block.
        let scope_block = BasicBlock {
            kind: BlockKind::Block,
            id: scope_block_id,
            instructions: scope_instrs,
            terminal: Terminal::Goto { block: fallthrough_block_id },
            preds: vec![original_block_id],
            phis: Vec::new(),
        };

        // Fallthrough block: holds after-scope instructions + original terminal.
        let fallthrough_block = BasicBlock {
            kind: original_kind,
            id: fallthrough_block_id,
            instructions: after_instrs,
            terminal: original_terminal.clone(),
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

        // Update predecessor lists for successors of the fallthrough block.
        let successors = terminal_successors(&original_terminal);
        hir.blocks.push((scope_block_id, scope_block));
        hir.blocks.push((fallthrough_block_id, fallthrough_block));

        for succ_id in successors {
            if let Some((_, succ_block)) = hir.blocks.iter_mut().find(|(id, _)| *id == succ_id) {
                for pred in &mut succ_block.preds {
                    if *pred == original_block_id {
                        *pred = fallthrough_block_id;
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
        Terminal::MaybeThrow { continuation, handler } => vec![*continuation, *handler],
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
pub fn flatten_scopes_with_hooks_or_use_hir(hir: &mut HIR) {
    use crate::hir::globals::is_hook_name;

    // Find scopes that contain hook calls
    let mut scopes_with_hooks: FxHashSet<ScopeId> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let crate::hir::types::InstructionValue::CallExpression { callee, .. } = &instr.value
                && let Some(name) = &callee.identifier.name
                && is_hook_name(name)
                && let Some(ref scope) = instr.lvalue.identifier.scope
            {
                scopes_with_hooks.insert(scope.id);
            }
        }
    }

    // Remove scope annotations for scopes containing hooks
    if scopes_with_hooks.is_empty() {
        return;
    }

    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope
                && scopes_with_hooks.contains(&scope.id)
            {
                instr.lvalue.identifier.scope = None;
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
        ReactiveTerminal::Return { .. } | ReactiveTerminal::Throw { .. } => {}
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
        ReactiveTerminal::Return { .. } | ReactiveTerminal::Throw { .. } => {}
    }
}
