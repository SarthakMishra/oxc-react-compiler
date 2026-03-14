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
pub fn validate_use_memo(hir: &HIR, errors: &mut ErrorCollector) {
    // Build id-to-name map for resolving SSA temporaries
    let id_to_name = build_name_map(hir);

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Extract hook name and args from either CallExpression or MethodCall.
            // CallExpression handles `useMemo(...)`, MethodCall handles `React.useMemo(...)`.
            let (hook_name, args) = match &instr.value {
                InstructionValue::CallExpression { callee, args } => {
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

                if hook_name == "useMemo" {
                    // Only useMemo callbacks must not accept parameters (called with no args).
                    // useCallback callbacks CAN and DO accept parameters — that's their purpose.
                    check_memo_callback_params(hir, callback_id, hook_name, instr.loc, errors);
                    // Check if the callback is async (useMemo must be sync)
                    check_memo_callback_async(hir, callback_id, instr.loc, errors);
                    // Check if the callback returns void (useMemo must return a value)
                    check_memo_callback_void(hir, callback_id, instr.loc, errors);
                    // Check if the callback calls setState
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
                    if let crate::hir::types::Terminal::Return { value } = &b.terminal {
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
fn check_memo_callback_set_state(
    hir: &HIR,
    callback_id: IdentifierId,
    _call_loc: crate::hir::types::SourceLocation,
    errors: &mut ErrorCollector,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                // Collect setState identifiers in the callback body
                let set_state_ids = collect_set_state_ids(&lowered_func.body);

                // Check all call expressions in the callback body
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        if let InstructionValue::CallExpression { callee, .. } = &inner_instr.value
                            && set_state_ids.contains(&callee.identifier.id)
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
