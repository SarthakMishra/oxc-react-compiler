use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{BlockId, HIR, Instruction, InstructionKind, InstructionValue, Terminal};

/// Set of `UnsupportedNode` names that upstream explicitly rejects as Todo errors.
///
/// These correspond to patterns where upstream's BuildHIR emits a `Todo` diagnostic
/// and bails out. Other `UnsupportedNode` instructions (e.g., `DebuggerStatement`,
/// statement-level type declarations) are benign and should not cause bail-out.
const REJECTED_UNSUPPORTED_NODES: &[&str] = &[
    // Upstream: Todo: (BuildHIR::lowerExpression) Handle YieldExpression expressions
    "YieldExpression",
    // Upstream: Todo: (BuildHIR::lowerExpression) Handle get functions in ObjectExpression
    "ObjectExpression_get_syntax",
    // Upstream: Todo: (BuildHIR::lowerExpression) Handle set functions in ObjectExpression
    "ObjectExpression_set_syntax",
    // Upstream: Todo: (BuildHIR::lowerExpression) Handle MetaProperty expressions other than import.meta
    "MetaProperty_new_target",
    // Upstream: Todo: (BuildHIR::lowerStatement) Handle for-await loops
    "ForAwaitOfStatement",
    // Upstream: Todo: (BuildHIR::lowerExpression) Handle ClassExpression
    "ClassExpression",
    // Upstream: Todo: (BuildHIR::lowerStatement) Handle TryStatement without a catch clause
    "TryStatement_without_catch",
    // Upstream: Todo: (BuildHIR::lowerExpression) Expected Identifier, got CallExpression/SequenceExpression key in ObjectExpression
    // Note: ThrowStatement in try/catch is detected via CFG walk in check_value_blocks_in_try,
    // not via UnsupportedNode markers, so it doesn't appear in this list.
    "ObjectExpression_computed_key",
];

/// Validate that the HIR contains no `UnsupportedNode` instructions for patterns
/// that upstream explicitly rejects, and detect other unsupported syntax patterns.
///
/// Upstream emits `Todo` errors during BuildHIR for patterns it doesn't support
/// (e.g., getters/setters in object expressions, `new.target`, `for await` loops,
/// `yield` expressions, class expressions). Our HIRBuilder emits `UnsupportedNode`
/// instructions for these cases. This pass converts the known-rejected ones into
/// `CompilerError::todo()` errors, causing the pipeline to bail — matching upstream.
///
/// Additionally detects:
/// - `var` declarations: upstream bails with "Todo: Handle var kinds in VariableDeclaration"
///   because `var` hoisting semantics are not correctly modeled in the HIR.
/// - Value blocks in try/catch: upstream bails when conditional/logical/optional/loop
///   expressions appear inside try blocks.
/// - Local variables named `fbt`: upstream bails on these.
///
/// This pass runs very early (before any optimization or SSA passes) so that
/// unsupported patterns are caught immediately.
pub fn validate_no_unsupported_nodes(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::UnsupportedNode { node } = &instr.value
                && REJECTED_UNSUPPORTED_NODES.contains(&node.as_str())
            {
                errors.push(CompilerError::todo(
                    instr.loc,
                    format!("(BuildHIR) Unsupported node: {node}"),
                ));
            }

            // Upstream: Todo: (BuildHIR::lowerStatement) Handle var kinds in VariableDeclaration
            // `var` declarations have function-level hoisting semantics that our HIR
            // does not model correctly. Upstream bails on these.
            check_var_declaration(instr, errors);

            // Upstream: Todo: Support local variables named `fbt`
            // fbt is a special module for Facebook internationalization. When fbt is
            // used as a local variable name, upstream bails.
            check_fbt_local(instr, errors);

            // Also check nested function bodies
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    validate_no_unsupported_nodes(&lowered_func.body, errors);
                }
                _ => {}
            }
        }
    }

    // Check for value blocks and throw statements inside try/catch.
    // Upstream: Todo: Support value blocks (conditional, logical, optional chaining, etc)
    // within a try/catch statement
    // Upstream: Todo: (BuildHIR::lowerStatement) Support ThrowStatement inside of try/catch
    check_value_blocks_in_try(hir, errors);
}

/// Check if an instruction declares a `var` variable.
/// Upstream does not support `var` declarations because their function-level
/// hoisting semantics are incompatible with the block-scoped HIR model.
fn check_var_declaration(instr: &Instruction, errors: &mut ErrorCollector) {
    match &instr.value {
        InstructionValue::DeclareLocal { type_: InstructionKind::Var, .. }
        | InstructionValue::StoreLocal { type_: Some(InstructionKind::Var), .. } => {
            errors.push(CompilerError::todo(
                instr.loc,
                "(BuildHIR::lowerStatement) Handle var kinds in VariableDeclaration".to_string(),
            ));
        }
        _ => {}
    }
}

/// Check if an instruction declares a local variable named `fbt`.
/// Upstream: Todo: Support local variables named `fbt`
fn check_fbt_local(instr: &Instruction, errors: &mut ErrorCollector) {
    if let InstructionValue::DeclareLocal { lvalue, .. }
    | InstructionValue::StoreLocal { lvalue, .. } = &instr.value
        && let Some(name) = &lvalue.identifier.name
        && name == "fbt"
    {
        errors.push(CompilerError::todo(
            instr.loc,
            "Support local variables named `fbt`".to_string(),
        ));
    }
}

/// Check for "value blocks" (conditional, logical, optional chaining, for loops)
/// inside try/catch statements.
///
/// Upstream: Todo: Support value blocks (conditional, logical, optional chaining, etc)
/// within a try/catch statement.
///
/// Upstream's BuildHIR detects when lowering an expression inside a try block
/// would create "value blocks" — intermediate blocks for evaluating conditional
/// expressions, logical expressions, optional chains, or loop constructs.
/// These value blocks break the try/catch control flow model because exceptions
/// thrown inside them would not be properly caught.
///
/// We detect this by walking the HIR CFG: for each Try terminal, collect all
/// blocks reachable from the try body (not the handler), and check if any of
/// those blocks contain terminals that create value blocks (conditional, logical,
/// optional, for/for-in/for-of).
fn check_value_blocks_in_try(hir: &HIR, errors: &mut ErrorCollector) {
    // Build a lookup table for O(1) block access instead of O(n) linear scan.
    let block_map: rustc_hash::FxHashMap<BlockId, &crate::hir::types::BasicBlock> =
        hir.blocks.iter().map(|(id, block)| (*id, block)).collect();

    for (_, block) in &hir.blocks {
        if let Terminal::Try { block: try_body, handler, fallthrough } = &block.terminal {
            let loc = block.instructions.last().map_or(oxc_span::SPAN, |i| i.loc);

            // Walk all blocks reachable from the try body.
            // A "value block" terminal is one that creates intermediate
            // evaluation blocks inside the try scope.
            let (has_value_block, has_throw) =
                check_try_body_terminals(&block_map, *try_body, *handler, *fallthrough);
            if has_value_block {
                errors.push(CompilerError::todo(
                    loc,
                    "Support value blocks (conditional, logical, optional chaining, etc) within a try/catch statement".to_string(),
                ));
            }
            if has_throw {
                errors.push(CompilerError::todo(
                    loc,
                    "(BuildHIR::lowerStatement) Support ThrowStatement inside of try/catch"
                        .to_string(),
                ));
            }
        }
    }
}

/// Check blocks reachable from `start` (within the try body, not entering
/// the handler or fallthrough) for "value block" terminals and throw terminals.
/// Returns (has_value_block, has_throw).
fn check_try_body_terminals(
    block_map: &rustc_hash::FxHashMap<BlockId, &crate::hir::types::BasicBlock>,
    start: BlockId,
    handler: BlockId,
    fallthrough: BlockId,
) -> (bool, bool) {
    let mut visited = rustc_hash::FxHashSet::default();
    let mut stack = vec![start];
    let mut found_value_block = false;
    let mut found_throw = false;
    while let Some(block_id) = stack.pop() {
        // Don't cross into handler or fallthrough — those are outside the try body
        if block_id == handler || block_id == fallthrough {
            continue;
        }
        if !visited.insert(block_id) {
            continue;
        }
        let Some(block) = block_map.get(&block_id) else {
            continue;
        };
        if is_value_block_terminal(&block.terminal) {
            found_value_block = true;
        }
        if matches!(&block.terminal, Terminal::Throw { .. }) {
            found_throw = true;
        }
        if found_value_block && found_throw {
            return (true, true);
        }
        // Follow successors within the try body
        // (including nested try handler blocks, which are still reachable from
        // the outer try body)
        collect_terminal_successors(&block.terminal, &mut stack);
    }
    (found_value_block, found_throw)
}

/// Returns true if the terminal creates "value blocks" that upstream doesn't support
/// inside try/catch.
fn is_value_block_terminal(terminal: &Terminal) -> bool {
    matches!(
        terminal,
        Terminal::Ternary { .. }
            | Terminal::Logical { .. }
            | Terminal::Optional { .. }
            | Terminal::For { .. }
            | Terminal::ForOf { .. }
            | Terminal::ForIn { .. }
    )
}

/// Collect all successor block IDs from a terminal.
fn collect_terminal_successors(terminal: &Terminal, successors: &mut Vec<BlockId>) {
    match terminal {
        Terminal::Goto { block } => {
            successors.push(*block);
        }
        Terminal::MaybeThrow { continuation, handler, .. } => {
            successors.push(*continuation);
            successors.push(*handler);
        }
        Terminal::If { consequent, alternate, fallthrough, .. } => {
            successors.push(*consequent);
            successors.push(*alternate);
            successors.push(*fallthrough);
        }
        Terminal::Branch { consequent, alternate, .. } => {
            successors.push(*consequent);
            successors.push(*alternate);
        }
        Terminal::Ternary { consequent, alternate, fallthrough, .. } => {
            successors.push(*consequent);
            successors.push(*alternate);
            successors.push(*fallthrough);
        }
        Terminal::Logical { right, fallthrough, .. } => {
            successors.push(*right);
            successors.push(*fallthrough);
        }
        Terminal::Optional { consequent, fallthrough, .. } => {
            successors.push(*consequent);
            successors.push(*fallthrough);
        }
        Terminal::Switch { cases, fallthrough, .. } => {
            for case in cases {
                successors.push(case.block);
            }
            successors.push(*fallthrough);
        }
        Terminal::For { init, test, update, body, fallthrough } => {
            successors.push(*init);
            successors.push(*test);
            if let Some(u) = update {
                successors.push(*u);
            }
            successors.push(*body);
            successors.push(*fallthrough);
        }
        Terminal::ForOf { init, test, body, fallthrough }
        | Terminal::ForIn { init, test, body, fallthrough } => {
            successors.push(*init);
            successors.push(*test);
            successors.push(*body);
            successors.push(*fallthrough);
        }
        Terminal::DoWhile { body, test, fallthrough }
        | Terminal::While { test, body, fallthrough } => {
            successors.push(*test);
            successors.push(*body);
            successors.push(*fallthrough);
        }
        Terminal::Sequence { blocks, fallthrough } => {
            successors.extend(blocks);
            successors.push(*fallthrough);
        }
        Terminal::Try { block, handler, fallthrough } => {
            successors.push(*block);
            successors.push(*handler);
            successors.push(*fallthrough);
        }
        Terminal::Scope { block, fallthrough, .. }
        | Terminal::PrunedScope { block, fallthrough, .. } => {
            successors.push(*block);
            successors.push(*fallthrough);
        }
        Terminal::Label { block, fallthrough, .. } => {
            successors.push(*block);
            successors.push(*fallthrough);
        }
        Terminal::Return { .. } | Terminal::Throw { .. } | Terminal::Unreachable => {}
    }
}
