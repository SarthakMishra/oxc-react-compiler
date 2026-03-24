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
/// FinishMemoize pair), check:
///
/// 1. The FinishMemoize is inside a reactive scope (if not, the compiler failed
///    to create a scope for this memoized value).
/// 2. The StartMemoize and FinishMemoize are in the same reactive scope (if not,
///    the compiler split the memo region across scopes).
///
/// DIVERGENCE: Upstream checks `identifier.scope` on the FinishMemoize.decl, but
/// our HIR doesn't populate `identifier.scope`. We use the current reactive scope
/// context instead.
pub fn validate_preserved_manual_memoization(func: &ReactiveFunction, errors: &mut ErrorCollector) {
    let mut memo_scopes: FxHashMap<u32, MemoRegion> = FxHashMap::default();
    walk_reactive_block(&func.body, None, &mut memo_scopes);

    for region in memo_scopes.values() {
        // Skip pruned memoizations
        if region.pruned {
            continue;
        }

        // Skip if ValidateExhaustiveDependencies already flagged invalid deps.
        // The root cause is the wrong deps, not a memoization preservation failure.
        if region.has_invalid_deps {
            continue;
        }

        // If the FinishMemoize is outside any reactive scope, the value was
        // supposed to be memoized but the compiler didn't create a scope for it.
        if region.finish_scope.is_none() {
            errors.push(CompilerError::invalid_react_with_kind(
                region.loc,
                UNMEMOIZED_ERROR,
                DiagnosticKind::MemoizationPreservation,
            ));
            continue;
        }

        // If start and finish are in different scopes, the manual memo region
        // was split across multiple reactive scopes — the memoization semantics
        // won't be preserved.
        if region.start_scope != region.finish_scope {
            errors.push(CompilerError::invalid_react_with_kind(
                region.loc,
                UNMEMOIZED_ERROR,
                DiagnosticKind::MemoizationPreservation,
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
    /// When true, ValidateExhaustiveDependencies already flagged this memo's
    /// dependency array as invalid. Skip reporting preservation errors to
    /// avoid duplicate diagnostics for the same root cause.
    has_invalid_deps: bool,
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
        InstructionValue::StartMemoize { manual_memo_id, has_invalid_deps } => {
            let entry = memo_scopes.entry(*manual_memo_id).or_insert(MemoRegion {
                start_scope: current_scope,
                finish_scope: None,
                pruned: false,
                has_invalid_deps: *has_invalid_deps,
                loc: instr.loc,
            });
            entry.start_scope = current_scope;
            entry.has_invalid_deps = *has_invalid_deps;
        }
        InstructionValue::FinishMemoize { manual_memo_id, pruned, .. } => {
            let entry = memo_scopes.entry(*manual_memo_id).or_insert(MemoRegion {
                start_scope: None,
                finish_scope: current_scope,
                pruned: *pruned,
                has_invalid_deps: false,
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
