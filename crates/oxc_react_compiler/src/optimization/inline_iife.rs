#![allow(dead_code)]

use crate::hir::types::{BlockId, FunctionExprType, HIR, InstructionValue, Param, Place, Terminal};

/// Inline immediately-invoked function expressions (IIFEs).
///
/// Pattern: `(() => { ... })()` or `(function() { ... })()`
///
/// Detects `CallExpression` where the callee was defined by a `FunctionExpression`
/// in the immediately preceding instruction, and that function has a single
/// return block with no complex control flow. For these simple cases, inline
/// the function body directly.
///
/// More complex IIFEs (with closures, multiple returns, or nested control flow)
/// are left as-is for safety.
pub fn inline_iife(hir: &mut HIR) {
    // Step 1: Find simple IIFE candidates.
    // A simple IIFE is a CallExpression whose callee is the lvalue of a
    // FunctionExpression in the immediately preceding instruction, where:
    // - The function has a single block (entry block only)
    // - The function has no context variables (no captures)
    // - The block terminates with a Return

    let mut next_block_id = hir.blocks.iter().map(|(id, _)| id.0).max().unwrap_or(0) + 1;

    // Collect IIFE sites: (block_idx, call_instr_idx, func_instr_idx)
    let mut iife_sites: Vec<(usize, usize, usize)> = Vec::new();

    for (block_idx, (_, block)) in hir.blocks.iter().enumerate() {
        for i in 1..block.instructions.len() {
            let prev = &block.instructions[i - 1];
            let curr = &block.instructions[i];

            // Check if curr is a CallExpression whose callee matches prev's lvalue.
            let callee_matches =
                if let InstructionValue::CallExpression { ref callee, .. } = curr.value {
                    callee.identifier.id == prev.lvalue.identifier.id
                } else {
                    false
                };

            if !callee_matches {
                continue;
            }

            // Check if prev is a simple FunctionExpression.
            if let InstructionValue::FunctionExpression { ref lowered_func, .. } = prev.value {
                // Only inline if the function has a single block and no context.
                if lowered_func.context.is_empty() && lowered_func.body.blocks.len() == 1 {
                    if let Some((_, entry_block)) = lowered_func.body.blocks.first() {
                        if matches!(entry_block.terminal, Terminal::Return { .. }) {
                            iife_sites.push((block_idx, i, i - 1));
                        }
                    }
                }
            }
        }
    }

    // Step 2: Inline each IIFE (process in reverse to keep indices valid).
    for (block_idx, call_idx, func_idx) in iife_sites.into_iter().rev() {
        let block = &mut hir.blocks[block_idx].1;

        // Extract the function expression and call arguments.
        let func_instr = block.instructions.remove(func_idx);
        // call_idx shifted by -1 after removal.
        let call_instr_idx = call_idx - 1;
        let call_instr = &mut block.instructions[call_instr_idx];

        let InstructionValue::FunctionExpression { lowered_func, .. } = func_instr.value else {
            continue;
        };

        let InstructionValue::CallExpression { ref args, .. } = call_instr.value else {
            continue;
        };

        let args = args.clone();
        let entry_block = &lowered_func.body.blocks[0].1;

        // Build parameter -> argument mapping.
        let mut param_to_arg: Vec<(Place, Place)> = Vec::new();
        for (param, arg) in lowered_func.params.iter().zip(args.iter()) {
            match param {
                Param::Identifier(place) => {
                    param_to_arg.push((place.clone(), arg.clone()));
                }
                Param::Spread(_) => {
                    // Can't simply inline spread params; bail.
                    continue;
                }
            }
        }

        // Get the return value from the function's terminal.
        let return_value = if let Terminal::Return { ref value } = entry_block.terminal {
            value.clone()
        } else {
            continue;
        };

        // Replace the call instruction with a reference to the return value.
        // The inlined function's instructions will be inserted before this point.
        let call_lvalue = call_instr.lvalue.clone();
        call_instr.value = InstructionValue::LoadLocal { place: return_value };

        // Insert the function body's instructions before the (now-transformed) call.
        // Also insert StoreLocal for parameter -> argument bindings.
        let mut new_instrs: Vec<crate::hir::types::Instruction> = Vec::new();

        // Parameter bindings.
        for (param_place, arg_place) in &param_to_arg {
            new_instrs.push(crate::hir::types::Instruction {
                id: func_instr.id, // Reuse IDs since this is a replacement.
                lvalue: param_place.clone(),
                value: InstructionValue::LoadLocal { place: arg_place.clone() },
                loc: func_instr.loc,
                effects: None,
            });
        }

        // Function body instructions.
        for body_instr in &entry_block.instructions {
            new_instrs.push(body_instr.clone());
        }

        // Insert before the call instruction.
        let insert_point = call_instr_idx;
        for (offset, instr) in new_instrs.into_iter().enumerate() {
            block.instructions.insert(insert_point + offset, instr);
        }
    }
}
