
use crate::hir::types::{
    BasicBlock, BlockId, HIR, Param, ReactiveBlock, ReactiveFunction, ReactiveInstruction,
    ReactiveScopeBlock, ReactiveTerminal, SourceLocation, Terminal,
};

/// Convert HIR CFG to ReactiveFunction tree.
///
/// This is the key transformation from CFG (with explicit blocks and gotos)
/// to a tree-shaped IR (with nested blocks for control flow).
///
/// Algorithm:
/// 1. Start from the entry block
/// 2. For each block, emit its instructions as ReactiveInstructions
/// 3. When encountering a terminal:
///    - Goto: continue with the target block
///    - If/Switch/Loop: recursively process branches, creating nested ReactiveBlocks
///    - Scope: wrap the scope block in a ReactiveScopeBlock
///    - Return/Throw: emit the terminal
/// 4. Handle scope terminals by wrapping blocks in ReactiveScopeBlock
pub fn build_reactive_function(
    hir: HIR,
    params: Vec<Param>,
    id: Option<String>,
    loc: SourceLocation,
    directives: Vec<String>,
) -> ReactiveFunction {
    let body = build_reactive_block(&hir, hir.entry);

    ReactiveFunction { loc, id, params, body, directives }
}

/// Public entry point for converting an HIR body to a ReactiveBlock.
/// Used by codegen for nested function expressions.
pub fn build_reactive_block_from_hir(hir: &HIR, start_block: BlockId) -> ReactiveBlock {
    build_reactive_block(hir, start_block)
}

fn find_block(hir: &HIR, block_id: BlockId) -> Option<&BasicBlock> {
    hir.blocks.iter().find(|(id, _)| *id == block_id).map(|(_, block)| block)
}

fn build_reactive_block(hir: &HIR, start_block: BlockId) -> ReactiveBlock {
    build_reactive_block_until(hir, start_block, None)
}

/// Build a reactive block from the HIR, optionally stopping when a Goto
/// targets `stop_at`. This prevents duplication when If/Ternary branches
/// both Goto the same fallthrough block.
fn build_reactive_block_until(
    hir: &HIR,
    start_block: BlockId,
    stop_at: Option<BlockId>,
) -> ReactiveBlock {
    let mut instructions = Vec::new();
    let mut current = start_block;

    loop {
        let block = match find_block(hir, current) {
            Some(block) => block,
            None => break,
        };

        // Emit instructions
        for instr in &block.instructions {
            instructions.push(ReactiveInstruction::Instruction(instr.clone()));
        }

        // Process terminal
        match &block.terminal {
            Terminal::Goto { block: next } => {
                // Stop if Goto targets the stop block (fallthrough)
                if stop_at == Some(*next) {
                    break;
                }
                current = *next;
                continue;
            }
            Terminal::If { test, consequent, alternate, fallthrough } => {
                let consequent_block = build_reactive_block_until(hir, *consequent, Some(*fallthrough));
                let alternate_block = build_reactive_block_until(hir, *alternate, Some(*fallthrough));
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::If {
                    test: test.clone(),
                    consequent: consequent_block,
                    alternate: alternate_block,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::Return { value } => {
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Return {
                    value: value.clone(),
                    id: current,
                }));
                break;
            }
            Terminal::Throw { value } => {
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Throw {
                    value: value.clone(),
                    id: current,
                }));
                break;
            }
            Terminal::Switch { test, cases, fallthrough } => {
                let reactive_cases: Vec<(Option<crate::hir::types::Place>, ReactiveBlock)> = cases
                    .iter()
                    .map(|case| {
                        let block = build_reactive_block(hir, case.block);
                        (case.test.clone(), block)
                    })
                    .collect();
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Switch {
                    test: test.clone(),
                    cases: reactive_cases,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::For { init, test, update, body, fallthrough } => {
                let init_block = build_reactive_block(hir, *init);
                let test_block = build_reactive_block(hir, *test);
                let update_block = update.map(|u| build_reactive_block(hir, u));
                let body_block = build_reactive_block(hir, *body);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::For {
                    init: init_block,
                    test: test_block,
                    update: update_block,
                    body: body_block,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::While { test, body, fallthrough } => {
                let test_block = build_reactive_block(hir, *test);
                let body_block = build_reactive_block(hir, *body);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::While {
                    test: test_block,
                    body: body_block,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::DoWhile { body, test, fallthrough } => {
                let body_block = build_reactive_block(hir, *body);
                let test_block = build_reactive_block(hir, *test);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::DoWhile {
                    body: body_block,
                    test: test_block,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::ForOf { init, test, body, fallthrough } => {
                let init_block = build_reactive_block(hir, *init);
                let test_block = build_reactive_block(hir, *test);
                let body_block = build_reactive_block(hir, *body);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::ForOf {
                    init: init_block,
                    test: test_block,
                    body: body_block,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::ForIn { init, test, body, fallthrough } => {
                let init_block = build_reactive_block(hir, *init);
                let test_block = build_reactive_block(hir, *test);
                let body_block = build_reactive_block(hir, *body);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::ForIn {
                    init: init_block,
                    test: test_block,
                    body: body_block,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::Try { block: try_block, handler, fallthrough } => {
                let try_reactive = build_reactive_block(hir, *try_block);
                let handler_reactive = build_reactive_block(hir, *handler);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Try {
                    block: try_reactive,
                    handler: handler_reactive,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::Label { block: label_block, fallthrough, label } => {
                let label_reactive = build_reactive_block(hir, *label_block);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Label {
                    block: label_reactive,
                    id: current,
                    label: *label,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::Scope { block: scope_block, fallthrough, scope } => {
                // Build only the scope block's instructions without following its Goto terminal.
                // The scope block's Goto points to fallthrough, which we process separately.
                let scope_reactive = build_scope_block_only(hir, *scope_block);

                // Try to find the ReactiveScope from the block's instructions
                // by looking at the scope ID
                let reactive_scope = find_scope_in_block(hir, *scope_block, *scope);

                if let Some(rs) = reactive_scope {
                    instructions.push(ReactiveInstruction::Scope(ReactiveScopeBlock {
                        scope: rs,
                        instructions: scope_reactive,
                    }));
                } else {
                    // Fallback: emit instructions without scope wrapping
                    for instr in scope_reactive.instructions {
                        instructions.push(instr);
                    }
                }
                current = *fallthrough;
                continue;
            }
            Terminal::PrunedScope { fallthrough, .. } => {
                // Pruned scopes are skipped
                current = *fallthrough;
                continue;
            }
            Terminal::Ternary { test, consequent, alternate, fallthrough } => {
                // Lower ternary to If — same CFG structure, phi nodes already resolved
                let consequent_block = build_reactive_block_until(hir, *consequent, Some(*fallthrough));
                let alternate_block = build_reactive_block_until(hir, *alternate, Some(*fallthrough));
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::If {
                    test: test.clone(),
                    consequent: consequent_block,
                    alternate: alternate_block,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::Logical { right, fallthrough, .. } => {
                // Inline the right-side block. Short-circuit semantics are not
                // preserved, but the React compiler's memoization model evaluates
                // both paths anyway (Babel does the same).
                let right_block = build_reactive_block_until(hir, *right, Some(*fallthrough));
                instructions.extend(right_block.instructions);
                current = *fallthrough;
                continue;
            }
            Terminal::Optional { consequent, fallthrough, .. } => {
                // Inline consequent block (optional chain continuation)
                let cons_block = build_reactive_block_until(hir, *consequent, Some(*fallthrough));
                instructions.extend(cons_block.instructions);
                current = *fallthrough;
                continue;
            }
            Terminal::Sequence { blocks, fallthrough } => {
                // Process all sequence blocks in order
                for block_id in blocks {
                    let block = build_reactive_block_until(hir, *block_id, Some(*fallthrough));
                    instructions.extend(block.instructions);
                }
                current = *fallthrough;
                continue;
            }
            Terminal::Branch { test, consequent, alternate } => {
                // Lower Branch to If (Branch has no fallthrough — it's terminal)
                let cons_block = build_reactive_block(hir, *consequent);
                let alt_block = build_reactive_block(hir, *alternate);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::If {
                    test: test.clone(),
                    consequent: cons_block,
                    alternate: alt_block,
                    id: current,
                }));
                break;
            }
            Terminal::MaybeThrow { continuation, .. } => {
                // Follow continuation path
                current = *continuation;
                continue;
            }
            Terminal::Unreachable => {
                break;
            }
        }
    }

    ReactiveBlock { instructions }
}

/// Build only the instructions from a single scope block, without following Goto terminals.
/// This prevents duplication when the scope block's Goto leads to the fallthrough block.
fn build_scope_block_only(hir: &HIR, block_id: BlockId) -> ReactiveBlock {
    let mut instructions = Vec::new();

    if let Some(block) = find_block(hir, block_id) {
        for instr in &block.instructions {
            instructions.push(ReactiveInstruction::Instruction(instr.clone()));
        }

        // Process the terminal, but don't follow Goto (that's the fallthrough).
        // Other terminals (If, Switch, etc.) within the scope are processed normally.
        match &block.terminal {
            Terminal::Goto { .. } => {
                // Don't follow — fallthrough is handled by the caller
            }
            Terminal::If { test, consequent, alternate, fallthrough } => {
                let consequent_block = build_reactive_block(hir, *consequent);
                let alternate_block = build_reactive_block(hir, *alternate);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::If {
                    test: test.clone(),
                    consequent: consequent_block,
                    alternate: alternate_block,
                    id: block_id,
                }));
                // Continue with fallthrough within the scope
                let remaining = build_scope_block_only(hir, *fallthrough);
                instructions.extend(remaining.instructions);
            }
            Terminal::Return { value } => {
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Return {
                    value: value.clone(),
                    id: block_id,
                }));
            }
            Terminal::Scope { block: scope_block, fallthrough, scope } => {
                // Nested scope within scope
                let scope_reactive = build_scope_block_only(hir, *scope_block);
                let reactive_scope = find_scope_in_block(hir, *scope_block, *scope);
                if let Some(rs) = reactive_scope {
                    instructions.push(ReactiveInstruction::Scope(ReactiveScopeBlock {
                        scope: rs,
                        instructions: scope_reactive,
                    }));
                } else {
                    instructions.extend(scope_reactive.instructions);
                }
                let remaining = build_scope_block_only(hir, *fallthrough);
                instructions.extend(remaining.instructions);
            }
            Terminal::Ternary { test, consequent, alternate, fallthrough } => {
                let consequent_block = build_reactive_block(hir, *consequent);
                let alternate_block = build_reactive_block(hir, *alternate);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::If {
                    test: test.clone(),
                    consequent: consequent_block,
                    alternate: alternate_block,
                    id: block_id,
                }));
                let remaining = build_scope_block_only(hir, *fallthrough);
                instructions.extend(remaining.instructions);
            }
            Terminal::Logical { right, fallthrough, .. } => {
                let right_block = build_reactive_block(hir, *right);
                instructions.extend(right_block.instructions);
                let remaining = build_scope_block_only(hir, *fallthrough);
                instructions.extend(remaining.instructions);
            }
            Terminal::Optional { consequent, fallthrough, .. } => {
                let cons_block = build_reactive_block(hir, *consequent);
                instructions.extend(cons_block.instructions);
                let remaining = build_scope_block_only(hir, *fallthrough);
                instructions.extend(remaining.instructions);
            }
            Terminal::Sequence { blocks, fallthrough } => {
                for bid in blocks {
                    let block = build_reactive_block(hir, *bid);
                    instructions.extend(block.instructions);
                }
                let remaining = build_scope_block_only(hir, *fallthrough);
                instructions.extend(remaining.instructions);
            }
            Terminal::MaybeThrow { continuation, .. } => {
                let remaining = build_scope_block_only(hir, *continuation);
                instructions.extend(remaining.instructions);
            }
            Terminal::Switch { test, cases, fallthrough } => {
                let reactive_cases: Vec<(Option<crate::hir::types::Place>, ReactiveBlock)> = cases
                    .iter()
                    .map(|case| {
                        let block = build_reactive_block(hir, case.block);
                        (case.test.clone(), block)
                    })
                    .collect();
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Switch {
                    test: test.clone(),
                    cases: reactive_cases,
                    id: block_id,
                }));
                let remaining = build_scope_block_only(hir, *fallthrough);
                instructions.extend(remaining.instructions);
            }
            Terminal::Throw { value } => {
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Throw {
                    value: value.clone(),
                    id: block_id,
                }));
            }
            Terminal::Branch { test, consequent, alternate } => {
                let cons_block = build_reactive_block(hir, *consequent);
                let alt_block = build_reactive_block(hir, *alternate);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::If {
                    test: test.clone(),
                    consequent: cons_block,
                    alternate: alt_block,
                    id: block_id,
                }));
            }
            _ => {
                // Unreachable, PrunedScope, loops, etc. — no special handling needed in scope blocks
            }
        }
    }

    ReactiveBlock { instructions }
}

/// Try to find a ReactiveScope from the instructions within a scope block.
fn find_scope_in_block(
    hir: &HIR,
    block_id: BlockId,
    scope_id: crate::hir::types::ScopeId,
) -> Option<crate::hir::types::ReactiveScope> {
    if let Some(block) = find_block(hir, block_id) {
        for instr in &block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope
                && scope.id == scope_id {
                    return Some(scope.as_ref().clone());
                }
        }
    }
    None
}
