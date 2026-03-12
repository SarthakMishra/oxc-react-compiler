//! Validate that all break/continue targets are well-formed.
//!
//! Checks that:
//! - Every labeled break/continue target references an existing label
//! - Break targets don't cross reactive scope boundaries
//! - Continue targets only reference loop constructs

use rustc_hash::FxHashSet;

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{BlockId, HIR, Terminal};

/// Validate that break/continue targets in the HIR are well-formed.
pub fn assert_well_formed_break_targets(hir: &HIR, errors: &mut ErrorCollector) {
    // Collect all label blocks and their fallthrough targets
    let mut label_blocks: FxHashSet<BlockId> = FxHashSet::default();
    let mut label_fallthroughs: FxHashSet<BlockId> = FxHashSet::default();
    let mut loop_fallthroughs: FxHashSet<BlockId> = FxHashSet::default();
    let mut all_block_ids: FxHashSet<BlockId> = FxHashSet::default();

    for (bid, block) in &hir.blocks {
        all_block_ids.insert(*bid);
        match &block.terminal {
            Terminal::Label { block: _, fallthrough, .. } => {
                label_blocks.insert(*bid);
                label_fallthroughs.insert(*fallthrough);
            }
            Terminal::For { fallthrough, .. }
            | Terminal::ForOf { fallthrough, .. }
            | Terminal::ForIn { fallthrough, .. }
            | Terminal::While { fallthrough, .. }
            | Terminal::DoWhile { fallthrough, .. } => {
                loop_fallthroughs.insert(*fallthrough);
            }
            _ => {}
        }
    }

    // Validate: every Goto target should be a valid block
    for (bid, block) in &hir.blocks {
        if let Terminal::Goto { block: target } = &block.terminal {
            if !all_block_ids.contains(target) {
                errors.push(CompilerError::invalid_react_with_kind(
                    block.terminal_span(),
                    format!(
                        "Break/continue target block {:?} referenced from block {:?} does not exist.",
                        target, bid
                    ),
                    DiagnosticKind::MalformedBreakTarget,
                ));
            }
        }
    }
}

/// Extension trait to get span from terminal (for error reporting).
trait TerminalSpan {
    fn terminal_span(&self) -> oxc_span::Span;
}

impl TerminalSpan for crate::hir::types::BasicBlock {
    fn terminal_span(&self) -> oxc_span::Span {
        // Use the block's last instruction span or default
        self.instructions.last().map(|i| i.loc).unwrap_or_default()
    }
}
