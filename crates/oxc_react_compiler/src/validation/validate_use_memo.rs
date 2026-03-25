use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, IdentifierId, InstructionValue, Type};
use rustc_hash::{FxHashMap, FxHashSet};

/// Validate correct usage of `useMemo` and `useCallback`.
///
/// Checks:
/// 1. `useMemo` / `useCallback` must be called with exactly 2 arguments
///    (a callback and a dependency array).
/// 2. The callback argument should be a function expression (not an arbitrary value).
/// 3. The callback passed to `useMemo` should not be async.
pub fn validate_use_memo(hir: &HIR, errors: &mut ErrorCollector, validate_preserve_memo: bool) {
    // Build id-to-name map for resolving SSA temporaries
    let id_to_name = build_name_map(hir);

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Extract hook name and args from either CallExpression or MethodCall.
            // CallExpression handles `useMemo(...)`, MethodCall handles `React.useMemo(...)`.
            let (hook_name, args) = match &instr.value {
                InstructionValue::CallExpression { callee, args, .. } => {
                    let name = callee
                        .identifier
                        .name
                        .as_deref()
                        .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));
                    match name {
                        Some(n) if n == "useMemo" || n == "useCallback" => (n, args.as_slice()),
                        _ => continue,
                    }
                }
                InstructionValue::MethodCall { property, args, .. } => {
                    if property == "useMemo" || property == "useCallback" {
                        (property.as_str(), args.as_slice())
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };

            // DIVERGENCE: Upstream does NOT validate argument count for useMemo/useCallback.
            // `useMemo(fn)` without a deps array is valid React (recomputes every render).
            // We only validate the deps array if one IS provided.

            // Check that the deps argument is an array literal, not a computed value
            if args.len() >= 2 {
                check_deps_is_array_literal(hir, &args[1], hook_name, instr.loc, errors);
            }

            if !args.is_empty() {
                let callback_id = args[0].identifier.id;

                // When preserve-memo validation is active, check that the first
                // argument is an inline function expression. Named function
                // references can't be analyzed for reactive dependencies.
                if validate_preserve_memo {
                    check_callback_is_inline_function(
                        hir,
                        callback_id,
                        hook_name,
                        instr.loc,
                        errors,
                    );
                }

                if hook_name == "useMemo" {
                    // Only useMemo callbacks must not accept parameters (called with no args).
                    // useCallback callbacks CAN and DO accept parameters — that's their purpose.
                    check_memo_callback_params(hir, callback_id, hook_name, instr.loc, errors);
                    // Check if the callback is async (useMemo must be sync)
                    check_memo_callback_async(hir, callback_id, instr.loc, errors);
                    // Check if the callback returns void (useMemo must return a value)
                    check_memo_callback_void(hir, callback_id, instr.loc, errors);
                    // Check if the callback calls setState (directly or transitively)
                    check_memo_callback_set_state(hir, callback_id, instr.loc, errors);
                }
            }
        }
    }
}

/// Build a map from identifier ID → name for SSA resolution.
fn build_name_map(hir: &HIR) -> FxHashMap<IdentifierId, String> {
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

    id_to_name
}

/// Check if the function expression producing the given identifier returns void.
/// useMemo callbacks must return a value — returning void/undefined is likely a bug.
fn check_memo_callback_void(
    hir: &HIR,
    callback_id: crate::hir::types::IdentifierId,
    call_loc: crate::hir::types::SourceLocation,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                // Check if the function body has any Return terminal with a non-undefined value.
                // If all returns are void (i.e., return undefined), this is likely a mistake.
                let has_return_value = lowered_func.body.blocks.iter().any(|(_, b)| {
                    if let crate::hir::types::Terminal::Return { value, .. } = &b.terminal {
                        // Check if the return value is a named variable or non-trivial
                        value.identifier.name.is_some()
                            || value.identifier.type_
                                != crate::hir::types::Type::Primitive(
                                    crate::hir::types::PrimitiveType::Undefined,
                                )
                    } else {
                        false
                    }
                });

                if !has_return_value {
                    errors.push(CompilerError::invalid_react_with_kind(
                        call_loc,
                        "useMemo callback does not return a value. \
                         useMemo is for memoizing computed values — use useEffect for side effects."
                            .to_string(),
                        DiagnosticKind::VoidUseMemo,
                    ));
                }
            }
        }
    }
}

/// Check if the useMemo callback calls setState, which could cause infinite loops.
/// Detects both direct setState calls within the callback body and indirect calls
/// through functions that transitively call setState (e.g., calling a useCallback
/// function that itself calls setState).
fn check_memo_callback_set_state(
    hir: &HIR,
    callback_id: IdentifierId,
    _call_loc: crate::hir::types::SourceLocation,
    errors: &mut ErrorCollector,
) {
    // Collect names of parent-scope functions that transitively call setState.
    // This handles patterns like:
    //   const fn = useCallback(() => { setState(init); });
    //   useMemo(() => { fn(); }, [...]);
    let fns_calling_set_state = collect_fns_calling_set_state(hir);

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                // Collect setState identifiers in the callback body
                let set_state_ids = collect_set_state_ids(&lowered_func.body);

                // Build a set of callee IDs that resolve to fns_calling_set_state names
                // via LoadContext instructions in the callback body
                let mut indirect_set_state_ids: FxHashSet<IdentifierId> = FxHashSet::default();
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        if let InstructionValue::LoadContext { place } = &inner_instr.value
                            && let Some(name) = &place.identifier.name
                            && fns_calling_set_state.contains(name.as_str())
                        {
                            indirect_set_state_ids.insert(inner_instr.lvalue.identifier.id);
                        }
                    }
                }

                // Check all call expressions in the callback body
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        if let InstructionValue::CallExpression { callee, .. } = &inner_instr.value
                            && (set_state_ids.contains(&callee.identifier.id)
                                || indirect_set_state_ids.contains(&callee.identifier.id))
                        {
                            errors.push(CompilerError::invalid_react_with_kind(
                                inner_instr.loc,
                                "Calling setState from useMemo may trigger an infinite loop. \
                                     Each time the memo callback is evaluated it will change the \
                                     state, which will cause React to re-render and re-evaluate \
                                     the memo callback."
                                    .to_string(),
                                DiagnosticKind::UseMemoValidation,
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Collect names of functions defined at the parent HIR level that
/// transitively call setState in their body.
fn collect_fns_calling_set_state(hir: &HIR) -> FxHashSet<String> {
    let mut result = FxHashSet::default();

    // Build a map from FE lvalue id -> variable name (via StoreLocal/StoreContext)
    let mut fe_id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    if let Some(name) = &lvalue.identifier.name {
                        fe_id_to_name.insert(value.identifier.id, name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                // Check if this FE's body calls setState
                if fe_body_calls_set_state(&lowered_func.body) {
                    // Find the name of this FE
                    if let Some(name) = &instr.lvalue.identifier.name {
                        result.insert(name.clone());
                    }
                    if let Some(name) = fe_id_to_name.get(&instr.lvalue.identifier.id) {
                        result.insert(name.clone());
                    }
                }
            }
        }
    }

    result
}

/// Check if a FunctionExpression body contains any setState calls.
fn fe_body_calls_set_state(hir: &HIR) -> bool {
    let set_state_ids = collect_set_state_ids(hir);
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value
                && set_state_ids.contains(&callee.identifier.id)
            {
                return true;
            }
        }
    }
    false
}

/// Collect all setState identifier IDs in an HIR body.
fn collect_set_state_ids(hir: &HIR) -> FxHashSet<IdentifierId> {
    let mut set_state_ids: FxHashSet<IdentifierId> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.type_ == Type::SetState {
                set_state_ids.insert(instr.lvalue.identifier.id);
            }
            if let Some(name) = &instr.lvalue.identifier.name
                && is_set_state_name(name)
            {
                set_state_ids.insert(instr.lvalue.identifier.id);
            }
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::SetState
                        || set_state_ids.contains(&place.identifier.id)
                    {
                        set_state_ids.insert(instr.lvalue.identifier.id);
                    }
                    if let Some(name) = &place.identifier.name
                        && is_set_state_name(name)
                    {
                        set_state_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                _ => {}
            }
        }
    }

    set_state_ids
}

/// Check if a name looks like a setState function (setX where X is uppercase).
fn is_set_state_name(name: &str) -> bool {
    name.starts_with("set") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase()
}

/// Check that the dependency list argument is an array literal, not a computed value.
///
/// Upstream rejects patterns like `useMemo(fn, hasDeps ? null : [text])` because
/// the compiler needs to statically analyze the dependency list.
fn check_deps_is_array_literal(
    hir: &HIR,
    deps_place: &crate::hir::types::Place,
    hook_name: &str,
    call_loc: crate::hir::types::SourceLocation,
    errors: &mut ErrorCollector,
) {
    // Resolve the deps argument through the HIR to find its defining instruction.
    // If the defining instruction is an ArrayExpression or Primitive, it's valid.
    // If we find a different instruction type, or no instruction at all (e.g., phi
    // result from a conditional), it means the deps are computed — reject.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != deps_place.identifier.id {
                continue;
            }
            // Valid: array literal or undefined (no deps)
            if matches!(instr.value, InstructionValue::ArrayExpression { .. })
                || matches!(
                    &instr.value,
                    InstructionValue::Primitive { value }
                        if matches!(
                            value,
                            crate::hir::types::Primitive::Undefined
                        )
                )
            {
                return;
            }
            // Invalid: any other instruction type
            errors.push(CompilerError::invalid_react_with_kind(
                call_loc,
                format!(
                    "Expected the dependency list for {hook_name} to be an array literal. \
                     The React Compiler does not support computed or dynamic dependency arrays."
                ),
                DiagnosticKind::UseMemoValidation,
            ));
            return;
        }
    }
    // No defining instruction found (phi result, etc.) — deps are computed
    errors.push(CompilerError::invalid_react_with_kind(
        call_loc,
        format!(
            "Expected the dependency list for {hook_name} to be an array literal. \
             The React Compiler does not support computed or dynamic dependency arrays."
        ),
        DiagnosticKind::UseMemoValidation,
    ));
}

/// Check if the callback function has parameters. useMemo/useCallback callbacks are
/// called by React with no arguments — accepting parameters is always a mistake.
fn check_memo_callback_params(
    hir: &HIR,
    callback_id: crate::hir::types::IdentifierId,
    hook_name: &str,
    call_loc: crate::hir::types::SourceLocation,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && !lowered_func.params.is_empty()
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    call_loc,
                    format!(
                        "{hook_name}() callbacks may not accept parameters. \
                         {hook_name}() callbacks are called by React to cache calculations \
                         across renders, and should not have side effects or accept inputs."
                    ),
                    DiagnosticKind::UseMemoValidation,
                ));
            }
        }
    }
}

/// Check that the first argument to useMemo/useCallback is an inline function expression.
/// Named function references (e.g., `useMemo(someHelper, [])`) cannot be analyzed
/// for reactive dependencies, so preserve-memo validation rejects them.
///
/// Checks if the identifier resolves to a FunctionExpression instruction.
/// If it resolves to anything else (LoadLocal, LoadGlobal, LoadContext), it's
/// not an inline function and should be rejected.
fn check_callback_is_inline_function(
    hir: &HIR,
    callback_id: crate::hir::types::IdentifierId,
    hook_name: &str,
    call_loc: crate::hir::types::SourceLocation,
    errors: &mut ErrorCollector,
) {
    // Check if callback_id is directly produced by a FunctionExpression instruction.
    // This handles the common case: `useMemo(() => ..., [...])`.
    let is_inline_fe = hir.blocks.iter().any(|(_, block)| {
        block.instructions.iter().any(|instr| {
            instr.lvalue.identifier.id == callback_id
                && matches!(instr.value, InstructionValue::FunctionExpression { .. })
        })
    });

    if is_inline_fe {
        return;
    }

    // Not a direct FE -- could be a LoadLocal that transitively resolves to an FE.
    // Check if it's a LoadLocal pointing to a StoreLocal of an FE.
    let resolved_to_fe = hir.blocks.iter().any(|(_, block)| {
        block.instructions.iter().any(|instr| {
            if instr.lvalue.identifier.id != callback_id {
                return false;
            }
            // If it's a LoadLocal, check if the source variable was defined by an FE
            if let InstructionValue::LoadLocal { place } = &instr.value {
                // Walk the HIR to find if place.identifier.id comes from a FE
                return hir.blocks.iter().any(|(_, b)| {
                    b.instructions.iter().any(|i| {
                        // StoreLocal that stores into this variable from an FE temp
                        if let InstructionValue::StoreLocal { lvalue, value, .. } = &i.value
                            && lvalue.identifier.id == place.identifier.id
                        {
                            // Check if the value comes from a FunctionExpression
                            hir.blocks.iter().any(|(_, b2)| {
                                b2.instructions.iter().any(|i2| {
                                    i2.lvalue.identifier.id == value.identifier.id
                                        && matches!(
                                            i2.value,
                                            InstructionValue::FunctionExpression { .. }
                                        )
                                })
                            })
                        } else {
                            // Direct FE assignment to this identifier
                            i.lvalue.identifier.id == place.identifier.id
                                && matches!(i.value, InstructionValue::FunctionExpression { .. })
                        }
                    })
                });
            }
            false
        })
    });

    if resolved_to_fe {
        return;
    }

    // The first argument is not an inline function expression
    errors.push(CompilerError::invalid_react_with_kind(
        call_loc,
        format!(
            "Expected the first argument to be an inline function expression. \
             {hook_name}() requires an inline function so the compiler can \
             analyze its reactive dependencies."
        ),
        DiagnosticKind::UseMemoValidation,
    ));
}

/// Check if the function expression producing the given identifier is async.
fn check_memo_callback_async(
    hir: &HIR,
    callback_id: crate::hir::types::IdentifierId,
    call_loc: crate::hir::types::SourceLocation,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && lowered_func.is_async
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    call_loc,
                    "useMemo callback must not be async. \
                         The callback should return a value synchronously."
                        .to_string(),
                    DiagnosticKind::UseMemoValidation,
                ));
            }
        }
    }
}
