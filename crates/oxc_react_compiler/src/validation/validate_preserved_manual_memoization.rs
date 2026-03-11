#![allow(dead_code)]

use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{
    Instruction, InstructionValue, ReactiveBlock, ReactiveFunction, ReactiveInstruction, ScopeId,
};
use rustc_hash::FxHashMap;

/// Validate that compiler-generated memoization preserves manual memoization
/// guarantees from `useMemo` / `useCallback`.
///
/// The compiler inserts `StartMemoize` / `FinishMemoize` instruction pairs to
/// mark regions that the developer explicitly memoized. After reactive scope
/// inference, we verify that each manual memo region is fully contained within
/// a single reactive scope. If a manual memo region spans multiple scopes (or
/// none), the compiler's output would have different memoization semantics than
/// the developer intended.
pub fn validate_preserved_manual_memoization(func: &ReactiveFunction, errors: &mut ErrorCollector) {
    // Collect all StartMemoize/FinishMemoize pairs and which reactive scopes
    // they appear in.
    let mut memo_scopes: FxHashMap<u32, MemoRegion> = FxHashMap::default();

    walk_reactive_block(&func.body, None, &mut memo_scopes);

    // Validate each memo region
    for (memo_id, region) in &memo_scopes {
        // If start and finish are in different scopes, the manual memo is split
        if region.start_scope != region.finish_scope {
            let loc = region.loc;
            errors.push(CompilerError::invalid_react(
                loc,
                format!(
                    "Manual memoization (memo id {}) is not preserved. \
                     The memoized region spans multiple reactive scopes, which means \
                     the compiler cannot guarantee the same memoization semantics.",
                    memo_id
                ),
            ));
        }

        // If the memo region has been pruned, warn that it was removed
        if region.pruned {
            errors.push(CompilerError::invalid_react(
                region.loc,
                format!(
                    "Manual memoization (memo id {}) was pruned. \
                     The compiler determined the memoized value does not need \
                     memoization, but this may change the program's semantics.",
                    memo_id
                ),
            ));
        }
    }
}

/// Tracks the reactive scope context of a StartMemoize/FinishMemoize pair.
#[derive(Debug)]
struct MemoRegion {
    start_scope: Option<ScopeId>,
    finish_scope: Option<ScopeId>,
    pruned: bool,
    loc: oxc_span::Span,
}

/// Recursively walk a reactive block, tracking which reactive scope we are in.
fn walk_reactive_block(
    block: &ReactiveBlock,
    current_scope: Option<ScopeId>,
    memo_scopes: &mut FxHashMap<u32, MemoRegion>,
) {
    for item in &block.instructions {
        match item {
            ReactiveInstruction::Instruction(instr) => {
                check_instruction(instr, current_scope, memo_scopes);
            }
            ReactiveInstruction::Scope(scope_block) => {
                let scope_id = scope_block.scope.id;
                walk_reactive_block(&scope_block.instructions, Some(scope_id), memo_scopes);
            }
            ReactiveInstruction::Terminal(terminal) => {
                walk_terminal_blocks(terminal, current_scope, memo_scopes);
            }
        }
    }
}

/// Check a single instruction for StartMemoize / FinishMemoize markers.
fn check_instruction(
    instr: &Instruction,
    current_scope: Option<ScopeId>,
    memo_scopes: &mut FxHashMap<u32, MemoRegion>,
) {
    match &instr.value {
        InstructionValue::StartMemoize { manual_memo_id } => {
            memo_scopes
                .entry(*manual_memo_id)
                .or_insert(MemoRegion {
                    start_scope: current_scope,
                    finish_scope: None,
                    pruned: false,
                    loc: instr.loc,
                })
                .start_scope = current_scope;
        }
        InstructionValue::FinishMemoize {
            manual_memo_id,
            pruned,
            ..
        } => {
            let entry = memo_scopes.entry(*manual_memo_id).or_insert(MemoRegion {
                start_scope: None,
                finish_scope: current_scope,
                pruned: *pruned,
                loc: instr.loc,
            });
            entry.finish_scope = current_scope;
            entry.pruned = *pruned;
        }
        _ => {}
    }
}

/// Walk all blocks within a reactive terminal.
fn walk_terminal_blocks(
    terminal: &crate::hir::types::ReactiveTerminal,
    current_scope: Option<ScopeId>,
    memo_scopes: &mut FxHashMap<u32, MemoRegion>,
) {
    use crate::hir::types::ReactiveTerminal;

    match terminal {
        ReactiveTerminal::If {
            consequent,
            alternate,
            ..
        } => {
            walk_reactive_block(consequent, current_scope, memo_scopes);
            walk_reactive_block(alternate, current_scope, memo_scopes);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, case_block) in cases {
                walk_reactive_block(case_block, current_scope, memo_scopes);
            }
        }
        ReactiveTerminal::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            walk_reactive_block(init, current_scope, memo_scopes);
            walk_reactive_block(test, current_scope, memo_scopes);
            if let Some(upd) = update {
                walk_reactive_block(upd, current_scope, memo_scopes);
            }
            walk_reactive_block(body, current_scope, memo_scopes);
        }
        ReactiveTerminal::ForOf {
            init, test, body, ..
        }
        | ReactiveTerminal::ForIn {
            init, test, body, ..
        } => {
            walk_reactive_block(init, current_scope, memo_scopes);
            walk_reactive_block(test, current_scope, memo_scopes);
            walk_reactive_block(body, current_scope, memo_scopes);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            walk_reactive_block(test, current_scope, memo_scopes);
            walk_reactive_block(body, current_scope, memo_scopes);
        }
        ReactiveTerminal::Label { block, .. } => {
            walk_reactive_block(block, current_scope, memo_scopes);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            walk_reactive_block(block, current_scope, memo_scopes);
            walk_reactive_block(handler, current_scope, memo_scopes);
        }
        ReactiveTerminal::Return { .. } | ReactiveTerminal::Throw { .. } => {}
    }
}
