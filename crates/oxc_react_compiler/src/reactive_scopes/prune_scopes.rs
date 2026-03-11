#![allow(dead_code)]

use crate::hir::types::{
    IdentifierId, ReactiveBlock, ReactiveFunction, ReactiveInstruction, ReactiveTerminal, ScopeId,
    Terminal, HIR,
};
use rustc_hash::FxHashSet;

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

fn collect_instruction_operand_ids(
    value: &crate::hir::types::InstructionValue,
    used: &mut FxHashSet<IdentifierId>,
) {
    use crate::hir::types::InstructionValue;
    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            used.insert(place.identifier.id);
        }
        InstructionValue::CallExpression { callee, args } => {
            used.insert(callee.identifier.id);
            for arg in args {
                used.insert(arg.identifier.id);
            }
        }
        InstructionValue::PropertyLoad { object, .. } => {
            used.insert(object.identifier.id);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            used.insert(left.identifier.id);
            used.insert(right.identifier.id);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            used.insert(value.identifier.id);
        }
        InstructionValue::JsxExpression {
            tag,
            props,
            children,
        } => {
            used.insert(tag.identifier.id);
            for attr in props {
                used.insert(attr.value.identifier.id);
            }
            for child in children {
                used.insert(child.identifier.id);
            }
        }
        _ => {
            // For other instruction types, a full operand walk would be ideal
            // but this covers the most common cases
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
        ReactiveTerminal::If {
            test,
            consequent,
            alternate,
            ..
        } => {
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
        ReactiveTerminal::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            collect_used_outside_scopes(init, in_scope, used);
            collect_used_outside_scopes(test, in_scope, used);
            if let Some(upd) = update {
                collect_used_outside_scopes(upd, in_scope, used);
            }
            collect_used_outside_scopes(body, in_scope, used);
        }
        ReactiveTerminal::ForOf {
            init, test, body, ..
        }
        | ReactiveTerminal::ForIn {
            init, test, body, ..
        } => {
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
                let any_used = scope_block
                    .scope
                    .declarations
                    .iter()
                    .any(|(id, _)| used_outside.contains(id));

                prune_scopes_in_block(&mut scope_block.instructions, used_outside);

                if any_used || scope_block.scope.declarations.is_empty() {
                    // Keep the scope
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
        ReactiveTerminal::If {
            consequent,
            alternate,
            ..
        } => {
            prune_scopes_in_block(consequent, used_outside);
            prune_scopes_in_block(alternate, used_outside);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                prune_scopes_in_block(block, used_outside);
            }
        }
        ReactiveTerminal::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            prune_scopes_in_block(init, used_outside);
            prune_scopes_in_block(test, used_outside);
            if let Some(upd) = update {
                prune_scopes_in_block(upd, used_outside);
            }
            prune_scopes_in_block(body, used_outside);
        }
        ReactiveTerminal::ForOf {
            init, test, body, ..
        }
        | ReactiveTerminal::ForIn {
            init, test, body, ..
        } => {
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
                // Remove non-reactive dependencies
                scope_block.scope.dependencies.retain(|dep| dep.reactive);
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

                let has_used_decls = scope_block
                    .scope
                    .declarations
                    .iter()
                    .any(|(id, _)| referenced.contains(id));

                if has_used_decls || scope_block.scope.declarations.is_empty() {
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
pub fn prune_always_invalidating_scopes(rf: &mut ReactiveFunction) {
    prune_always_invalidating_in_block(&mut rf.body);
}

fn prune_always_invalidating_in_block(block: &mut ReactiveBlock) {
    let mut new_instructions = Vec::new();

    for instr in std::mem::take(&mut block.instructions) {
        match instr {
            ReactiveInstruction::Scope(mut scope_block) => {
                prune_always_invalidating_in_block(&mut scope_block.instructions);

                // A scope always invalidates if it has a dependency on a value
                // that is freshly created each render (e.g., an object literal
                // or function expression outside any scope).
                // For now, we keep all scopes — a full implementation would
                // track value provenance.
                new_instructions.push(ReactiveInstruction::Scope(scope_block));
            }
            ReactiveInstruction::Terminal(mut terminal) => {
                for_each_block_in_terminal_mut(&mut terminal, prune_always_invalidating_in_block);
                new_instructions.push(ReactiveInstruction::Terminal(terminal));
            }
            other => {
                new_instructions.push(other);
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
                if instruction.lvalue.identifier.name.is_none() {
                    instruction.lvalue.identifier.name =
                        Some(format!("t{}", instruction.lvalue.identifier.id.0));
                }
            }
            ReactiveInstruction::Scope(scope_block) => {
                promote_temps_in_block(&mut scope_block.instructions, counter);
            }
            ReactiveInstruction::Terminal(terminal) => {
                for_each_block_in_terminal_mut(terminal, |block| {
                    promote_temps_in_block(block, counter);
                });
            }
        }
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
pub fn build_reactive_scope_terminals_hir(hir: &mut HIR) {
    // Collect all scopes and their ranges
    let mut scope_ranges: Vec<(ScopeId, u32, u32)> = Vec::new();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                let already = scope_ranges.iter().any(|(id, _, _)| *id == scope.id);
                if !already {
                    scope_ranges.push((scope.id, scope.range.start.0, scope.range.end.0));
                }
            }
        }
    }

    // For each scope, find the block it starts in and insert a Scope terminal
    // This is a simplified version — the full implementation would split blocks
    // at scope boundaries.
    let _ = scope_ranges;
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
            {
                if let Some(name) = &callee.identifier.name {
                    if is_hook_name(name) {
                        if let Some(ref scope) = instr.lvalue.identifier.scope {
                            scopes_with_hooks.insert(scope.id);
                        }
                    }
                }
            }
        }
    }

    // Remove scope annotations for scopes containing hooks
    if scopes_with_hooks.is_empty() {
        return;
    }

    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                if scopes_with_hooks.contains(&scope.id) {
                    instr.lvalue.identifier.scope = None;
                }
            }
        }
    }
}

// --- Helper: iterate over all sub-blocks of a terminal ---

fn for_each_block_in_terminal(terminal: &ReactiveTerminal, mut f: impl FnMut(&ReactiveBlock)) {
    match terminal {
        ReactiveTerminal::If {
            consequent,
            alternate,
            ..
        } => {
            f(consequent);
            f(alternate);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                f(block);
            }
        }
        ReactiveTerminal::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            f(init);
            f(test);
            if let Some(upd) = update {
                f(upd);
            }
            f(body);
        }
        ReactiveTerminal::ForOf {
            init, test, body, ..
        }
        | ReactiveTerminal::ForIn {
            init, test, body, ..
        } => {
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
        ReactiveTerminal::If {
            consequent,
            alternate,
            ..
        } => {
            f(consequent);
            f(alternate);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                f(block);
            }
        }
        ReactiveTerminal::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            f(init);
            f(test);
            if let Some(upd) = update {
                f(upd);
            }
            f(body);
        }
        ReactiveTerminal::ForOf {
            init, test, body, ..
        }
        | ReactiveTerminal::ForIn {
            init, test, body, ..
        } => {
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
