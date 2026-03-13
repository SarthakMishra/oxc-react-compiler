//! Validate that no blocklisted imports are used in compiled code.
//!
//! Checks for `LoadGlobal` instructions whose binding names match
//! configured blocklisted import sources. This prevents specific
//! libraries from being used in compiled components.

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue};

/// Validate that no blocklisted imports appear in the HIR.
///
/// Scans for `LoadGlobal` instructions whose names match entries in
/// the blocklist. This catches both direct usage and indirect references
/// to blocklisted modules.
pub fn validate_blocklisted_imports(hir: &HIR, blocklist: &[String], errors: &mut ErrorCollector) {
    if blocklist.is_empty() {
        return;
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::LoadGlobal { binding } = &instr.value
                && blocklist.iter().any(|b| b == &binding.name) {
                    errors.push(CompilerError::invalid_react_with_kind(
                        instr.loc,
                        format!(
                            "Import `{}` is blocklisted and cannot be used in compiled components.",
                            binding.name
                        ),
                        DiagnosticKind::BlocklistedImport,
                    ));
                }
        }
    }
}
