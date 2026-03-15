use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, IdentifierId, InstructionValue, Type};
use rustc_hash::{FxHashMap, FxHashSet};

/// Validate that setState is not called unconditionally during render.
///
/// Calling setState during render causes infinite re-render loops.
/// Uses type-based detection (Type::SetState from useState/useReducer
/// destructuring) with naming heuristic fallback. Resolves identities
/// through SSA temporaries via LoadLocal/LoadGlobal/LoadContext instructions.
///
/// Also detects transitive setState calls through helper functions:
/// if `foo` calls `setState`, and the component calls `foo()` during render,
/// that is also an error. Handles arbitrarily deep call chains.
pub fn validate_no_set_state_in_render(hir: &HIR, errors: &mut ErrorCollector) {
    // Collect all identifier IDs that are setState-like (by type or name)
    let mut set_state_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    // Also collect setState variable names for cross-scope tracking
    let mut set_state_names: FxHashSet<String> = FxHashSet::default();

    // Collect names loaded from LoadGlobal — these are imports/globals, not
    // React setState functions. Used to exclude false positives from the name
    // heuristic (e.g., `setPropertyByKey` from shared-runtime).
    let mut global_names: FxHashSet<String> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::LoadGlobal { binding } = &instr.value {
                global_names.insert(binding.name.clone());
            }
        }
    }

    // Pass 1: Identify setState identifiers from their definition sites
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Check type on the lvalue
            if instr.lvalue.identifier.type_ == Type::SetState {
                set_state_ids.insert(instr.lvalue.identifier.id);
                if let Some(name) = &instr.lvalue.identifier.name {
                    set_state_names.insert(name.clone());
                }
            }

            // Name heuristic: setX where X is uppercase, but only for local
            // variables (not globals/imports which may be utility functions
            // like setPropertyByKey).
            if let Some(name) = &instr.lvalue.identifier.name
                && is_set_state_name(name)
                && !global_names.contains(name)
            {
                set_state_ids.insert(instr.lvalue.identifier.id);
                set_state_names.insert(name.clone());
            }

            // Track through LoadLocal/LoadContext: if loading a setState variable,
            // the result is also setState
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::SetState
                        || set_state_ids.contains(&place.identifier.id)
                    {
                        set_state_ids.insert(instr.lvalue.identifier.id);
                    }
                    // Name-based tracking for cross-scope resolution
                    if let Some(name) = &place.identifier.name
                        && (set_state_names.contains(name)
                            || (is_set_state_name(name) && !global_names.contains(name)))
                    {
                        set_state_ids.insert(instr.lvalue.identifier.id);
                        set_state_names.insert(name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    // Pass 1.5: Build a map of function lvalue IDs to whether they call setState
    // (directly or transitively). Also build name-based lookup for call chain resolution.
    let mut functions_calling_set_state: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut func_name_calls_set_state: FxHashSet<String> = FxHashSet::default();
    // Map: function name → list of function names it calls (for transitive resolution)
    let mut func_calls_map: FxHashMap<String, Vec<String>> = FxHashMap::default();
    // Temp map for FunctionExpression lvalues without names (keyed by temp ID)
    let mut temp_func_calls: FxHashMap<IdentifierId, Vec<String>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func } => {
                    let (calls_set_state, called_funcs) =
                        check_nested_set_state_call(&lowered_func.body, &set_state_names);
                    if calls_set_state {
                        functions_calling_set_state.insert(instr.lvalue.identifier.id);
                        if let Some(name) = &instr.lvalue.identifier.name {
                            func_name_calls_set_state.insert(name.clone());
                        }
                    }
                    if let Some(name) = &instr.lvalue.identifier.name {
                        func_calls_map.insert(name.clone(), called_funcs);
                    } else {
                        // FunctionExpression lvalue is a temp — store by ID for
                        // later re-keying when StoreLocal gives it a name
                        temp_func_calls.insert(instr.lvalue.identifier.id, called_funcs);
                    }
                }
                // Propagate function-calling-setState through StoreLocal/DeclareLocal:
                // FunctionExpression lvalue is a temp; the named variable is the StoreLocal target.
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    if functions_calling_set_state.contains(&value.identifier.id) {
                        functions_calling_set_state.insert(instr.lvalue.identifier.id);
                        if let Some(name) = &lvalue.identifier.name {
                            func_name_calls_set_state.insert(name.clone());
                        }
                    }
                    // Also propagate func_calls_map: if the value was a FunctionExpression
                    // temp that has an entry keyed by its temp ID in a parallel map,
                    // re-key it by the named variable. We use a separate temp_to_calls map.
                    if let Some(name) = &lvalue.identifier.name {
                        // Check if value.identifier.id had func_calls tracked via temp_func_calls
                        if let Some(calls) = temp_func_calls.remove(&value.identifier.id) {
                            func_calls_map.insert(name.clone(), calls);
                        }
                    }
                }
                InstructionValue::DeclareLocal { lvalue, .. } => {
                    // DeclareLocal doesn't carry a value, but the lvalue might have the
                    // same ID as a function expression lvalue if SSA hasn't renamed it.
                    // Check by name if the lvalue was previously identified.
                    if let Some(name) = &lvalue.identifier.name
                        && func_name_calls_set_state.contains(name)
                    {
                        functions_calling_set_state.insert(instr.lvalue.identifier.id);
                    }
                }
                _ => {}
            }
        }
    }

    // Fixpoint: resolve transitive calls (foo calls bar which calls setState).
    // DIVERGENCE: Upstream uses a different analysis approach that doesn't need
    // a fixpoint loop. We cap at 10 iterations to prevent pathological cases;
    // real code rarely has call chains deeper than 3-4 levels.
    let mut changed = true;
    let mut iterations = 0;
    while changed && iterations < 10 {
        changed = false;
        iterations += 1;
        let current_names: Vec<String> = func_calls_map.keys().cloned().collect();
        for func_name in &current_names {
            if func_name_calls_set_state.contains(func_name) {
                continue;
            }
            if let Some(called) = func_calls_map.get(func_name)
                && called.iter().any(|c| func_name_calls_set_state.contains(c))
            {
                func_name_calls_set_state.insert(func_name.clone());
                changed = true;
            }
        }
    }

    // Update functions_calling_set_state with transitive results
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::FunctionExpression { .. } = &instr.value
                && let Some(name) = &instr.lvalue.identifier.name
                && func_name_calls_set_state.contains(name)
            {
                functions_calling_set_state.insert(instr.lvalue.identifier.id);
            }
        }
    }

    // Build ID-based lookup for functions via LoadLocal
    let mut func_caller_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } =
                &instr.value
            {
                if functions_calling_set_state.contains(&place.identifier.id) {
                    func_caller_ids.insert(instr.lvalue.identifier.id);
                }
                if let Some(name) = &place.identifier.name
                    && func_name_calls_set_state.contains(name)
                {
                    func_caller_ids.insert(instr.lvalue.identifier.id);
                }
            }
        }
    }

    // Pass 2: Check for unconditional setState calls (direct or transitive)
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                let is_direct_set_state = set_state_ids.contains(&callee.identifier.id);
                let is_transitive_set_state = func_caller_ids.contains(&callee.identifier.id)
                    || functions_calling_set_state.contains(&callee.identifier.id);

                if is_direct_set_state || is_transitive_set_state {
                    errors.push(CompilerError::invalid_react_with_kind(
                        instr.loc,
                        "Cannot call setState during render. \
                         Calling setState during render may trigger an infinite loop. \
                         * To reset state based on a condition, check if state is already \
                         set and early return.\n\
                         * To derive data from props/state, calculate it during render."
                            .to_string(),
                        DiagnosticKind::SetStateInRender,
                    ));
                    return;
                }
            }
        }
    }
}

/// Check if a nested function body calls setState (directly or via local helpers).
/// Returns (calls_set_state, list_of_called_function_names).
fn check_nested_set_state_call(
    hir: &HIR,
    outer_set_state_names: &FxHashSet<String>,
) -> (bool, Vec<String>) {
    let mut local_set_state_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut called_funcs: Vec<String> = Vec::new();

    // Build local ID map
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.type_ == Type::SetState {
                local_set_state_ids.insert(instr.lvalue.identifier.id);
            }
            if let Some(name) = &instr.lvalue.identifier.name
                && is_set_state_name(name)
            {
                local_set_state_ids.insert(instr.lvalue.identifier.id);
            }
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::SetState
                        || local_set_state_ids.contains(&place.identifier.id)
                    {
                        local_set_state_ids.insert(instr.lvalue.identifier.id);
                    }
                    if let Some(name) = &place.identifier.name
                        && (outer_set_state_names.contains(name) || is_set_state_name(name))
                    {
                        local_set_state_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                _ => {}
            }
        }
    }

    // Check for setState calls and collect called function names
    let mut calls_set_state = false;
    // Build name-to-id mapping for callee resolution
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                if local_set_state_ids.contains(&callee.identifier.id) {
                    calls_set_state = true;
                }
                // Track which functions this function calls
                if let Some(name) = id_to_name.get(&callee.identifier.id) {
                    called_funcs.push(name.clone());
                }
            }
        }
    }

    (calls_set_state, called_funcs)
}

/// Check if a name looks like a setState function (setX where X is uppercase).
fn is_set_state_name(name: &str) -> bool {
    name.starts_with("set") && name.len() > 3 && name.as_bytes()[3].is_ascii_uppercase()
}
