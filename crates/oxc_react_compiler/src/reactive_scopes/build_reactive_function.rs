use crate::hir::types::{
    BasicBlock, BlockId, HIR, Param, ReactiveBlock, ReactiveFunction, ReactiveInstruction,
    ReactiveScopeBlock, ReactiveTerminal, SourceLocation, Terminal,
};
use rustc_hash::FxHashSet;

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
    is_arrow: bool,
    is_async: bool,
    is_generator: bool,
) -> ReactiveFunction {
    let mut visited = FxHashSet::default();
    let body = build_reactive_block_until(&hir, hir.entry, None, &mut visited, None);

    ReactiveFunction { loc, id, params, body, directives, is_arrow, is_async, is_generator }
}

/// Public entry point for converting an HIR body to a ReactiveBlock.
/// Used by codegen for nested function expressions.
pub fn build_reactive_block_from_hir(hir: &HIR, start_block: BlockId) -> ReactiveBlock {
    let mut visited = FxHashSet::default();
    build_reactive_block_until(hir, start_block, None, &mut visited, None)
}

fn find_block(hir: &HIR, block_id: BlockId) -> Option<&BasicBlock> {
    hir.blocks.iter().find(|(id, _)| *id == block_id).map(|(_, block)| block)
}

/// Loop context for detecting `continue` and `break` gotos inside loop bodies.
/// `continue_target` is the block ID that a `continue` statement jumps to (typically
/// the loop's test block). `break_target` is the loop's fallthrough block.
#[derive(Clone, Copy)]
struct LoopContext {
    continue_target: BlockId,
    break_target: BlockId,
}

/// Build a reactive block from the HIR, optionally stopping when a Goto
/// targets `stop_at`. This prevents duplication when If/Ternary branches
/// both Goto the same fallthrough block.
///
/// The `visited` set is shared across ALL recursive calls to prevent
/// exponential blowup from loop back-edges. When following a Goto that
/// leads back to an already-visited block (e.g., a loop header), we stop
/// instead of re-processing the entire loop body.
///
/// `loop_ctx` tracks the innermost enclosing loop's continue/break targets.
/// When a Goto inside a loop body targets these blocks, we emit explicit
/// `Continue` or `Break` reactive terminals instead of silently dropping them.
fn build_reactive_block_until(
    hir: &HIR,
    start_block: BlockId,
    stop_at: Option<BlockId>,
    visited: &mut FxHashSet<BlockId>,
    loop_ctx: Option<LoopContext>,
) -> ReactiveBlock {
    let mut instructions = Vec::new();
    let mut current = start_block;

    loop {
        // Prevent infinite loops from cyclic Goto chains and loop back-edges.
        // Each block is processed at most once across the entire tree construction.
        if !visited.insert(current) {
            // Check if this is a continue (back-edge to loop header).
            // Only emit explicit `continue` when inside a branch (stop_at is set),
            // not at the natural end of the loop body where the back-edge is implicit.
            if let Some(ctx) = loop_ctx
                && current == ctx.continue_target
                && stop_at.is_some()
            {
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Continue {
                    id: current,
                }));
            }
            break;
        }

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
                // Check for loop break gotos (continue is handled by visited check above)
                if let Some(ctx) = loop_ctx
                    && *next == ctx.break_target
                {
                    instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Break {
                        id: current,
                    }));
                    break;
                }
                current = *next;
                continue;
            }
            Terminal::If { test, consequent, alternate, fallthrough } => {
                let consequent_block = build_reactive_block_until(
                    hir,
                    *consequent,
                    Some(*fallthrough),
                    visited,
                    loop_ctx,
                );
                let alternate_block = build_reactive_block_until(
                    hir,
                    *alternate,
                    Some(*fallthrough),
                    visited,
                    loop_ctx,
                );
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
                        let block =
                            build_reactive_block_until(hir, case.block, None, visited, loop_ctx);
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
                let init_block = build_reactive_block_until(hir, *init, None, visited, loop_ctx);
                let test_block = build_reactive_block_until(hir, *test, None, visited, loop_ctx);
                // continue goes to update (or test if no update), break goes to fallthrough
                let continue_target = update.unwrap_or(*test);
                let body_loop_ctx =
                    Some(LoopContext { continue_target, break_target: *fallthrough });
                let update_block = update
                    .map(|u| build_reactive_block_until(hir, u, None, visited, body_loop_ctx));
                let body_block =
                    build_reactive_block_until(hir, *body, None, visited, body_loop_ctx);
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
                let test_block = build_reactive_block_until(hir, *test, None, visited, loop_ctx);
                let body_loop_ctx =
                    Some(LoopContext { continue_target: *test, break_target: *fallthrough });
                let body_block =
                    build_reactive_block_until(hir, *body, None, visited, body_loop_ctx);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::While {
                    test: test_block,
                    body: body_block,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::DoWhile { body, test, fallthrough } => {
                let body_loop_ctx =
                    Some(LoopContext { continue_target: *test, break_target: *fallthrough });
                let body_block =
                    build_reactive_block_until(hir, *body, None, visited, body_loop_ctx);
                let test_block =
                    build_reactive_block_until(hir, *test, None, visited, body_loop_ctx);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::DoWhile {
                    body: body_block,
                    test: test_block,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::ForOf { init, test, body, fallthrough } => {
                // Use stop_at to prevent each sub-block from following Gotos
                // into sibling blocks. Without this, init follows Goto->test->body,
                // consuming all instructions and leaving test/body empty.
                let init_block =
                    build_reactive_block_until(hir, *init, Some(*test), visited, loop_ctx);
                let test_block =
                    build_reactive_block_until(hir, *test, Some(*body), visited, loop_ctx);
                // continue goes to init (where GetIterator/IteratorNext live), break goes to fallthrough
                let body_loop_ctx =
                    Some(LoopContext { continue_target: *init, break_target: *fallthrough });
                let body_block =
                    build_reactive_block_until(hir, *body, None, visited, body_loop_ctx);
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
                let init_block =
                    build_reactive_block_until(hir, *init, Some(*test), visited, loop_ctx);
                let test_block =
                    build_reactive_block_until(hir, *test, Some(*body), visited, loop_ctx);
                // continue goes to init (where NextPropertyOf lives), break goes to fallthrough
                let body_loop_ctx =
                    Some(LoopContext { continue_target: *init, break_target: *fallthrough });
                let body_block =
                    build_reactive_block_until(hir, *body, None, visited, body_loop_ctx);
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
                let try_reactive =
                    build_reactive_block_until(hir, *try_block, None, visited, loop_ctx);
                let handler_reactive =
                    build_reactive_block_until(hir, *handler, None, visited, loop_ctx);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Try {
                    block: try_reactive,
                    handler: handler_reactive,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::Label { block: label_block, fallthrough, label } => {
                let label_reactive =
                    build_reactive_block_until(hir, *label_block, None, visited, loop_ctx);
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
                // Pass the scope's fallthrough as a boundary so that If/Ternary branches
                // inside the scope don't consume post-scope blocks.
                let scope_reactive = build_scope_block_only(
                    hir,
                    *scope_block,
                    visited,
                    Some(*fallthrough),
                    loop_ctx,
                );

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
            Terminal::Ternary { test, consequent, alternate, fallthrough, .. } => {
                let consequent_block = build_reactive_block_until(
                    hir,
                    *consequent,
                    Some(*fallthrough),
                    visited,
                    loop_ctx,
                );
                let alternate_block = build_reactive_block_until(
                    hir,
                    *alternate,
                    Some(*fallthrough),
                    visited,
                    loop_ctx,
                );

                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::If {
                    test: test.clone(),
                    consequent: consequent_block,
                    alternate: alternate_block,
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::Logical { operator, right, fallthrough, result, .. } => {
                // The left-side instructions (including StoreLocal for result)
                // are already emitted above. The right block must execute
                // conditionally based on the operator.
                let right_block =
                    build_reactive_block_until(hir, *right, Some(*fallthrough), visited, loop_ctx);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Logical {
                    operator: *operator,
                    right: right_block,
                    result: result.clone(),
                    id: current,
                }));
                current = *fallthrough;
                continue;
            }
            Terminal::Optional { consequent, fallthrough, .. } => {
                // Inline consequent block (optional chain continuation)
                let cons_block = build_reactive_block_until(
                    hir,
                    *consequent,
                    Some(*fallthrough),
                    visited,
                    loop_ctx,
                );
                instructions.extend(cons_block.instructions);
                current = *fallthrough;
                continue;
            }
            Terminal::Sequence { blocks, fallthrough } => {
                // Process all sequence blocks in order
                for block_id in blocks {
                    let block = build_reactive_block_until(
                        hir,
                        *block_id,
                        Some(*fallthrough),
                        visited,
                        loop_ctx,
                    );
                    instructions.extend(block.instructions);
                }
                current = *fallthrough;
                continue;
            }
            Terminal::Branch { test, consequent, alternate } => {
                // Lower Branch to If (Branch has no fallthrough — it's terminal)
                let cons_block =
                    build_reactive_block_until(hir, *consequent, None, visited, loop_ctx);
                let alt_block =
                    build_reactive_block_until(hir, *alternate, None, visited, loop_ctx);
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
///
/// `scope_fallthrough` is the scope's own fallthrough block ID. When processing
/// If/Ternary branches inside the scope, `build_reactive_block_until` calls use
/// this as an additional stop boundary to prevent consuming post-scope blocks.
fn build_scope_block_only(
    hir: &HIR,
    block_id: BlockId,
    visited: &mut FxHashSet<BlockId>,
    scope_fallthrough: Option<BlockId>,
    loop_ctx: Option<LoopContext>,
) -> ReactiveBlock {
    let mut instructions = Vec::new();

    // Prevent infinite recursion from cyclic fallthrough chains
    if !visited.insert(block_id) {
        return ReactiveBlock { instructions };
    }

    // Don't process blocks at or beyond the scope boundary
    if scope_fallthrough == Some(block_id) {
        // Un-visit this block so the outer caller can process it
        visited.remove(&block_id);
        return ReactiveBlock { instructions };
    }

    if let Some(block) = find_block(hir, block_id) {
        for instr in &block.instructions {
            instructions.push(ReactiveInstruction::Instruction(instr.clone()));
        }

        // Process the terminal, but don't follow Goto (that's the fallthrough).
        // Other terminals (If, Switch, etc.) within the scope are processed normally.
        match &block.terminal {
            Terminal::Goto { block: next } => {
                // Don't follow — fallthrough is handled by the caller.
                // But if this Goto targets a block that ISN'T the scope fallthrough,
                // we should follow it within the scope.
                if scope_fallthrough != Some(*next) {
                    let remaining =
                        build_scope_block_only(hir, *next, visited, scope_fallthrough, loop_ctx);
                    instructions.extend(remaining.instructions);
                }
            }
            Terminal::If { test, consequent, alternate, fallthrough } => {
                let consequent_block = build_reactive_block_until(
                    hir,
                    *consequent,
                    Some(*fallthrough),
                    visited,
                    loop_ctx,
                );
                let alternate_block = build_reactive_block_until(
                    hir,
                    *alternate,
                    Some(*fallthrough),
                    visited,
                    loop_ctx,
                );
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::If {
                    test: test.clone(),
                    consequent: consequent_block,
                    alternate: alternate_block,
                    id: block_id,
                }));
                // Continue with fallthrough within the scope
                let remaining =
                    build_scope_block_only(hir, *fallthrough, visited, scope_fallthrough, loop_ctx);
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
                let scope_reactive = build_scope_block_only(
                    hir,
                    *scope_block,
                    visited,
                    Some(*fallthrough),
                    loop_ctx,
                );
                let reactive_scope = find_scope_in_block(hir, *scope_block, *scope);
                if let Some(rs) = reactive_scope {
                    instructions.push(ReactiveInstruction::Scope(ReactiveScopeBlock {
                        scope: rs,
                        instructions: scope_reactive,
                    }));
                } else {
                    instructions.extend(scope_reactive.instructions);
                }
                let remaining =
                    build_scope_block_only(hir, *fallthrough, visited, scope_fallthrough, loop_ctx);
                instructions.extend(remaining.instructions);
            }
            Terminal::Ternary { test, consequent, alternate, fallthrough, .. } => {
                let consequent_block = build_reactive_block_until(
                    hir,
                    *consequent,
                    Some(*fallthrough),
                    visited,
                    loop_ctx,
                );
                let alternate_block = build_reactive_block_until(
                    hir,
                    *alternate,
                    Some(*fallthrough),
                    visited,
                    loop_ctx,
                );
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::If {
                    test: test.clone(),
                    consequent: consequent_block,
                    alternate: alternate_block,
                    id: block_id,
                }));
                let remaining =
                    build_scope_block_only(hir, *fallthrough, visited, scope_fallthrough, loop_ctx);
                instructions.extend(remaining.instructions);
            }
            Terminal::Logical { operator, right, fallthrough, result, .. } => {
                let right_block =
                    build_reactive_block_until(hir, *right, Some(*fallthrough), visited, loop_ctx);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Logical {
                    operator: *operator,
                    right: right_block,
                    result: result.clone(),
                    id: block_id,
                }));
                let remaining =
                    build_scope_block_only(hir, *fallthrough, visited, scope_fallthrough, loop_ctx);
                instructions.extend(remaining.instructions);
            }
            Terminal::Optional { consequent, fallthrough, .. } => {
                let cons_block = build_reactive_block_until(
                    hir,
                    *consequent,
                    Some(*fallthrough),
                    visited,
                    loop_ctx,
                );
                instructions.extend(cons_block.instructions);
                let remaining =
                    build_scope_block_only(hir, *fallthrough, visited, scope_fallthrough, loop_ctx);
                instructions.extend(remaining.instructions);
            }
            Terminal::Sequence { blocks, fallthrough } => {
                for bid in blocks {
                    let block = build_reactive_block_until(
                        hir,
                        *bid,
                        Some(*fallthrough),
                        visited,
                        loop_ctx,
                    );
                    instructions.extend(block.instructions);
                }
                let remaining =
                    build_scope_block_only(hir, *fallthrough, visited, scope_fallthrough, loop_ctx);
                instructions.extend(remaining.instructions);
            }
            Terminal::MaybeThrow { continuation, .. } => {
                let remaining = build_scope_block_only(
                    hir,
                    *continuation,
                    visited,
                    scope_fallthrough,
                    loop_ctx,
                );
                instructions.extend(remaining.instructions);
            }
            Terminal::Switch { test, cases, fallthrough } => {
                let reactive_cases: Vec<(Option<crate::hir::types::Place>, ReactiveBlock)> = cases
                    .iter()
                    .map(|case| {
                        let block =
                            build_reactive_block_until(hir, case.block, None, visited, loop_ctx);
                        (case.test.clone(), block)
                    })
                    .collect();
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Switch {
                    test: test.clone(),
                    cases: reactive_cases,
                    id: block_id,
                }));
                let remaining =
                    build_scope_block_only(hir, *fallthrough, visited, scope_fallthrough, loop_ctx);
                instructions.extend(remaining.instructions);
            }
            Terminal::Throw { value } => {
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::Throw {
                    value: value.clone(),
                    id: block_id,
                }));
            }
            Terminal::Branch { test, consequent, alternate } => {
                let cons_block =
                    build_reactive_block_until(hir, *consequent, None, visited, loop_ctx);
                let alt_block =
                    build_reactive_block_until(hir, *alternate, None, visited, loop_ctx);
                instructions.push(ReactiveInstruction::Terminal(ReactiveTerminal::If {
                    test: test.clone(),
                    consequent: cons_block,
                    alternate: alt_block,
                    id: block_id,
                }));
            }
            Terminal::PrunedScope { fallthrough, .. } => {
                let remaining =
                    build_scope_block_only(hir, *fallthrough, visited, scope_fallthrough, loop_ctx);
                instructions.extend(remaining.instructions);
            }
            _ => {
                // Unreachable, loops, etc. — no special handling needed in scope blocks
            }
        }
    }

    ReactiveBlock { instructions }
}

fn find_scope_in_block(
    hir: &HIR,
    block_id: BlockId,
    scope_id: crate::hir::types::ScopeId,
) -> Option<crate::hir::types::ReactiveScope> {
    if let Some(block) = find_block(hir, block_id) {
        for instr in &block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope
                && scope.id == scope_id
            {
                return Some(scope.as_ref().clone());
            }
        }
    }
    None
}
