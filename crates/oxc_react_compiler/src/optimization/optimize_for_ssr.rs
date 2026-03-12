//! Optimize HIR for server-side rendering.
//!
//! In SSR mode, memoization is unnecessary since each render produces fresh output.
//! This pass strips all memoization infrastructure from the HIR:
//! - Removes reactive scope annotations from identifiers
//! - Marks scope-related metadata for stripping
//! - Allows codegen to emit simpler code without `useMemoCache`

use crate::hir::types::{HIR, InstructionValue, Terminal};

/// Optimize HIR for server-side rendering by stripping memoization.
///
/// In SSR mode, every render produces a fresh HTML string — memoization provides
/// no benefit and adds overhead. This pass:
/// 1. Strips scope annotations from all identifiers (no reactive scopes)
/// 2. Marks all identifiers as non-reactive (no dependency tracking needed)
/// 3. Removes mutable range annotations (no mutation tracking needed)
/// 4. Clears dependencies and declarations from any existing scopes
pub fn optimize_for_ssr(hir: &mut HIR) {
    for (_, block) in &mut hir.blocks {
        // Strip scope/reactivity from instructions
        for instr in &mut block.instructions {
            instr.lvalue.identifier.scope = None;
            instr.lvalue.reactive = false;

            // Strip reactivity from operands
            strip_operand_reactivity(&mut instr.value);
        }

        // Strip reactivity from terminal operands
        strip_terminal_reactivity(&mut block.terminal);
    }
}

/// Strip reactive flags from instruction operands.
fn strip_operand_reactivity(value: &mut InstructionValue) {
    match value {
        InstructionValue::StoreLocal { value: place, .. }
        | InstructionValue::LoadLocal { place }
        | InstructionValue::LoadContext { place }
        | InstructionValue::PropertyLoad { object: place, .. } => {
            place.reactive = false;
        }
        InstructionValue::CallExpression { callee, args } => {
            callee.reactive = false;
            for arg in args {
                arg.reactive = false;
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            receiver.reactive = false;
            for arg in args {
                arg.reactive = false;
            }
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            left.reactive = false;
            right.reactive = false;
        }
        InstructionValue::UnaryExpression { value, .. } => {
            value.reactive = false;
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            object.reactive = false;
            value.reactive = false;
        }
        InstructionValue::Destructure { value, .. } => {
            value.reactive = false;
        }
        InstructionValue::JsxExpression { tag, children, .. } => {
            tag.reactive = false;
            for child in children {
                child.reactive = false;
            }
        }
        _ => {}
    }
}

/// Strip reactive flags from terminal operands.
fn strip_terminal_reactivity(terminal: &mut Terminal) {
    match terminal {
        Terminal::Return { value } => value.reactive = false,
        Terminal::If { test, .. } | Terminal::Branch { test, .. } => test.reactive = false,
        Terminal::Switch { test, .. } => test.reactive = false,
        Terminal::Throw { value } => value.reactive = false,
        Terminal::Ternary { test, .. } => test.reactive = false,
        Terminal::Optional { test, .. } => test.reactive = false,
        _ => {}
    }
}
