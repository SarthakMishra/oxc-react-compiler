#![allow(dead_code)]

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue};

/// Validate that PascalCase functions are not called as regular functions.
///
/// In React, PascalCase names are components and should be rendered as JSX
/// elements (`<Component />`) rather than called directly (`Component()`).
/// Direct calls bypass React's reconciliation and can cause bugs with hooks.
pub fn validate_no_capitalized_calls(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                if let Some(name) = &callee.identifier.name {
                    // Skip hook calls (useXxx) — those are valid PascalCase-like calls
                    if name.starts_with("use")
                        && name.as_bytes().get(3).map_or(false, |b| b.is_ascii_uppercase())
                    {
                        continue;
                    }

                    // Check if the callee starts with an uppercase letter
                    if name.as_bytes().first().map_or(false, |b| b.is_ascii_uppercase()) {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            format!(
                                "\"{}\" is a component and cannot be called directly. \
                                 Use JSX syntax (<{} />) instead of calling it as a function.",
                                name, name
                            ),
                            DiagnosticKind::CapitalizedCalls,
                        ));
                    }
                }
            }
        }
    }
}
