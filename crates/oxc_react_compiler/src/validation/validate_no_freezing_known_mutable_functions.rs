use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{AliasingEffect, HIR, IdentifierId, InstructionValue};
use rustc_hash::FxHashSet;

/// Validate that known mutable function references are not frozen.
///
/// Some functions are inherently mutable (e.g., setState dispatchers, refs).
/// Freezing them would violate their contract. This pass checks for `Freeze`
/// aliasing effects applied to places that are known to be mutable functions.
pub fn validate_no_freezing_known_mutable_functions(hir: &HIR, errors: &mut ErrorCollector) {
    // Step 1: Collect identifiers that are known mutable functions.
    // These include setState dispatchers and dispatch functions from useReducer.
    let mutable_fn_ids = collect_known_mutable_function_ids(hir);

    // Step 2: Walk all instructions and check aliasing effects for Freeze on those ids.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(effects) = &instr.effects {
                for effect in effects {
                    if let AliasingEffect::Freeze { value, .. } = effect
                        && mutable_fn_ids.contains(&value.identifier.id)
                    {
                        let name = value.identifier.name.as_deref().unwrap_or("<unknown>");
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            format!(
                                "Cannot freeze mutable function \"{name}\". \
                                     This function reference is inherently mutable \
                                     and should not be frozen by the compiler."
                            ),
                            DiagnosticKind::ImmutabilityViolation,
                        ));
                    }
                }
            }
        }
    }
}

/// Collect identifier IDs for functions known to be mutable (setState, dispatch, etc.).
fn collect_known_mutable_function_ids(hir: &HIR) -> FxHashSet<IdentifierId> {
    let mut mutable_ids = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Look for destructuring patterns from useState/useReducer calls
            // that extract the setter/dispatch function.
            if let InstructionValue::StoreLocal { lvalue, .. } = &instr.value
                && let Some(name) = &lvalue.identifier.name
            {
                // setState-like: setX where X starts with uppercase
                if name.starts_with("set")
                    && name.len() > 3
                    && name.as_bytes()[3].is_ascii_uppercase()
                {
                    mutable_ids.insert(lvalue.identifier.id);
                }
                // dispatch from useReducer
                if name == "dispatch" {
                    mutable_ids.insert(lvalue.identifier.id);
                }
            }
        }
    }

    mutable_ids
}
