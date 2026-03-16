// DIVERGENCE: Upstream detects frozen-value mutations inside
// InferMutableRanges.ts as part of the abstract interpretation.
// Our port uses a post-effects validation pass that tracks freeze/mutate
// by variable name, because our HIR creates fresh IdentifierIds per
// Place reference — there is no single stable ID across references.

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{
    AliasingEffect, ArrayElement, DestructureArrayItem, DestructurePattern, DestructureTarget, HIR,
    IdentifierId, InstructionId, InstructionValue, ObjectPropertyKey, Place,
};
use rustc_hash::{FxHashMap, FxHashSet};

const FROZEN_MUTATION_ERROR: &str = "This value cannot be modified. Modifying a value used \
     previously in JSX is not allowed. Consider moving the \
     modification before the JSX expression.";

/// Check if a name looks like a React hook (starts with "use" + uppercase, or is "use").
fn is_hook_name(name: &str) -> bool {
    if name == "use" {
        return true;
    }
    if let Some(rest) = name.strip_prefix("use") {
        rest.starts_with(|c: char| c.is_ascii_uppercase())
    } else {
        false
    }
}

/// Resolve a place's name: check id_to_source_name first, fall back to identifier.name.
fn resolve_name<'a>(
    place: &'a Place,
    id_to_source_name: &'a FxHashMap<IdentifierId, &'a str>,
) -> Option<&'a str> {
    id_to_source_name.get(&place.identifier.id).copied().or(place.identifier.name.as_deref())
}

/// Detect mutations to values that have been frozen (used in JSX or passed to hooks).
///
/// After `infer_mutation_aliasing_effects` runs, each instruction has computed effects.
/// This pass walks instructions in program order and tracks which variable names have
/// been frozen. Any mutation to a frozen variable is an error.
///
/// Since the HIR creates fresh IdentifierIds for every Place reference, we track by
/// variable name using a lvalue-ID → source-variable-name mapping built from
/// `LoadLocal`/`LoadContext` instructions.
pub fn validate_no_mutation_after_freeze(
    hir: &HIR,
    errors: &mut ErrorCollector,
    param_names: &[String],
) {
    // Build lvalue_id → source variable name map (borrows from HIR, no clones).
    // When LoadLocal { place: x_ref } → lvalue_temp, this maps lvalue_temp's ID to "x".
    let mut id_to_source_name: FxHashMap<IdentifierId, &str> = FxHashMap::default();

    // Also track: hook return lvalue IDs (for pre-freeze of useContext/useState results)
    let mut hook_return_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    // Map: function lvalue ID → captured variable names (for hook-call-freezes-captures)
    let mut func_captures: FxHashMap<IdentifierId, Vec<&str>> = FxHashMap::default();
    // Map: function name → captured variable names (resolved through LoadLocal chains)
    let mut name_to_func_captures: FxHashMap<&str, Vec<&str>> = FxHashMap::default();

    // Alias tracking: maps aliased_name → source_name for StoreLocal assignments.
    // Used to propagate freeze through aliases (e.g., `let y = x; freeze(x); mutate(y)`).
    let mut alias_map: FxHashMap<&str, &str> = FxHashMap::default();

    // Derivation chain: maps lvalue_id → source_object_id for PropertyLoad, GetIterator,
    // IteratorNext. Used to detect mutations on values derived from frozen collections
    // (e.g., `for (const x of props.items) { x.modified = true }`).
    let mut load_source: FxHashMap<IdentifierId, IdentifierId> = FxHashMap::default();

    // Map: function lvalue ID → reference to lowered function body
    // Used for re-checking nested function bodies after hook calls freeze their captures.
    let mut func_bodies: FxHashMap<IdentifierId, &HIR> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        // Map phi place IDs to names for freeze propagation through phi nodes
        for phi in &block.phis {
            if let Some(name) = &phi.place.identifier.name {
                id_to_source_name.insert(phi.place.identifier.id, name);
            }
            for (_, operand) in &phi.operands {
                if let Some(name) = &operand.identifier.name {
                    id_to_source_name.entry(operand.identifier.id).or_insert(name);
                }
            }
        }

        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_source_name.insert(instr.lvalue.identifier.id, name);
                    }
                    // Track derivation: LoadLocal lvalue derives from the place's ID
                    load_source.insert(instr.lvalue.identifier.id, place.identifier.id);
                }
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_source_name.insert(instr.lvalue.identifier.id, name);
                        // Record alias: lname was assigned from value's source name
                        if let Some(vname) = id_to_source_name
                            .get(&value.identifier.id)
                            .copied()
                            .or(value.identifier.name.as_deref())
                        {
                            alias_map.insert(name, vname);
                        }
                    }
                    // Track derivation: StoreLocal lvalue derives from value
                    load_source.insert(lvalue.identifier.id, value.identifier.id);
                    load_source.insert(instr.lvalue.identifier.id, value.identifier.id);
                }
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_source_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::CallExpression { callee, .. } => {
                    // Detect hook calls for pre-freeze of return values
                    let callee_name = id_to_source_name
                        .get(&callee.identifier.id)
                        .copied()
                        .or(callee.identifier.name.as_deref());
                    if callee_name.is_some_and(is_hook_name) {
                        hook_return_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                InstructionValue::FunctionExpression { lowered_func, .. } => {
                    // Collect captured variable names from function context and body.
                    // DIVERGENCE: Upstream populates HIRFunction.context during building.
                    // Our builder leaves context empty for nested arrows/functions, so
                    // we also scan the inner body for LoadLocal/LoadContext references
                    // to names that exist in the outer scope's id_to_source_name map
                    // but are not declared inside the inner function.
                    let mut captures: Vec<&str> = lowered_func
                        .context
                        .iter()
                        .filter_map(|p| {
                            id_to_source_name
                                .get(&p.identifier.id)
                                .copied()
                                .or(p.identifier.name.as_deref())
                        })
                        .collect();
                    // Scan inner body for outer-scope references
                    let inner_locals = collect_inner_declared_names(&lowered_func.body);
                    for (_, inner_block) in &lowered_func.body.blocks {
                        for inner_instr in &inner_block.instructions {
                            if let InstructionValue::LoadLocal { place }
                            | InstructionValue::LoadContext { place } = &inner_instr.value
                                && let Some(name) = &place.identifier.name
                                && !inner_locals.contains(name.as_str())
                                && !captures.contains(&name.as_str())
                            {
                                captures.push(name);
                            }
                        }
                    }
                    if !captures.is_empty() {
                        func_captures.insert(instr.lvalue.identifier.id, captures.clone());
                        if let Some(name) = &instr.lvalue.identifier.name {
                            name_to_func_captures.insert(name, captures);
                        }
                    }
                    // Store reference to function body for post-hook recheck
                    func_bodies.insert(instr.lvalue.identifier.id, &lowered_func.body);
                }
                // Track derivation chains for frozen-value propagation
                InstructionValue::PropertyLoad { object, .. }
                | InstructionValue::ComputedLoad { object, .. } => {
                    load_source.insert(instr.lvalue.identifier.id, object.identifier.id);
                }
                InstructionValue::GetIterator { collection } => {
                    load_source.insert(instr.lvalue.identifier.id, collection.identifier.id);
                }
                InstructionValue::IteratorNext { iterator, .. } => {
                    load_source.insert(instr.lvalue.identifier.id, iterator.identifier.id);
                }
                _ => {}
            }
            // Fallback: map lvalue's own name (for instruction types not matched above)
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_source_name.insert(instr.lvalue.identifier.id, name);
            }

            // Also map ALL operand place IDs to names. This is critical because
            // our HIR creates fresh IdentifierIds per Place reference, so an
            // operand in a JSX child or function arg has a different ID than the
            // LoadLocal lvalue it came from. By mapping operand IDs too, we can
            // resolve names when Freeze/Mutate effects reference operand places.
            map_operand_ids(&instr.value, &mut id_to_source_name);
        }
    }

    // DIVERGENCE: Upstream tracks freeze transitively through the abstract
    // interpreter (InferMutableRanges). We approximate by pre-freezing hook
    // return values and all their destructured targets at their definition site.
    // This over-freezes setters (e.g., setState from useState) but in practice
    // setters are never mutated via property stores, so no false positives arise.
    //
    // DIVERGENCE: We also pre-freeze function parameters (e.g., "props" in component
    // functions, "options" in hooks). Upstream freezes params transitively through
    // InferMutableRanges; we approximate by marking them frozen at the start.
    // The param_names are extracted from the HIRFunction's params list by the pipeline.
    let mut pre_frozen: FxHashSet<&str> = FxHashSet::default();

    // Pre-freeze function parameters: component props, hook arguments, etc.
    for name in param_names {
        pre_frozen.insert(name);
    }

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Propagate hook_return_ids through StoreLocal chains
            match &instr.value {
                InstructionValue::StoreLocal { value, lvalue, .. }
                | InstructionValue::StoreContext { value, lvalue } => {
                    if hook_return_ids.contains(&value.identifier.id) {
                        hook_return_ids.insert(instr.lvalue.identifier.id);
                        if let Some(name) = &lvalue.identifier.name {
                            pre_frozen.insert(name);
                        }
                    }
                }
                // Destructure of hook return: const [state, setState] = useState(...)
                // Freezes ALL destructured targets. This over-freezes the setter,
                // but setters are never mutated via property stores in practice.
                InstructionValue::Destructure { value, lvalue_pattern } => {
                    if hook_return_ids.contains(&value.identifier.id) {
                        collect_frozen_from_destructure(lvalue_pattern, &mut pre_frozen);
                    }
                }
                _ => {}
            }
        }
    }

    // Build a name → last mutation instruction ID map from aliasing effects.
    // This runs at Pass 16.5 (before infer_mutation_aliasing_ranges), so we
    // derive range information from the effects computed by Pass 16.
    // For each variable name, find the last instruction that has a Mutate-like
    // effect on it. This tells us the extent of valid mutations.
    let mut name_to_last_mutate: FxHashMap<&str, InstructionId> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    let mutated_id = match effect {
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value } => {
                            Some(&value.identifier.id)
                        }
                        _ => None,
                    };
                    if let Some(id) = mutated_id
                        && let Some(name) = id_to_source_name.get(id).copied()
                    {
                        let entry = name_to_last_mutate.entry(name).or_insert(instr.id);
                        if instr.id > *entry {
                            *entry = instr.id;
                        }
                    }
                }
            }
            // Also track MethodCall, PropertyStore as mutations
            match &instr.value {
                InstructionValue::MethodCall { receiver, .. } => {
                    if let Some(name) = id_to_source_name.get(&receiver.identifier.id).copied() {
                        let entry = name_to_last_mutate.entry(name).or_insert(instr.id);
                        if instr.id > *entry {
                            *entry = instr.id;
                        }
                    }
                }
                InstructionValue::PropertyStore { object, .. }
                | InstructionValue::ComputedStore { object, .. } => {
                    if let Some(name) = id_to_source_name.get(&object.identifier.id).copied() {
                        let entry = name_to_last_mutate.entry(name).or_insert(instr.id);
                        if instr.id > *entry {
                            *entry = instr.id;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Walk instructions in program order, tracking frozen variable names
    let mut frozen_names: FxHashSet<&str> = pre_frozen;

    // Propagate pre-freeze through aliases: if `frozen` is pre-frozen and
    // `x = frozen` was recorded, then `x` should also be frozen.
    let initial_frozen: Vec<&str> = frozen_names.iter().copied().collect();
    for name in initial_frozen {
        propagate_freeze_through_aliases(name, &mut frozen_names, &alias_map);
    }

    for (_, block) in &hir.blocks {
        // Process phi nodes first: if any operand is frozen, the phi output is frozen.
        // This handles cases like: `x = cond ? frozen : {}; x.property = true`
        for phi in &block.phis {
            let phi_is_frozen = phi.operands.iter().any(|(_, operand)| {
                id_to_source_name
                    .get(&operand.identifier.id)
                    .copied()
                    .or(operand.identifier.name.as_deref())
                    .is_some_and(|name| frozen_names.contains(name))
            });
            if phi_is_frozen {
                if let Some(name) = &phi.place.identifier.name {
                    frozen_names.insert(name);
                }
                if let Some(name) = id_to_source_name.get(&phi.place.identifier.id) {
                    frozen_names.insert(name);
                }
            }
        }

        for instr in &block.instructions {
            // First: process freeze effects to update frozen_names.
            //
            // RANGE GUARD (hook calls only): For Freeze effects from hook calls
            // (CallExpression/MethodCall), only freeze if the value's mutable range
            // has ended. Hook calls may or may not actually freeze their arguments
            // — the range data tells us whether later valid mutations exist.
            //
            // For JSX expressions, freezes are ALWAYS applied: a value used in JSX
            // is immutable — any subsequent mutation is the error we're detecting.
            // Applying the range guard to JSX would suppress detection of the very
            // mutations we're looking for (the range extends past JSX precisely
            // because of the invalid mutation).
            let is_jsx_instruction = matches!(
                instr.value,
                InstructionValue::JsxExpression { .. } | InstructionValue::JsxFragment { .. }
            );

            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    match effect {
                        AliasingEffect::Freeze { value, .. }
                        | AliasingEffect::ImmutableCapture { from: value, .. } => {
                            let name = id_to_source_name
                                .get(&value.identifier.id)
                                .copied()
                                .or(value.identifier.name.as_deref());

                            if let Some(name) = name {
                                // For non-JSX freezes (hook calls), apply mutation range guard:
                                // only freeze if there are no mutations to this value after
                                // this instruction. If the value is mutated after the hook call,
                                // the hook didn't actually freeze it — the mutation is still valid.
                                if !is_jsx_instruction {
                                    let last_mutate = name_to_last_mutate.get(name).copied();
                                    if last_mutate.is_some_and(|m| m > instr.id) {
                                        continue;
                                    }
                                }

                                frozen_names.insert(name);
                                propagate_freeze_through_aliases(
                                    name,
                                    &mut frozen_names,
                                    &alias_map,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Hook calls freeze captured variables of function arguments.
            // e.g., useIdentity(() => { mutate(x) }) freezes x after the call.
            if let InstructionValue::CallExpression { callee, args } = &instr.value {
                let callee_name = resolve_name(callee, &id_to_source_name);
                if callee_name.is_some_and(is_hook_name) {
                    for arg in args {
                        // Check if this arg is a function with captures
                        let arg_captures = func_captures.get(&arg.identifier.id).or_else(|| {
                            let arg_name = resolve_name(arg, &id_to_source_name)?;
                            name_to_func_captures.get(arg_name)
                        });
                        if let Some(captures) = arg_captures {
                            for &captured_name in captures {
                                frozen_names.insert(captured_name);
                            }
                        }

                        // Re-check the function argument's body for mutations to
                        // now-frozen variables (the function was defined before the
                        // hook call froze its captures).
                        if let Some(fn_body) = func_bodies.get(&arg.identifier.id)
                            && check_nested_function_mutation(fn_body, &frozen_names)
                        {
                            errors.push(CompilerError::invalid_react_with_kind(
                                instr.loc,
                                FROZEN_MUTATION_ERROR,
                                DiagnosticKind::ImmutabilityViolation,
                            ));
                            return;
                        }
                    }
                }
            }

            // Check instruction-level mutations (MethodCall, PropertyStore, etc.)
            if check_instruction_mutation_extended(
                instr,
                &id_to_source_name,
                &load_source,
                &frozen_names,
            ) {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    FROZEN_MUTATION_ERROR,
                    DiagnosticKind::ImmutabilityViolation,
                ));
                return;
            }

            // Check for mutations inside nested function bodies (closures).
            // If a FunctionExpression captures frozen variables, scan its body
            // for mutations to those variables.
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && !frozen_names.is_empty()
                && check_nested_function_mutation(&lowered_func.body, &frozen_names)
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    FROZEN_MUTATION_ERROR,
                    DiagnosticKind::ImmutabilityViolation,
                ));
                return;
            }

            // Also check explicit Mutate effects from the aliasing pass
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    let mutated_name = match effect {
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value }
                        | AliasingEffect::MutateFrozen { place: value, .. } => {
                            id_to_source_name.get(&value.identifier.id).copied()
                        }
                        _ => None,
                    };

                    if let Some(name) = mutated_name
                        && frozen_names.contains(name)
                    {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            FROZEN_MUTATION_ERROR,
                            DiagnosticKind::ImmutabilityViolation,
                        ));
                        return;
                    }
                }
            }
        }
    }
}

/// Propagate freeze status through aliases: if `name` is frozen,
/// any variable that is an alias of `name` should also be frozen.
fn propagate_freeze_through_aliases<'a>(
    name: &'a str,
    frozen_names: &mut FxHashSet<&'a str>,
    alias_map: &FxHashMap<&'a str, &'a str>,
) {
    // Find all names that alias `name` (reverse lookup: alias → source)
    for (&alias_name, &source_name) in alias_map {
        if source_name == name && !frozen_names.contains(alias_name) {
            frozen_names.insert(alias_name);
            // Recurse for transitive aliases
            propagate_freeze_through_aliases(alias_name, frozen_names, alias_map);
        }
    }
}

/// Check if an identifier is transitively derived from a frozen source via
/// PropertyLoad, GetIterator, IteratorNext, LoadLocal, StoreLocal chains.
/// Used to detect mutations on values like iterator items from frozen collections.
fn is_derived_from_frozen(
    id: IdentifierId,
    id_to_source_name: &FxHashMap<IdentifierId, &str>,
    load_source: &FxHashMap<IdentifierId, IdentifierId>,
    frozen_names: &FxHashSet<&str>,
) -> bool {
    let mut current = id;
    let mut depth = 0;
    let mut visited = FxHashSet::default();
    loop {
        if depth > 15 || visited.contains(&current) {
            break false; // Prevent infinite loops
        }
        visited.insert(current);
        depth += 1;

        // Check if current ID's source name is frozen
        if let Some(name) = id_to_source_name.get(&current)
            && frozen_names.contains(name)
        {
            return true;
        }

        // Follow the derivation chain
        if let Some(&parent_id) = load_source.get(&current) {
            current = parent_id;
        } else {
            // Chain is broken — try name-based bridging: if current ID has a name,
            // find any other ID with the same name that has a load_source entry.
            // This bridges SSA gaps where the same variable has different IDs across blocks.
            if let Some(name) = id_to_source_name.get(&current) {
                let bridge = load_source.iter().find(|(src_id, _)| {
                    !visited.contains(src_id)
                        && id_to_source_name.get(src_id).copied() == Some(*name)
                });
                if let Some((bridged_id, _)) = bridge {
                    let bridged_id = *bridged_id;
                    current = bridged_id;
                    continue;
                }
            }
            break false;
        }
    }
}

/// Collect all variable names declared within an inner function body.
/// Used to distinguish local declarations from outer-scope captures.
fn collect_inner_declared_names(hir: &HIR) -> FxHashSet<&str> {
    let mut names = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(name) = &lvalue.identifier.name {
                        names.insert(name.as_str());
                    }
                }
                InstructionValue::StoreLocal {
                    lvalue,
                    type_:
                        Some(
                            crate::hir::types::InstructionKind::Let
                            | crate::hir::types::InstructionKind::Const
                            | crate::hir::types::InstructionKind::Var,
                        ),
                    ..
                } => {
                    if let Some(name) = &lvalue.identifier.name {
                        names.insert(name.as_str());
                    }
                }
                _ => {}
            }
        }
    }
    names
}

/// Extended mutation check that also detects mutations on values derived from
/// frozen sources (e.g., iterator items of frozen collections).
fn check_instruction_mutation_extended(
    instr: &crate::hir::types::Instruction,
    id_to_source_name: &FxHashMap<IdentifierId, &str>,
    load_source: &FxHashMap<IdentifierId, IdentifierId>,
    frozen_names: &FxHashSet<&str>,
) -> bool {
    let check_frozen = |id: &IdentifierId| -> bool {
        id_to_source_name.get(id).is_some_and(|name| frozen_names.contains(name))
            || is_derived_from_frozen(*id, id_to_source_name, load_source, frozen_names)
    };

    match &instr.value {
        // MethodCall: only flag if (a) receiver is DIRECTLY frozen (not just
        // derived from frozen — PropertyLoad creates new values, not aliases),
        // and (b) there's a Mutate-like effect that targets the RECEIVER
        // specifically (not just any operand). Read-only methods like .at()
        // have conditional mutation effects on their arguments but NOT on
        // the receiver. Mutating methods like .push() have effects on the
        // receiver.
        InstructionValue::MethodCall { receiver, .. } => {
            // Direct-only frozen check: don't use is_derived_from_frozen.
            // PropertyLoad from a frozen source creates a new value; method
            // calls on derived values (items.at(i)) are NOT mutations of the
            // frozen source. Only flag when the receiver itself is directly
            // in frozen_names (e.g., x.push() where x was used in JSX).
            let is_directly_frozen = id_to_source_name
                .get(&receiver.identifier.id)
                .is_some_and(|name| frozen_names.contains(name));
            if !is_directly_frozen {
                return false;
            }
            // Check if this instruction has any Mutate-like effect (on any operand).
            // Method calls on frozen receivers with ANY mutation effect are
            // considered mutations of the receiver, since we lack method
            // signatures to know which operand is actually mutated.
            instr.effects.as_ref().is_some_and(|effects| {
                effects.iter().any(|e| {
                    matches!(
                        e,
                        AliasingEffect::Mutate { .. }
                            | AliasingEffect::MutateConditionally { .. }
                            | AliasingEffect::MutateTransitive { .. }
                            | AliasingEffect::MutateTransitiveConditionally { .. }
                            | AliasingEffect::MutateFrozen { .. }
                    )
                })
            })
        }
        InstructionValue::PropertyStore { object, .. }
        | InstructionValue::ComputedStore { object, .. } => check_frozen(&object.identifier.id),
        InstructionValue::PropertyDelete { object, .. }
        | InstructionValue::ComputedDelete { object, .. } => check_frozen(&object.identifier.id),
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => check_frozen(&lvalue.identifier.id),
        _ => false,
    }
}

/// Collect variable names from a destructure pattern and add to frozen set.
/// Used for hook return destructuring: const [state, setter] = useState(...)
fn collect_frozen_from_destructure<'a>(
    pattern: &'a DestructurePattern,
    frozen: &mut FxHashSet<&'a str>,
) {
    match pattern {
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(DestructureTarget::Place(p)) => {
                        if let Some(name) = &p.identifier.name {
                            frozen.insert(name);
                        }
                    }
                    DestructureArrayItem::Value(DestructureTarget::Pattern(nested)) => {
                        collect_frozen_from_destructure(nested, frozen);
                    }
                    DestructureArrayItem::Spread(p) => {
                        if let Some(name) = &p.identifier.name {
                            frozen.insert(name);
                        }
                    }
                    DestructureArrayItem::Hole => {}
                }
            }
            if let Some(rest) = rest
                && let Some(name) = &rest.identifier.name
            {
                frozen.insert(name);
            }
        }
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                match &prop.value {
                    DestructureTarget::Place(p) => {
                        if let Some(name) = &p.identifier.name {
                            frozen.insert(name);
                        }
                    }
                    DestructureTarget::Pattern(nested) => {
                        collect_frozen_from_destructure(nested, frozen);
                    }
                }
            }
            if let Some(rest) = rest
                && let Some(name) = &rest.identifier.name
            {
                frozen.insert(name);
            }
        }
    }
}

/// Check if a nested function body contains mutations to any of the outer frozen variables.
/// This handles cases like: const onChange = () => { x.value = ...; } where x is frozen.
fn check_nested_function_mutation(hir: &HIR, outer_frozen: &FxHashSet<&str>) -> bool {
    // Build a local id_to_source_name for the nested function's HIR
    let mut local_id_map: FxHashMap<IdentifierId, &str> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        local_id_map.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::StoreContext { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name {
                        local_id_map.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(name) = &lvalue.identifier.name {
                        local_id_map.insert(instr.lvalue.identifier.id, name);
                    }
                }
                _ => {}
            }
            if let Some(name) = &instr.lvalue.identifier.name {
                local_id_map.insert(instr.lvalue.identifier.id, name);
            }
            map_operand_ids(&instr.value, &mut local_id_map);
        }
    }

    // Check for mutations using outer frozen names
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if check_instruction_mutation(instr, &local_id_map, outer_frozen) {
                return true;
            }

            // Also check Mutate effects
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    let mutated_name = match effect {
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value }
                        | AliasingEffect::MutateFrozen { place: value, .. } => {
                            local_id_map.get(&value.identifier.id).copied()
                        }
                        _ => None,
                    };
                    if let Some(name) = mutated_name
                        && outer_frozen.contains(name)
                    {
                        return true;
                    }
                }
            }

            // Recurse into nested nested functions (depth-limited by HIR structure)
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && check_nested_function_mutation(&lowered_func.body, outer_frozen)
            {
                return true;
            }
        }
    }

    false
}

/// Map operand place IDs to variable names for all places in an instruction value.
/// This covers the ID disconnect: operand places in JSX children, function args, etc.
/// have fresh IDs that differ from the LoadLocal lvalue IDs.
fn map_operand_ids<'a>(value: &'a InstructionValue, id_map: &mut FxHashMap<IdentifierId, &'a str>) {
    let mut map_place = |place: &'a Place| {
        if let Some(name) = &place.identifier.name {
            id_map.entry(place.identifier.id).or_insert(name);
        }
    };

    match value {
        InstructionValue::LoadLocal { place }
        | InstructionValue::LoadContext { place }
        | InstructionValue::Await { value: place }
        | InstructionValue::GetIterator { collection: place }
        | InstructionValue::IteratorNext { iterator: place, .. }
        | InstructionValue::NextPropertyOf { value: place }
        | InstructionValue::UnaryExpression { value: place, .. }
        | InstructionValue::TypeCastExpression { value: place, .. } => {
            map_place(place);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            map_place(lvalue);
            map_place(value);
        }
        InstructionValue::DeclareLocal { lvalue, .. }
        | InstructionValue::DeclareContext { lvalue } => {
            map_place(lvalue);
        }
        InstructionValue::Destructure { value, .. } => {
            map_place(value);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            map_place(left);
            map_place(right);
        }
        InstructionValue::CallExpression { callee, args }
        | InstructionValue::NewExpression { callee, args } => {
            map_place(callee);
            for arg in args {
                map_place(arg);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            map_place(receiver);
            for arg in args {
                map_place(arg);
            }
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            map_place(object);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            map_place(object);
            map_place(value);
        }
        InstructionValue::ComputedLoad { object, property } => {
            map_place(object);
            map_place(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            map_place(object);
            map_place(property);
            map_place(value);
        }
        InstructionValue::ComputedDelete { object, property } => {
            map_place(object);
            map_place(property);
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            map_place(tag);
            for attr in props {
                map_place(&attr.value);
            }
            for child in children {
                map_place(child);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                map_place(child);
            }
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                map_place(&prop.value);
                if let ObjectPropertyKey::Computed(key) = &prop.key {
                    map_place(key);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    ArrayElement::Expression(p) | ArrayElement::Spread(p) => map_place(p),
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                map_place(sub);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value: tpl } => {
            map_place(tag);
            for sub in &tpl.subexpressions {
                map_place(sub);
            }
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            map_place(lvalue);
        }
        InstructionValue::StoreGlobal { value, .. } => {
            map_place(value);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            map_place(decl);
            for dep in deps {
                map_place(dep);
            }
        }
        // No operands to map
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. } => {}
    }
}

/// Check if an instruction directly mutates a frozen value via its instruction value.
/// This catches cases where the aliasing effects don't generate explicit Mutate effects,
/// such as MethodCall receivers (x.push()) and PropertyStore (x.prop = ...).
fn check_instruction_mutation(
    instr: &crate::hir::types::Instruction,
    id_to_source_name: &FxHashMap<IdentifierId, &str>,
    frozen_names: &FxHashSet<&str>,
) -> bool {
    let check_frozen = |id: &IdentifierId| -> bool {
        id_to_source_name.get(id).is_some_and(|name| frozen_names.contains(name))
    };

    match &instr.value {
        // x.push(...), x.splice(...), etc. — method call may mutate receiver
        InstructionValue::MethodCall { receiver, .. } => check_frozen(&receiver.identifier.id),
        // x.prop = value — direct property mutation
        InstructionValue::PropertyStore { object, .. }
        | InstructionValue::ComputedStore { object, .. } => check_frozen(&object.identifier.id),
        // delete x[i]
        InstructionValue::PropertyDelete { object, .. }
        | InstructionValue::ComputedDelete { object, .. } => check_frozen(&object.identifier.id),
        // ++x, x++
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => check_frozen(&lvalue.identifier.id),
        _ => false,
    }
}
