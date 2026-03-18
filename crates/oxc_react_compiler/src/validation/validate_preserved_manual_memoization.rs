use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{
    Instruction, InstructionValue, ReactiveBlock, ReactiveFunction, ReactiveInstruction, ScopeId,
};
use rustc_hash::FxHashMap;

const UNMEMOIZED_ERROR: &str = "Existing memoization could not be preserved. React Compiler \
has skipped optimizing this component because the existing manual memoization \
could not be preserved. This value was memoized in source but not in \
compilation output.";

/// Validate that compiler-generated memoization preserves manual memoization
/// guarantees from `useMemo` / `useCallback`.
///
/// Upstream: ValidatePreservedManualMemoization.ts
///
/// After reactive scope inference and RF optimization passes, walk the reactive
/// function in evaluation order. For each manual memoization region (StartMemoize/
/// FinishMemoize pair), check that the memoized value is covered by a reactive
/// scope. If it is not, the compiler failed to create a scope for the value.
///
/// DIVERGENCE: Upstream checks `identifier.scope` on each FinishMemoize operand
/// to verify it was assigned to a completed scope. Our HIR doesn't populate
/// `identifier.scope`, so we approximate by tracking whether any reactive scope
/// was encountered between StartMemoize and FinishMemoize, or whether FinishMemoize
/// itself is inside a scope. If either is true, the value is considered memoized.
///
/// Upstream also validates that inferred scope dependencies match the manual
/// deps from source (`validateInferredDep` + `compareDeps`). We skip that check
/// because it requires temporaries/dependency normalization infrastructure we
/// haven't ported yet.
pub fn validate_preserved_manual_memoization(func: &ReactiveFunction, errors: &mut ErrorCollector) {
    let mut memo_scopes: FxHashMap<u32, MemoRegion> = FxHashMap::default();
    walk_reactive_block(&func.body, None, &mut memo_scopes);

    for region in memo_scopes.values() {
        // Skip pruned memoizations
        if region.pruned {
            continue;
        }

        // A memo region is preserved if:
        // 1. The FinishMemoize is inside a reactive scope, OR
        // 2. A reactive scope was encountered between StartMemoize and FinishMemoize
        //    (the scope covers the computation even if Start/Finish are outside it).
        if region.finish_scope.is_some() || region.has_inner_scope {
            continue;
        }

        errors.push(CompilerError::invalid_react_with_kind(
            region.loc,
            UNMEMOIZED_ERROR,
            DiagnosticKind::MemoizationPreservation,
        ));
    }
}

/// Tracks the reactive scope context of a StartMemoize/FinishMemoize pair.
#[derive(Debug)]
struct MemoRegion {
    finish_scope: Option<ScopeId>,
    /// Whether a reactive scope was encountered between StartMemoize and FinishMemoize.
    has_inner_scope: bool,
    /// Whether the FinishMemoize has been seen yet (region is still open).
    finished: bool,
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
                // Mark all active (started but not finished) memo regions as having
                // an inner scope.
                for region in memo_scopes.values_mut() {
                    if !region.finished {
                        region.has_inner_scope = true;
                    }
                }
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
            memo_scopes.entry(*manual_memo_id).or_insert(MemoRegion {
                finish_scope: None,
                has_inner_scope: false,
                finished: false,
                pruned: false,
                loc: instr.loc,
            });
        }
        InstructionValue::FinishMemoize { manual_memo_id, pruned, .. } => {
            let entry = memo_scopes.entry(*manual_memo_id).or_insert(MemoRegion {
                finish_scope: current_scope,
                has_inner_scope: false,
                finished: true,
                pruned: *pruned,
                loc: instr.loc,
            });
            entry.finish_scope = current_scope;
            entry.finished = true;
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
        ReactiveTerminal::If { consequent, alternate, .. } => {
            walk_reactive_block(consequent, current_scope, memo_scopes);
            walk_reactive_block(alternate, current_scope, memo_scopes);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, case_block) in cases {
                walk_reactive_block(case_block, current_scope, memo_scopes);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            walk_reactive_block(init, current_scope, memo_scopes);
            walk_reactive_block(test, current_scope, memo_scopes);
            if let Some(upd) = update {
                walk_reactive_block(upd, current_scope, memo_scopes);
            }
            walk_reactive_block(body, current_scope, memo_scopes);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
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
        ReactiveTerminal::Logical { right, .. } => {
            walk_reactive_block(right, current_scope, memo_scopes);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
    }
}
