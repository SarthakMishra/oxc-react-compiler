use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::globals::is_hook_name;
use crate::hir::types::{BlockId, HIR, IdentifierId, InstructionValue, Terminal};
use rustc_hash::{FxHashMap, FxHashSet};

/// Validate that hooks are called according to the Rules of Hooks:
/// 1. Hooks must be called at the top level (not inside conditions/loops)
/// 2. Hooks must be called in the same order every render
/// 3. Hooks must not be referenced as normal values (must be called)
/// 4. Hooks must not be called inside nested function expressions
pub fn validate_hooks_usage(hir: &HIR, errors: &mut ErrorCollector) {
    // Track which blocks are inside conditionals/loops
    let conditional_blocks = find_conditional_blocks(hir);

    // Build a map from identifier ID → resolved name.
    // In SSA form, `useHook()` decomposes into `t0 = LoadGlobal(useHook); t1 = Call(t0, ...)`.
    // The callee (t0) has name: None, so we resolve it via the LoadGlobal/LoadLocal binding name.
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();

    // Collect identifier IDs that are used as hook callees — these are valid hook usages.
    let mut hook_callee_ids: FxHashSet<IdentifierId> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Populate id_to_name from load instructions
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

            // Also use the identifier's own name if set
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_name.entry(instr.lvalue.identifier.id).or_insert_with(|| name.clone());
            }

            // Track callee IDs for Rule 3
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                hook_callee_ids.insert(callee.identifier.id);
            }
        }
    }

    // Helper: resolve the effective name for an identifier
    let resolve_name = |id: IdentifierId, name: &Option<String>| -> Option<String> {
        name.clone().or_else(|| id_to_name.get(&id).cloned())
    };

    for (block_id, block) in &hir.blocks {
        for instr in &block.instructions {
            // Rule 1: Hooks called conditionally
            if let InstructionValue::CallExpression { callee, .. } = &instr.value
                && let Some(name) = resolve_name(callee.identifier.id, &callee.identifier.name)
                && is_hook_name(&name)
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
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
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
                InstructionValue::LoadGlobal { binding } => {
                    if is_hook_name(&binding.name)
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

    // Rule 4: Hooks must not be called inside nested function expressions
    check_hooks_in_nested_functions(hir, errors);
}

/// Check for hook calls inside nested function expressions and object methods.
///
/// Upstream: Hooks must be called at the top level in the body of a function
/// component or custom hook, and may not be called within function expressions.
fn check_hooks_in_nested_functions(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let nested_hir: Option<&HIR> = match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. } => {
                    Some(&lowered_func.body)
                }
                InstructionValue::ObjectMethod { lowered_func } => Some(&lowered_func.body),
                _ => None,
            };
            if let Some(body) = nested_hir {
                check_nested_hir_for_hook_calls(body, errors);
            }
        }
    }
}

/// Recursively scan a nested HIR for hook calls and emit errors.
fn check_nested_hir_for_hook_calls(body: &HIR, errors: &mut ErrorCollector) {
    // Build name-resolution map for this nested body
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();

    for (_, block) in &body.blocks {
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

    for (_, block) in &body.blocks {
        for instr in &block.instructions {
            // Check CallExpression for hook calls
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                let name = callee
                    .identifier
                    .name
                    .clone()
                    .or_else(|| id_to_name.get(&callee.identifier.id).cloned());
                if let Some(name) = name
                    && is_hook_name(&name)
                {
                    errors.push(CompilerError::invalid_react_with_kind(
                        instr.loc,
                        format!(
                            "Hooks must be called at the top level in the body of a function \
                             component or custom hook, and may not be called within function \
                             expressions. See the Rules of Hooks \
                             (https://react.dev/warnings/invalid-hook-call-warning). \
                             Cannot call {name} within a function expression."
                        ),
                        DiagnosticKind::HooksViolation,
                    ));
                }
            }

            // Check MethodCall for hook-named methods
            if let InstructionValue::MethodCall { property, .. } = &instr.value
                && is_hook_name(property)
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    format!(
                        "Hooks must be called at the top level in the body of a function \
                         component or custom hook, and may not be called within function \
                         expressions. See the Rules of Hooks \
                         (https://react.dev/warnings/invalid-hook-call-warning). \
                         Cannot call {property} within a function expression."
                    ),
                    DiagnosticKind::HooksViolation,
                ));
            }

            // Recurse into deeper nested functions
            let nested_hir: Option<&HIR> = match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. } => {
                    Some(&lowered_func.body)
                }
                InstructionValue::ObjectMethod { lowered_func } => Some(&lowered_func.body),
                _ => None,
            };
            if let Some(nested_body) = nested_hir {
                check_nested_hir_for_hook_calls(nested_body, errors);
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
