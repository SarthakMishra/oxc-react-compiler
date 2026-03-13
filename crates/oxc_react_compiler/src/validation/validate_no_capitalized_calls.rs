use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::globals::is_hook_name;
use crate::hir::types::{HIR, IdentifierId, InstructionValue};
use rustc_hash::FxHashMap;

/// Validate that PascalCase functions are not called as regular functions.
///
/// In React, PascalCase names are components and should be rendered as JSX
/// elements (`<Component />`) rather than called directly (`Component()`).
/// Direct calls bypass React's reconciliation and can cause bugs with hooks.
pub fn validate_no_capitalized_calls(hir: &HIR, errors: &mut ErrorCollector) {
    // Build a map from identifier ID → resolved name for SSA resolution.
    // Globals like `SomeFunc()` decompose to `t0 = LoadGlobal(SomeFunc); Call(t0)`.
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

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::CallExpression { callee, .. } => {
                    let name = callee
                        .identifier
                        .name
                        .clone()
                        .or_else(|| id_to_name.get(&callee.identifier.id).cloned());

                    if let Some(name) = name {
                        // Skip hook calls (useXxx) — those are valid PascalCase-like calls
                        if is_hook_name(&name) {
                            continue;
                        }

                        // Check if the callee starts with an uppercase letter
                        if name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
                            errors.push(CompilerError::invalid_react_with_kind(
                                instr.loc,
                                "Capitalized functions are reserved for components, which \
                                     must be invoked with JSX. If this is a component, render \
                                     it using JSX syntax instead of calling it directly."
                                    .to_string(),
                                DiagnosticKind::CapitalizedCalls,
                            ));
                        }
                    }
                }
                InstructionValue::MethodCall { property, .. } => {
                    // Check method calls like `someGlobal.SomeFunc()`
                    if !is_hook_name(property)
                        && property.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                    {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            "Capitalized functions are reserved for components, which \
                                 must be invoked with JSX. If this is a component, render \
                                 it using JSX syntax instead of calling it directly."
                                .to_string(),
                            DiagnosticKind::CapitalizedCalls,
                        ));
                    }
                }
                _ => {}
            }
        }
    }
}
