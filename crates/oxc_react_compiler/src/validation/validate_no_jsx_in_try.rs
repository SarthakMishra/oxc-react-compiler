
use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{BlockId, HIR, InstructionValue, Terminal};
use rustc_hash::FxHashSet;

/// Validate that JSX is not used inside try blocks.
///
/// React components that may throw should be wrapped with error boundaries
/// rather than try/catch. Catching errors from JSX rendering prevents React
/// from properly handling the error and can lead to inconsistent UI state.
pub fn validate_no_jsx_in_try(hir: &HIR, errors: &mut ErrorCollector) {
    // Step 1: Collect all block IDs that are inside a Try terminal's `block` arm.
    let try_blocks = collect_try_block_ids(hir);

    // Step 2: Check those blocks for JSX instructions.
    for (block_id, block) in &hir.blocks {
        if !try_blocks.contains(block_id) {
            continue;
        }

        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::JsxExpression { .. } | InstructionValue::JsxFragment { .. } => {
                    errors.push(CompilerError::invalid_react_with_kind(
                        instr.loc,
                        "JSX inside a try block. Use an error boundary component \
                         instead of try/catch for handling errors in JSX rendering."
                            .to_string(),
                        DiagnosticKind::JsxInTry,
                    ));
                }
                _ => {}
            }
        }
    }
}

/// Collect block IDs that are the `block` arm of Try terminals, including
/// transitively reachable blocks via Goto.
fn collect_try_block_ids(hir: &HIR) -> FxHashSet<BlockId> {
    let mut try_blocks = FxHashSet::default();

    for (_, block) in &hir.blocks {
        if let Terminal::Try { block: try_body, .. } = &block.terminal {
            mark_reachable(hir, *try_body, &mut try_blocks);
        }
    }

    try_blocks
}

/// Transitively mark blocks reachable from a given block.
fn mark_reachable(hir: &HIR, start: BlockId, visited: &mut FxHashSet<BlockId>) {
    if !visited.insert(start) {
        return;
    }

    if let Some((_, block)) = hir.blocks.iter().find(|(id, _)| *id == start)
        && let Terminal::Goto { block: next } = &block.terminal {
            mark_reachable(hir, *next, visited);
        }
}
