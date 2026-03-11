#![allow(dead_code)]

use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::HIR;

/// Assert that all mutable ranges are valid (end > start).
///
/// This is a debug assertion pass that catches internal compiler bugs.
pub fn assert_valid_mutable_ranges(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let range = instr.lvalue.identifier.mutable_range;
            if range.end.0 < range.start.0 {
                errors.push(CompilerError::invariant(
                    instr.loc,
                    format!(
                        "Invalid mutable range [{}, {}) for identifier {:?}",
                        range.start.0, range.end.0, instr.lvalue.identifier.name,
                    ),
                ));
            }
        }
        for phi in &block.phis {
            let range = phi.place.identifier.mutable_range;
            if range.end.0 < range.start.0 {
                errors.push(CompilerError::invariant(
                    phi.place.loc,
                    format!(
                        "Invalid mutable range [{}, {}) for phi identifier {:?}",
                        range.start.0, range.end.0, phi.place.identifier.name,
                    ),
                ));
            }
        }
    }
}
