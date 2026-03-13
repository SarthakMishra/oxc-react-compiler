//! Validate that known impure functions are not called during render.
//!
//! Functions like `console.log`, `Math.random`, `Date.now`, etc. have side
//! effects or non-deterministic behavior that can cause tearing when called
//! during render in concurrent React.

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, IdentifierId, InstructionValue};
use rustc_hash::FxHashMap;

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
    ("performance", "now"),
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
    // Build id-to-name map for resolving SSA temporaries
    let id_to_name = build_name_map(hir);

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::CallExpression { callee, .. } => {
                    let name = callee
                        .identifier
                        .name
                        .as_deref()
                        .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));
                    if let Some(name) = name
                        && IMPURE_FUNCTIONS.contains(&name)
                    {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            format!(
                                "Cannot call impure function during render. \
                                 `{name}` is an impure function. \
                                 This may cause non-deterministic behavior in concurrent React."
                            ),
                            DiagnosticKind::ImpureFunctionInRender,
                        ));
                    }
                }
                InstructionValue::MethodCall { receiver, property, .. } => {
                    let obj_name =
                        receiver.identifier.name.as_deref().or_else(|| {
                            id_to_name.get(&receiver.identifier.id).map(String::as_str)
                        });
                    if let Some(obj_name) = obj_name
                        && IMPURE_METHOD_CALLS
                            .iter()
                            .any(|(o, m)| *o == obj_name && *m == property.as_str())
                    {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            format!(
                                "Cannot call impure function during render. \
                                 `{obj_name}.{property}` is an impure function. \
                                 This may cause non-deterministic behavior in concurrent React."
                            ),
                            DiagnosticKind::ImpureFunctionInRender,
                        ));
                    }
                }
                _ => {}
            }
        }
    }
}

/// Build a map from identifier ID → name for SSA resolution.
fn build_name_map(hir: &HIR) -> FxHashMap<IdentifierId, String> {
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadGlobal { binding } => {
                    id_to_name.insert(instr.lvalue.identifier.id, binding.name.clone());
                }
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name.clone());
                    }
                }
                _ => {}
            }
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_name.entry(instr.lvalue.identifier.id).or_insert_with(|| name.clone());
            }
        }
    }

    id_to_name
}
