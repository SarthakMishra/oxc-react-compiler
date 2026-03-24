use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{
    Instruction, InstructionValue, ReactiveBlock, ReactiveFunction, ReactiveInstruction,
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
/// FinishMemoize pair), check that the FinishMemoize is inside a reactive scope.
/// If not, the compiler failed to create a scope for this memoized value.
///
/// DIVERGENCE: Upstream performs two additional checks that we skip:
/// 1. At StartMemoize: verifies each dependency operand's scope has completed
///    (prevents depending on values still being mutated). Requires
///    `identifier.scope` which our HIR doesn't populate.
/// 2. At scope visits inside memo regions: validates inferred scope dependencies
///    match the source deps from useMemo/useCallback (`validateInferredDep`).
///    Requires `ManualMemoDependency` and source deps on StartMemoize, which we
///    don't store. This means we don't catch dependency mismatch errors (e.g.,
///    aliased deps, property path mismatches). Those error fixtures are tracked
///    as known failures in known-failures.txt.
///
/// Previous versions also checked `start_scope == finish_scope`, but this
/// is not an upstream check and was overly strict -- it rejected 54+ fixtures
/// where the memo output was correctly memoized in a different scope than
/// the StartMemoize marker.
pub fn validate_preserved_manual_memoization(func: &ReactiveFunction, errors: &mut ErrorCollector) {
    let mut memo_regions: FxHashMap<u32, MemoRegion> = FxHashMap::default();
    walk_reactive_block(&func.body, false, &mut memo_regions);

    for region in memo_regions.values() {
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
        if !region.finish_in_scope {
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
    /// Whether FinishMemoize was found inside any reactive scope.
    finish_in_scope: bool,
    pruned: bool,
    /// When true, ValidateExhaustiveDependencies already flagged this memo's
    /// dependency array as invalid. Skip reporting preservation errors to
    /// avoid duplicate diagnostics for the same root cause.
    has_invalid_deps: bool,
    loc: oxc_span::Span,
}

/// Recursively walk a reactive block, tracking whether we are inside a scope.
fn walk_reactive_block(
    block: &ReactiveBlock,
    in_scope: bool,
    memo_regions: &mut FxHashMap<u32, MemoRegion>,
) {
    for item in &block.instructions {
        match item {
            ReactiveInstruction::Instruction(instr) => {
                check_instruction(instr, in_scope, memo_regions);
            }
            ReactiveInstruction::Scope(scope_block) => {
                walk_reactive_block(&scope_block.instructions, true, memo_regions);
            }
            ReactiveInstruction::Terminal(terminal) => {
                walk_terminal_blocks(terminal, in_scope, memo_regions);
            }
        }
    }
}

/// Check a single instruction for StartMemoize / FinishMemoize markers.
fn check_instruction(
    instr: &Instruction,
    in_scope: bool,
    memo_regions: &mut FxHashMap<u32, MemoRegion>,
) {
    match &instr.value {
        InstructionValue::StartMemoize { manual_memo_id, has_invalid_deps } => {
            let entry = memo_regions.entry(*manual_memo_id).or_insert(MemoRegion {
                finish_in_scope: false,
                pruned: false,
                has_invalid_deps: *has_invalid_deps,
                loc: instr.loc,
            });
            entry.has_invalid_deps = *has_invalid_deps;
        }
        InstructionValue::FinishMemoize { manual_memo_id, pruned, .. } => {
            let entry = memo_regions.entry(*manual_memo_id).or_insert(MemoRegion {
                finish_in_scope: in_scope,
                pruned: *pruned,
                has_invalid_deps: false,
                loc: instr.loc,
            });
            entry.finish_in_scope = in_scope;
            entry.pruned = *pruned;
        }
        _ => {}
    }
}

/// Walk all blocks within a reactive terminal.
fn walk_terminal_blocks(
    terminal: &crate::hir::types::ReactiveTerminal,
    in_scope: bool,
    memo_regions: &mut FxHashMap<u32, MemoRegion>,
) {
    use crate::hir::types::ReactiveTerminal;

    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            walk_reactive_block(consequent, in_scope, memo_regions);
            walk_reactive_block(alternate, in_scope, memo_regions);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, case_block) in cases {
                walk_reactive_block(case_block, in_scope, memo_regions);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            walk_reactive_block(init, in_scope, memo_regions);
            walk_reactive_block(test, in_scope, memo_regions);
            if let Some(upd) = update {
                walk_reactive_block(upd, in_scope, memo_regions);
            }
            walk_reactive_block(body, in_scope, memo_regions);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            walk_reactive_block(init, in_scope, memo_regions);
            walk_reactive_block(test, in_scope, memo_regions);
            walk_reactive_block(body, in_scope, memo_regions);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            walk_reactive_block(test, in_scope, memo_regions);
            walk_reactive_block(body, in_scope, memo_regions);
        }
        ReactiveTerminal::Label { block, .. } => {
            walk_reactive_block(block, in_scope, memo_regions);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            walk_reactive_block(block, in_scope, memo_regions);
            walk_reactive_block(handler, in_scope, memo_regions);
        }
        ReactiveTerminal::Logical { right, .. } => {
            walk_reactive_block(right, in_scope, memo_regions);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
    }
}
