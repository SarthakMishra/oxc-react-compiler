use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, IdentifierId, InstructionValue, Type};
use rustc_hash::{FxHashMap, FxHashSet};

/// Hook names whose first callback argument executes after render.
const EFFECT_HOOKS: &[&str] = &["useEffect", "useLayoutEffect", "useInsertionEffect"];

/// Validate that ref values are not accessed during render.
///
/// Accessing `.current` on a ref during render can cause tearing
/// because refs are mutable and not tracked by React.
///
/// Ref access inside effect callbacks, event handlers, and useCallback
/// bodies is fine — those execute after render, not during it.
///
/// Uses both type-based detection (Type::Ref from useRef() calls) and
/// naming heuristic fallback. Resolves identities through SSA temporaries.
pub fn validate_no_ref_access_in_render(hir: &HIR, errors: &mut ErrorCollector) {
    // Collect IDs of function expressions used in non-render contexts
    // (effects, event handlers, useCallback). Ref access inside these is fine.
    let non_render_ids = collect_non_render_callback_ids(hir);

    // Collect all identifier IDs that are ref-like (by type or name)
    let mut ref_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut ref_names: FxHashSet<String> = FxHashSet::default();

    // Pass 1: Identify ref identifiers from their definition sites
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.type_ == Type::Ref {
                ref_ids.insert(instr.lvalue.identifier.id);
                if let Some(name) = &instr.lvalue.identifier.name {
                    ref_names.insert(name.clone());
                }
            }

            if let Some(name) = &instr.lvalue.identifier.name
                && is_ref_name(name)
            {
                ref_ids.insert(instr.lvalue.identifier.id);
                ref_names.insert(name.clone());
            }

            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::Ref || ref_ids.contains(&place.identifier.id)
                    {
                        ref_ids.insert(instr.lvalue.identifier.id);
                    }
                    if let Some(name) = &place.identifier.name
                        && (is_ref_name(name) || ref_names.contains(name))
                    {
                        ref_ids.insert(instr.lvalue.identifier.id);
                        ref_names.insert(name.clone());
                    }
                }
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    if ref_ids.contains(&value.identifier.id) {
                        ref_ids.insert(instr.lvalue.identifier.id);
                        if let Some(name) = &lvalue.identifier.name {
                            ref_names.insert(name.clone());
                        }
                    }
                }
                InstructionValue::PropertyLoad { property, .. } => {
                    if is_ref_name(property) {
                        ref_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                _ => {}
            }
        }
    }

    // Pass 2: Check for ref.current access (read or write) at top level
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let is_ref_current = match &instr.value {
                InstructionValue::PropertyLoad { object, property, .. } => {
                    property == "current" && ref_ids.contains(&object.identifier.id)
                }
                InstructionValue::PropertyStore { object, property, .. } => {
                    property == "current" && ref_ids.contains(&object.identifier.id)
                }
                _ => false,
            };
            if is_ref_current {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    "Cannot access refs during render. \
                     React refs are values that are not needed for rendering. \
                     Refs should only be accessed in effects or event handlers."
                        .to_string(),
                    DiagnosticKind::RefAccessInRender,
                ));
                return;
            }

            // Scan nested function bodies for ref.current access,
            // but SKIP functions that are effect/event handler callbacks
            // (ref access is fine in those contexts).
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func } => {
                    if !non_render_ids.contains(&instr.lvalue.identifier.id)
                        && check_nested_ref_access(&lowered_func.body, &ref_names, &non_render_ids)
                    {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            "Cannot access refs during render. \
                             React refs are values that are not needed for rendering. \
                             Refs should only be accessed in effects or event handlers."
                                .to_string(),
                            DiagnosticKind::RefAccessInRender,
                        ));
                        return;
                    }
                }
                _ => {}
            }
        }
    }
}

/// Collect IDs of function expressions used in non-render contexts:
/// - First argument to useEffect / useLayoutEffect / useInsertionEffect
/// - Value of a JSX event handler prop (onXxx)
/// - First argument to useCallback
/// - Ref callback props (ref={callback})
///
/// These functions execute AFTER render, so ref access inside them is fine.
fn collect_non_render_callback_ids(hir: &HIR) -> FxHashSet<IdentifierId> {
    let mut ids: FxHashSet<IdentifierId> = FxHashSet::default();

    // Build id-to-name map to resolve callee identifiers
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

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::CallExpression { callee, args, .. } => {
                    let callee_name = callee
                        .identifier
                        .name
                        .as_deref()
                        .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));
                    if let Some(name) = callee_name {
                        if EFFECT_HOOKS.contains(&name) && !args.is_empty() {
                            ids.insert(args[0].identifier.id);
                        }
                        if name == "useCallback" && !args.is_empty() {
                            ids.insert(args[0].identifier.id);
                        }
                        // useImperativeHandle(ref, createFn) — createFn runs in effect phase
                        if name == "useImperativeHandle" && args.len() >= 2 {
                            ids.insert(args[1].identifier.id);
                        }
                    }
                }
                InstructionValue::MethodCall { property, args, .. } => {
                    if EFFECT_HOOKS.contains(&property.as_str()) && !args.is_empty() {
                        ids.insert(args[0].identifier.id);
                    }
                    if property == "useCallback" && !args.is_empty() {
                        ids.insert(args[0].identifier.id);
                    }
                    if property == "useImperativeHandle" && args.len() >= 2 {
                        ids.insert(args[1].identifier.id);
                    }
                }
                // JSX event handler props AND ref callbacks
                InstructionValue::JsxExpression { props, .. } => {
                    for attr in props {
                        let is_callback_prop = match &attr.name {
                            crate::hir::types::JsxAttributeName::Named(name) => {
                                // Event handlers (onXxx) and ref callbacks
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
    }

    // Propagate through LoadLocal/StoreLocal alias chains
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

    let copy: Vec<IdentifierId> = ids.iter().copied().collect();
    for id in copy {
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

    ids
}

/// Check if a nested function body accesses ref.current for any ref name
/// from the outer scope. Skips nested functions that are in non-render contexts.
fn check_nested_ref_access(
    hir: &HIR,
    outer_ref_names: &FxHashSet<String>,
    non_render_ids: &FxHashSet<IdentifierId>,
) -> bool {
    let mut local_ref_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut local_ref_names: FxHashSet<String> = outer_ref_names.clone();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.type_ == Type::Ref {
                local_ref_ids.insert(instr.lvalue.identifier.id);
                if let Some(name) = &instr.lvalue.identifier.name {
                    local_ref_names.insert(name.clone());
                }
            }

            if let Some(name) = &instr.lvalue.identifier.name
                && (is_ref_name(name) || local_ref_names.contains(name))
            {
                local_ref_ids.insert(instr.lvalue.identifier.id);
                local_ref_names.insert(name.clone());
            }

            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.type_ == Type::Ref
                        || local_ref_ids.contains(&place.identifier.id)
                    {
                        local_ref_ids.insert(instr.lvalue.identifier.id);
                    }
                    if let Some(name) = &place.identifier.name
                        && (is_ref_name(name) || local_ref_names.contains(name))
                    {
                        local_ref_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    if local_ref_ids.contains(&value.identifier.id) {
                        local_ref_ids.insert(instr.lvalue.identifier.id);
                        if let Some(name) = &lvalue.identifier.name {
                            local_ref_names.insert(name.clone());
                        }
                    }
                }
                InstructionValue::PropertyLoad { property, .. } => {
                    if is_ref_name(property) {
                        local_ref_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                _ => {}
            }
        }
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let is_ref_current = match &instr.value {
                InstructionValue::PropertyLoad { object, property, .. } => {
                    property == "current" && local_ref_ids.contains(&object.identifier.id)
                }
                InstructionValue::PropertyStore { object, property, .. } => {
                    property == "current" && local_ref_ids.contains(&object.identifier.id)
                }
                _ => false,
            };
            if is_ref_current {
                return true;
            }

            // Recurse into further nested functions, skipping non-render callbacks
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func } => {
                    if !non_render_ids.contains(&instr.lvalue.identifier.id)
                        && check_nested_ref_access(
                            &lowered_func.body,
                            &local_ref_names,
                            non_render_ids,
                        )
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }

    false
}

/// Check if a name looks like a React ref.
fn is_ref_name(name: &str) -> bool {
    name.ends_with("Ref") || name.ends_with("ref") || name == "ref"
}
