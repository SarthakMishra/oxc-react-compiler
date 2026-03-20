use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::globals::is_hook_name;
use crate::hir::types::{BlockId, HIR, IdentifierId, InstructionValue, Terminal};
use rustc_hash::{FxHashMap, FxHashSet};

/// Validate that hooks are called according to the Rules of Hooks:
/// 1. Hooks must be called at the top level (not inside conditions/loops)
/// 2. Hooks must be called in the same order every render
/// 3. Hooks must not be referenced as normal values (must be called)
/// 4. Hooks must not be called inside nested function expressions
/// 5. Hook-named callees must have stable identity across renders
#[expect(clippy::implicit_hasher)]
pub fn validate_hooks_usage(
    hir: &HIR,
    errors: &mut ErrorCollector,
    hook_aliases: &FxHashSet<String>,
) {
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

            // Track callee IDs for Rule 3 — IDs used in valid hook contexts:
            // - CallExpression callee (hook is being called)
            // - PropertyLoad object (accessing hook.name, hook.length, etc.)
            // - MethodCall receiver (hook.bind(), etc.)
            match &instr.value {
                InstructionValue::CallExpression { callee, .. } => {
                    hook_callee_ids.insert(callee.identifier.id);
                }
                InstructionValue::PropertyLoad { object, .. } => {
                    hook_callee_ids.insert(object.identifier.id);
                }
                InstructionValue::MethodCall { receiver, .. } => {
                    hook_callee_ids.insert(receiver.identifier.id);
                }
                _ => {}
            }
        }
    }

    // Helper: resolve the effective name for an identifier
    let resolve_name = |id: IdentifierId, name: &Option<String>| -> Option<String> {
        name.clone().or_else(|| id_to_name.get(&id).cloned())
    };

    // Helper: check if a name is a hook (either by convention or by alias)
    let is_hook = |name: &str| -> bool { is_hook_name(name) || hook_aliases.contains(name) };

    for (block_id, block) in &hir.blocks {
        for instr in &block.instructions {
            // Rule 1: Hooks called conditionally
            if let InstructionValue::CallExpression { callee, .. } = &instr.value
                && let Some(name) = resolve_name(callee.identifier.id, &callee.identifier.name)
                && is_hook(&name)
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
                && is_hook(property)
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
                        && is_hook(name)
                        && !hook_callee_ids.contains(&instr.lvalue.identifier.id)
                        && !hook_callee_ids.contains(&place.identifier.id)
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
                    if is_hook(&binding.name)
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
                    if is_hook(property) && !hook_callee_ids.contains(&instr.lvalue.identifier.id) {
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

    // Rule 5: Dynamic hook identity — hook-named callees whose value may change
    // between renders (e.g., `const useX = someFunc; useX()`)
    check_dynamic_hook_identity(hir, &id_to_name, &is_hook, errors);

    // Rule 4: Hooks must not be called inside nested function expressions
    check_hooks_in_nested_functions(hir, errors, hook_aliases);
}

/// Check for dynamic hook identity: when a hook-named callee's value comes from
/// a non-stable source (e.g., a function return value, a conditional assignment).
///
/// Upstream error: "Hooks must be the same function on every render, but this value
/// may change over time to a different function."
///
/// Examples that should error:
/// - `const useMedia = useVideoPlayer(); useMedia()` — hook return is not a stable hook
/// - `const useX = someFunction; useX()` — local reassignment is not stable
fn check_dynamic_hook_identity(
    hir: &HIR,
    id_to_name: &FxHashMap<IdentifierId, String>,
    is_hook: &dyn Fn(&str) -> bool,
    errors: &mut ErrorCollector,
) {
    // Build maps tracking whether each identifier/variable is a "stable hook source".
    // A stable hook source is:
    // - LoadGlobal of a hook name (e.g., useState imported globally)
    // - FunctionExpression (a locally defined function is stable)
    // - LoadLocal/LoadContext of a variable with known stability
    // An unstable source is:
    // - CallExpression return value (e.g., `const useX = useVideoPlayer()`)
    // - StoreLocal from a non-hook source
    // - Function parameters (props values change between renders)
    //
    // We track by BOTH identifier ID and variable name, because our HIR creates
    // fresh IdentifierIds per Place reference — tracking by ID alone would lose
    // stability info across SSA edges (StoreLocal lvalue vs LoadLocal place).
    let mut is_stable_hook_by_id: FxHashMap<IdentifierId, bool> = FxHashMap::default();
    let mut is_stable_hook_by_name: FxHashMap<String, bool> = FxHashMap::default();

    // Function parameters are NOT stable hook sources — they come from props
    // and may change between renders. Mark them unstable by name.
    // DIVERGENCE: Upstream tracks this through the type system (params have type
    // that indicates they're props). We use the convention that any param name
    // that looks hook-like is unstable, since it came from the caller.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // DeclareLocal for params happen at the start of the function
            if let InstructionValue::DeclareLocal { lvalue, .. } = &instr.value
                && let Some(name) = &lvalue.identifier.name
                && is_hook(name)
            {
                // Check if this is a parameter (appears in block 0 / entry)
                // Simple heuristic: if a DeclareLocal for a hook-like name has no
                // corresponding StoreLocal with a FunctionExpression or LoadGlobal
                // source, treat it as potentially unstable (could be a prop destructure)
                is_stable_hook_by_name.insert(name.clone(), false);
                is_stable_hook_by_id.insert(lvalue.identifier.id, false);
            }
        }
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadGlobal { binding } => {
                    let stable = is_hook(&binding.name);
                    is_stable_hook_by_id.insert(instr.lvalue.identifier.id, stable);
                    if stable {
                        is_stable_hook_by_name.insert(binding.name.clone(), true);
                    }
                }
                InstructionValue::FunctionExpression { .. } => {
                    // Locally defined functions are stable
                    is_stable_hook_by_id.insert(instr.lvalue.identifier.id, true);
                }
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    // Resolve stability: check by ID first, then by name
                    let source_stable = is_stable_hook_by_id
                        .get(&place.identifier.id)
                        .copied()
                        .or_else(|| {
                            place
                                .identifier
                                .name
                                .as_deref()
                                .and_then(|n| is_stable_hook_by_name.get(n).copied())
                        })
                        .unwrap_or_else(|| {
                            // If no stability info exists and the name is hook-like,
                            // it's likely an external/captured hook reference — treat as stable.
                            // But only if we haven't explicitly marked it unstable above.
                            place.identifier.name.as_deref().is_some_and(is_hook)
                        });
                    is_stable_hook_by_id.insert(instr.lvalue.identifier.id, source_stable);
                }
                InstructionValue::CallExpression { .. } => {
                    // Return values from calls are NOT stable hook references
                    is_stable_hook_by_id.insert(instr.lvalue.identifier.id, false);
                }
                InstructionValue::StoreLocal { value, lvalue, .. } => {
                    // Propagate stability from the stored value
                    let source_stable =
                        is_stable_hook_by_id.get(&value.identifier.id).copied().unwrap_or(false);
                    is_stable_hook_by_id.insert(instr.lvalue.identifier.id, source_stable);
                    // Also track by name so LoadLocal can resolve across SSA edges
                    if let Some(name) = &lvalue.identifier.name {
                        is_stable_hook_by_name.insert(name.clone(), source_stable);
                    }
                }
                _ => {}
            }
        }
    }

    // Now check all CallExpressions: if the callee has a hook-like name but is NOT stable,
    // emit a dynamic hook identity error.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                let callee_name = callee
                    .identifier
                    .name
                    .as_deref()
                    .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));

                if let Some(name) = callee_name
                    && is_hook(name)
                    && !is_stable_hook_by_id.get(&callee.identifier.id).copied().unwrap_or(true)
                {
                    errors.push(CompilerError::invalid_react_with_kind(
                        instr.loc,
                        format!(
                            "Hooks must be the same function on every render, but \"{name}\" \
                             may change over time to a different function. See the Rules of Hooks \
                             (https://react.dev/warnings/invalid-hook-call-warning)."
                        ),
                        DiagnosticKind::HooksViolation,
                    ));
                }
            }
        }
    }
}

/// Check for hook calls inside nested function expressions and object methods.
///
/// Upstream: Hooks must be called at the top level in the body of a function
/// component or custom hook, and may not be called within function expressions.
fn check_hooks_in_nested_functions(
    hir: &HIR,
    errors: &mut ErrorCollector,
    hook_aliases: &FxHashSet<String>,
) {
    let is_hook = |name: &str| -> bool { is_hook_name(name) || hook_aliases.contains(name) };

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
                check_nested_hir_for_hook_calls(body, errors, &is_hook);
            }
        }
    }
}

/// Recursively scan a nested HIR for hook calls and emit errors.
fn check_nested_hir_for_hook_calls(
    body: &HIR,
    errors: &mut ErrorCollector,
    is_hook: &dyn Fn(&str) -> bool,
) {
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
                    && is_hook(&name)
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
                && is_hook(property)
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
                check_nested_hir_for_hook_calls(nested_body, errors, is_hook);
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
