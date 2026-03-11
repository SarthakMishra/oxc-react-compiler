#![allow(dead_code)]

use crate::hir::types::{HIR, IdentifierId, InstructionValue, Primitive};
use rustc_hash::FxHashMap;

/// Propagate known constant values through the HIR.
///
/// For each instruction whose lvalue is assigned a `Primitive`, record
/// `lvalue.identifier.id -> constant_value`. Then replace subsequent
/// `LoadLocal` of that identifier with the constant.
///
/// This is a simple forward dataflow pass (not a full lattice-based analysis).
/// It does NOT propagate across function boundaries.
pub fn constant_propagation(hir: &mut HIR) {
    // Phase 1: Collect constants (identifiers assigned exactly one constant value)
    let constants = collect_constants(hir);

    if constants.is_empty() {
        return;
    }

    // Phase 2: Replace LoadLocal with Primitive where possible
    for (_, block) in hir.blocks.iter_mut() {
        for instr in &mut block.instructions {
            if let InstructionValue::LoadLocal { place } = &instr.value {
                if let Some(constant) = constants.get(&place.identifier.id) {
                    instr.value = InstructionValue::Primitive { value: constant.clone() };
                }
            }
        }
    }
}

/// Collect identifiers that are assigned exactly one constant value across
/// all blocks. If an identifier is assigned more than once with different
/// values, or assigned a non-constant, it is excluded.
fn collect_constants(hir: &HIR) -> FxHashMap<IdentifierId, Primitive> {
    // None means "assigned but not a single constant" (poisoned).
    let mut constants: FxHashMap<IdentifierId, Option<Primitive>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let id = instr.lvalue.identifier.id;
            match &instr.value {
                InstructionValue::Primitive { value } => {
                    match constants.get(&id) {
                        None => {
                            constants.insert(id, Some(value.clone()));
                        }
                        Some(Some(existing)) if *existing == *value => {
                            // Same constant, OK
                        }
                        _ => {
                            // Multiple different values -> not constant
                            constants.insert(id, None);
                        }
                    }
                }
                InstructionValue::StoreLocal { lvalue, value, .. } => {
                    // The lvalue target gets a non-primitive assignment (the value
                    // comes from another place, not a literal). Mark the lvalue's
                    // identifier as non-constant.
                    let target_id = lvalue.identifier.id;
                    constants.insert(target_id, None);
                    // Also mark the instruction's own lvalue
                    let _ = value;
                    constants.insert(id, None);
                }
                // Any other instruction that writes to the lvalue invalidates it
                // as a constant candidate.
                _ => {
                    constants.insert(id, None);
                }
            }
        }
    }

    constants.into_iter().filter_map(|(id, val)| val.map(|v| (id, v))).collect()
}
