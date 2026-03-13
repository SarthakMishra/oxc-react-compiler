// DIVERGENCE: Upstream detects frozen-value mutations inside
// InferMutableRanges.ts as part of the abstract interpretation.
// Our port uses a post-effects validation pass that tracks freeze/mutate
// by variable name, because our HIR creates fresh IdentifierIds per
// Place reference — there is no single stable ID across references.

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{AliasingEffect, HIR, IdentifierId, InstructionValue};
use rustc_hash::{FxHashMap, FxHashSet};

const FROZEN_MUTATION_ERROR: &str = "This value cannot be modified. Modifying a value used \
     previously in JSX is not allowed. Consider moving the \
     modification before the JSX expression.";

/// Detect mutations to values that have been frozen (used in JSX or passed to hooks).
///
/// After `infer_mutation_aliasing_effects` runs, each instruction has computed effects.
/// This pass walks instructions in program order and tracks which variable names have
/// been frozen. Any mutation to a frozen variable is an error.
///
/// Since the HIR creates fresh IdentifierIds for every Place reference, we track by
/// variable name using a lvalue-ID → source-variable-name mapping built from
/// `LoadLocal`/`LoadContext` instructions.
pub fn validate_no_mutation_after_freeze(hir: &HIR, errors: &mut ErrorCollector) {
    // Build lvalue_id → source variable name map (borrows from HIR, no clones).
    // When LoadLocal { place: x_ref } → lvalue_temp, this maps lvalue_temp's ID to "x".
    let mut id_to_source_name: FxHashMap<IdentifierId, &str> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_source_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::StoreContext { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_source_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_source_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                _ => {}
            }
            // Fallback: map lvalue's own name (for instruction types not matched above)
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_source_name.insert(instr.lvalue.identifier.id, name);
            }
        }
    }

    // Walk instructions in program order, tracking frozen variable names
    let mut frozen_names: FxHashSet<&str> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // First: process freeze effects to update frozen_names
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    match effect {
                        AliasingEffect::Freeze { value, .. }
                        | AliasingEffect::ImmutableCapture { from: value, .. } => {
                            if let Some(name) = id_to_source_name.get(&value.identifier.id) {
                                frozen_names.insert(name);
                            }
                            if let Some(name) = &value.identifier.name {
                                frozen_names.insert(name);
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Then: check instruction-level mutations (MethodCall, PropertyStore, etc.)
            if check_instruction_mutation(instr, &id_to_source_name, &frozen_names) {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    FROZEN_MUTATION_ERROR,
                    DiagnosticKind::ImmutabilityViolation,
                ));
                return;
            }

            // Also check explicit Mutate effects from the aliasing pass
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    let mutated_name = match effect {
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value }
                        | AliasingEffect::MutateFrozen { place: value, .. } => {
                            id_to_source_name.get(&value.identifier.id).copied()
                        }
                        _ => None,
                    };

                    if let Some(name) = mutated_name
                        && frozen_names.contains(name)
                    {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            FROZEN_MUTATION_ERROR,
                            DiagnosticKind::ImmutabilityViolation,
                        ));
                        return;
                    }
                }
            }
        }
    }
}

/// Check if an instruction directly mutates a frozen value via its instruction value.
/// This catches cases where the aliasing effects don't generate explicit Mutate effects,
/// such as MethodCall receivers (x.push()) and PropertyStore (x.prop = ...).
fn check_instruction_mutation(
    instr: &crate::hir::types::Instruction,
    id_to_source_name: &FxHashMap<IdentifierId, &str>,
    frozen_names: &FxHashSet<&str>,
) -> bool {
    let check_frozen = |id: &IdentifierId| -> bool {
        id_to_source_name.get(id).is_some_and(|name| frozen_names.contains(name))
    };

    match &instr.value {
        // x.push(...), x.splice(...), etc. — method call may mutate receiver
        InstructionValue::MethodCall { receiver, .. } => check_frozen(&receiver.identifier.id),
        // x.prop = value — direct property mutation
        InstructionValue::PropertyStore { object, .. }
        | InstructionValue::ComputedStore { object, .. } => check_frozen(&object.identifier.id),
        // delete x[i]
        InstructionValue::PropertyDelete { object, .. }
        | InstructionValue::ComputedDelete { object, .. } => check_frozen(&object.identifier.id),
        // ++x, x++
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => check_frozen(&lvalue.identifier.id),
        _ => false,
    }
}
