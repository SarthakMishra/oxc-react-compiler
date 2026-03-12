#![allow(dead_code)]

//! Validate that known impure functions are not called during render.
//!
//! Functions like `console.log`, `Math.random`, `Date.now`, etc. have side
//! effects or non-deterministic behavior that can cause tearing when called
//! during render in concurrent React.

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue};

/// Known impure global function calls (object.method patterns).
const IMPURE_METHOD_CALLS: &[(&str, &str)] = &[
    ("console", "log"),
    ("console", "warn"),
    ("console", "error"),
    ("console", "info"),
    ("console", "debug"),
    ("console", "trace"),
    ("console", "table"),
    ("console", "dir"),
    ("console", "group"),
    ("console", "groupEnd"),
    ("console", "time"),
    ("console", "timeEnd"),
    ("console", "count"),
    ("console", "assert"),
    ("Math", "random"),
    ("Date", "now"),
];

/// Known impure standalone functions.
const IMPURE_FUNCTIONS: &[&str] = &[
    "alert",
    "confirm",
    "prompt",
    "fetch",
    "setTimeout",
    "setInterval",
    "clearTimeout",
    "clearInterval",
    "requestAnimationFrame",
    "cancelAnimationFrame",
];

/// Validate that no known impure functions are called during render.
pub fn validate_no_impure_functions_in_render(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::CallExpression { callee, .. } => {
                    if let Some(name) = callee.identifier.name.as_deref() {
                        if IMPURE_FUNCTIONS.contains(&name) {
                            errors.push(CompilerError::invalid_react_with_kind(
                                instr.loc,
                                format!(
                                    "Calling impure function `{name}` during render. \
                                     This may cause non-deterministic behavior in concurrent React."
                                ),
                                DiagnosticKind::ImpureFunctionInRender,
                            ));
                        }
                    }
                }
                InstructionValue::MethodCall { receiver, property, .. } => {
                    if let Some(obj_name) = receiver.identifier.name.as_deref() {
                        if IMPURE_METHOD_CALLS
                            .iter()
                            .any(|(o, m)| *o == obj_name && *m == property.as_str())
                        {
                            errors.push(CompilerError::invalid_react_with_kind(
                                instr.loc,
                                format!(
                                    "Calling impure function `{obj_name}.{property}` during render. \
                                     This may cause non-deterministic behavior in concurrent React."
                                ),
                                DiagnosticKind::ImpureFunctionInRender,
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
