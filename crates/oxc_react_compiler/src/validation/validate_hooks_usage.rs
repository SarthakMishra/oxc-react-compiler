use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::globals::is_hook_name;
use crate::hir::types::{BlockId, HIR, InstructionValue, Terminal};
use rustc_hash::FxHashSet;

/// Validate that hooks are called according to the Rules of Hooks:
/// 1. Hooks must be called at the top level (not inside conditions/loops)
/// 2. Hooks must be called in the same order every render
/// 3. Hooks must not be referenced as normal values (must be called)
pub fn validate_hooks_usage(hir: &HIR, errors: &mut ErrorCollector) {
    // Track which blocks are inside conditionals/loops
    let conditional_blocks = find_conditional_blocks(hir);

    // Collect identifier IDs that are used as hook callees — these are valid hook usages.
    // In SSA form, `useState(0)` decomposes into `t0 = LoadLocal(useState); t1 = Call(t0, ...)`,
    // so we need to track the callee's identifier ID, not the lvalue's.
    let mut hook_callee_ids: FxHashSet<crate::hir::types::IdentifierId> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                // Track the callee identifier ID — this is the LoadLocal/LoadGlobal
                // result that is being used as a function call target.
                hook_callee_ids.insert(callee.identifier.id);
            }
        }
    }

    for (block_id, block) in &hir.blocks {
        for instr in &block.instructions {
            // Rule 1: Hooks called conditionally
            if let InstructionValue::CallExpression { callee, .. } = &instr.value
                && let Some(name) = &callee.identifier.name
                && is_hook_name(name)
                && conditional_blocks.contains(block_id)
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    format!(
                        "React Hook \"{name}\" is called conditionally. \
                                 Hooks must be called in the exact same order in every render."
                    ),
                    DiagnosticKind::HooksViolation,
                ));
            }

            // Rule 1b: Method calls that look like hooks (e.g., Foo.useFoo())
            if let InstructionValue::MethodCall { property, .. } = &instr.value
                && is_hook_name(property)
                && conditional_blocks.contains(block_id)
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    format!(
                        "React Hook \"{property}\" is called conditionally. \
                                 Hooks must be called in the exact same order in every render."
                    ),
                    DiagnosticKind::HooksViolation,
                ));
            }

            // Rule 3: Hooks referenced as values (not called)
            // Check for instructions that load a hook name without calling it
            match &instr.value {
                InstructionValue::LoadLocal { place }
                | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name
                        && is_hook_name(name)
                        && !hook_callee_ids.contains(&instr.lvalue.identifier.id)
                    {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            "Hooks may not be referenced as normal values, \
                             they must be called. See https://react.dev/reference/rules/react-calls-components-and-hooks".to_string(),
                            DiagnosticKind::HooksViolation,
                        ));
                    }
                }
                InstructionValue::PropertyLoad { property, .. } => {
                    if is_hook_name(property)
                        && !hook_callee_ids.contains(&instr.lvalue.identifier.id)
                    {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            "Hooks may not be referenced as normal values, \
                             they must be called. See https://react.dev/reference/rules/react-calls-components-and-hooks".to_string(),
                            DiagnosticKind::HooksViolation,
                        ));
                    }
                }
                _ => {}
            }
        }
    }
}

/// Find blocks that are inside conditional or loop constructs.
///
/// This performs a transitive closure: if block A is conditional and its
/// terminal leads to block B, then B is also conditional.
fn find_conditional_blocks(hir: &HIR) -> FxHashSet<BlockId> {
    let mut conditional = FxHashSet::default();

    // Direct children of conditional/loop terminals
    for (_, block) in &hir.blocks {
        match &block.terminal {
            Terminal::If { consequent, alternate, .. } => {
                conditional.insert(*consequent);
                conditional.insert(*alternate);
                // Transitively mark blocks reachable from conditional branches
                mark_reachable(hir, *consequent, &mut conditional);
                mark_reachable(hir, *alternate, &mut conditional);
            }
            Terminal::Switch { cases, .. } => {
                for case in cases {
                    conditional.insert(case.block);
                    mark_reachable(hir, case.block, &mut conditional);
                }
            }
            Terminal::For { body, .. }
            | Terminal::ForOf { body, .. }
            | Terminal::ForIn { body, .. } => {
                conditional.insert(*body);
                mark_reachable(hir, *body, &mut conditional);
            }
            Terminal::While { body, .. } | Terminal::DoWhile { body, .. } => {
                conditional.insert(*body);
                mark_reachable(hir, *body, &mut conditional);
            }
            Terminal::Ternary { consequent, alternate, .. } => {
                conditional.insert(*consequent);
                conditional.insert(*alternate);
            }
            Terminal::Optional { consequent, .. } => {
                conditional.insert(*consequent);
            }
            Terminal::Logical { left, right, .. } => {
                conditional.insert(*left);
                conditional.insert(*right);
            }
            _ => {}
        }
    }

    conditional
}

/// Transitively mark blocks reachable from a given block via Goto terminals.
fn mark_reachable(hir: &HIR, start: BlockId, visited: &mut FxHashSet<BlockId>) {
    if !visited.insert(start) {
        return; // Already visited
    }

    if let Some(block) = hir.blocks.iter().find(|(id, _)| *id == start).map(|(_, b)| b) {
        match &block.terminal {
            Terminal::Goto { block: next } => {
                mark_reachable(hir, *next, visited);
            }
            // Don't follow terminals that exit the conditional context
            // (e.g., fallthrough goes back to the main flow)
            _ => {}
        }
    }
}
