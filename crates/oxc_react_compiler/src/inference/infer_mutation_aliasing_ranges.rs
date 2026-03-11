#![allow(dead_code)]

use crate::hir::types::{InstructionId, MutableRange, HIR};

/// Compute mutable ranges for all identifiers.
///
/// Uses the effects computed by `infer_mutation_aliasing_effects` to determine
/// the instruction range during which each value is being mutated.
///
/// - `start`: instruction that creates the value
/// - `end`: last instruction that mutates the value (transitively through aliases)
pub fn infer_mutation_aliasing_ranges(hir: &mut HIR) {
    // For each identifier, find the instruction that creates it (start)
    // and the last instruction that mutates it (end)
    for (_, block) in hir.blocks.iter_mut() {
        for instr in &mut block.instructions {
            let id = instr.id;
            // Set mutable range start to this instruction
            if instr.lvalue.identifier.mutable_range.start == InstructionId(0) {
                instr.lvalue.identifier.mutable_range = MutableRange {
                    start: id,
                    end: InstructionId(id.0 + 1), // default: mutable for one instruction
                };
            }
        }
    }

    // TODO: Full implementation requires:
    // 1. Build a map of identifier -> all mutation sites (from effects)
    // 2. For each identifier, compute the range [creation, last_mutation]
    // 3. Handle transitive mutations through aliases
    // 4. Handle mutations through captured references in functions
}
