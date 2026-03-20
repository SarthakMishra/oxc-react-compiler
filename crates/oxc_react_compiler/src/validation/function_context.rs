use crate::hir::globals::is_hook_name;
use crate::hir::types::{HIR, IdentifierId, InstructionValue, Terminal};
use rustc_hash::{FxHashMap, FxHashSet};

/// Collect the set of FunctionExpression lvalue IDs that are directly called
/// at render time (called as `fn()` at the top level of the component body).
///
/// Uses ID-based forward alias chain tracking: follows the FE lvalue through
/// StoreLocal and LoadLocal to find all IDs it flows into, then checks if any
/// of those IDs appear as a CallExpression callee.
pub fn collect_directly_called_fe_ids(hir: &HIR) -> FxHashSet<IdentifierId> {
    // Step 1: Find all FE lvalue IDs
    let mut func_expr_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if matches!(
                instr.value,
                InstructionValue::FunctionExpression { .. } | InstructionValue::ObjectMethod { .. }
            ) {
                func_expr_ids.insert(instr.lvalue.identifier.id);
            }
        }
    }

    // Step 2: Build forward alias map: source_id → IDs it flows into.
    let mut forward_aliases: FxHashMap<IdentifierId, Vec<IdentifierId>> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::StoreLocal { value, .. }
                | InstructionValue::StoreContext { value, .. } => {
                    forward_aliases
                        .entry(value.identifier.id)
                        .or_default()
                        .push(instr.lvalue.identifier.id);
                }
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    forward_aliases
                        .entry(place.identifier.id)
                        .or_default()
                        .push(instr.lvalue.identifier.id);
                }
                _ => {}
            }
        }
    }

    // Step 3: For each FE, compute the set of all IDs reachable via alias chains.
    let mut fe_reachable: FxHashMap<IdentifierId, FxHashSet<IdentifierId>> = FxHashMap::default();
    for &fe_id in &func_expr_ids {
        let mut reachable = FxHashSet::default();
        let mut worklist = vec![fe_id];
        while let Some(current) = worklist.pop() {
            if !reachable.insert(current) {
                continue;
            }
            if let Some(targets) = forward_aliases.get(&current) {
                worklist.extend(targets);
            }
        }
        fe_reachable.insert(fe_id, reachable);
    }

    // Step 4: Find all CallExpression callee IDs
    let mut callee_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                callee_ids.insert(callee.identifier.id);
            }
        }
    }

    // Step 5: A FE is "directly called" if any ID in its reachable set
    // appears as a CallExpression callee
    let mut result = FxHashSet::default();
    for (fe_id, reachable) in &fe_reachable {
        if reachable.iter().any(|id| callee_ids.contains(id)) {
            result.insert(*fe_id);
        }
    }
    result
}

/// Check if a FE's variable name is shadowed by a local variable inside
/// its body. If so, it should NOT be treated as render-only because the
/// inner reassignment of the shadowed name needs to be flagged.
pub fn has_self_shadowing(hir: &HIR) -> FxHashSet<IdentifierId> {
    let mut result = FxHashSet::default();

    // Build FE ID → variable name mapping
    let mut func_expr_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut fe_to_var: FxHashMap<IdentifierId, String> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if matches!(
                instr.value,
                InstructionValue::FunctionExpression { .. } | InstructionValue::ObjectMethod { .. }
            ) {
                func_expr_ids.insert(instr.lvalue.identifier.id);
            }
        }
    }
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreLocal { lvalue, value, .. } = &instr.value
                && func_expr_ids.contains(&value.identifier.id)
                && let Some(name) = &lvalue.identifier.name
            {
                fe_to_var.insert(value.identifier.id, name.clone());
            }
        }
    }

    // Check each FE body for shadowed variable assignments
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && let Some(var_name) = fe_to_var.get(&instr.lvalue.identifier.id)
            {
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        if let InstructionValue::StoreLocal { lvalue, .. }
                        | InstructionValue::DeclareLocal { lvalue, .. } = &inner_instr.value
                            && let Some(inner_name) = &lvalue.identifier.name
                            && inner_name == var_name
                        {
                            result.insert(instr.lvalue.identifier.id);
                        }
                    }
                }
            }
        }
    }

    result
}

/// Collect the set of FunctionExpression lvalue IDs that execute in post-render
/// contexts (effect hooks, event handlers, hook arguments, returned values).
///
/// After initial seeding and alias propagation, performs transitive expansion:
/// if a post-render FE's body references a named variable that holds another FE,
/// that FE is also marked as post-render.
pub fn collect_post_render_fn_ids(hir: &HIR) -> FxHashSet<IdentifierId> {
    // Phase 1: Build auxiliary maps.
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();
    let mut func_expr_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut fe_id_to_var_name: FxHashMap<IdentifierId, String> = FxHashMap::default();
    let mut var_name_to_fe_id: FxHashMap<String, IdentifierId> = FxHashMap::default();

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
            if matches!(
                instr.value,
                InstructionValue::FunctionExpression { .. } | InstructionValue::ObjectMethod { .. }
            ) {
                func_expr_ids.insert(instr.lvalue.identifier.id);
            }
        }
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::StoreLocal { lvalue, value, .. } = &instr.value
                && func_expr_ids.contains(&value.identifier.id)
                && let Some(name) = &lvalue.identifier.name
            {
                fe_id_to_var_name.insert(value.identifier.id, name.clone());
                var_name_to_fe_id.insert(name.clone(), value.identifier.id);
            }
        }
    }

    // Phase 2: Collect initial post-render IDs.
    let mut ids: FxHashSet<IdentifierId> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::CallExpression { callee, args, .. } => {
                    let callee_name = callee
                        .identifier
                        .name
                        .as_deref()
                        .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));
                    if let Some(name) = callee_name
                        && is_hook_name(name)
                    {
                        for arg in args {
                            ids.insert(arg.identifier.id);
                        }
                    }
                }
                InstructionValue::MethodCall { property, args, .. } => {
                    if is_hook_name(property) {
                        for arg in args {
                            ids.insert(arg.identifier.id);
                        }
                    }
                }
                InstructionValue::JsxExpression { props, .. } => {
                    for attr in props {
                        let is_callback_prop = match &attr.name {
                            crate::hir::types::JsxAttributeName::Named(name) => {
                                name == "ref"
                                    || (name.starts_with("on")
                                        && name.len() > 2
                                        && name.as_bytes()[2].is_ascii_uppercase())
                            }
                            _ => false,
                        };
                        if is_callback_prop {
                            ids.insert(attr.value.identifier.id);
                        }
                    }
                }
                _ => {}
            }
        }
        if let Terminal::Return { value, .. } = &block.terminal {
            ids.insert(value.identifier.id);
        }
    }

    // Phase 3: Alias propagation.
    let mut id_aliases: FxHashMap<IdentifierId, IdentifierId> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    id_aliases.insert(instr.lvalue.identifier.id, place.identifier.id);
                }
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    id_aliases.insert(lvalue.identifier.id, value.identifier.id);
                }
                _ => {}
            }
        }
    }

    let snapshot: Vec<IdentifierId> = ids.iter().copied().collect();
    for id in snapshot {
        let mut current = id;
        for _ in 0..10 {
            if let Some(&alias) = id_aliases.get(&current) {
                ids.insert(alias);
                current = alias;
            } else {
                break;
            }
        }
    }

    // Phase 4: Transitive fixpoint expansion.
    let fn_var_names: FxHashSet<&str> = fe_id_to_var_name.values().map(String::as_str).collect();
    if !fn_var_names.is_empty() {
        let mut fe_bodies: FxHashMap<IdentifierId, &HIR> = FxHashMap::default();
        for (_, block) in &hir.blocks {
            for instr in &block.instructions {
                if let InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } = &instr.value
                {
                    fe_bodies.insert(instr.lvalue.identifier.id, &lowered_func.body);
                }
            }
        }

        let mut worklist: Vec<IdentifierId> =
            ids.iter().copied().filter(|id| func_expr_ids.contains(id)).collect();
        let mut visited: FxHashSet<IdentifierId> = worklist.iter().copied().collect();

        while let Some(fe_id) = worklist.pop() {
            let Some(body) = fe_bodies.get(&fe_id) else {
                continue;
            };
            for (_, inner_block) in &body.blocks {
                for inner_instr in &inner_block.instructions {
                    if let InstructionValue::LoadLocal { place }
                    | InstructionValue::LoadContext { place } = &inner_instr.value
                        && let Some(name) = &place.identifier.name
                        && fn_var_names.contains(name.as_str())
                        && let Some(&other_fe_id) = var_name_to_fe_id.get(name.as_str())
                        && visited.insert(other_fe_id)
                    {
                        ids.insert(other_fe_id);
                        worklist.push(other_fe_id);
                    }
                }
            }
        }
    }

    ids
}
