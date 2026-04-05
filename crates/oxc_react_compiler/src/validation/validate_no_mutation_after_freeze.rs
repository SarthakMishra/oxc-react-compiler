// Frozen-mutation validation: detect mutations to values that have been frozen.
//
// This pass uses a hybrid approach:
// 1. MutateFrozen effects from infer_mutation_aliasing_effects (Pass 16) catch
//    definite Mutate effects on values frozen in the heap (params, JSX-frozen values
//    when IDs align).
// 2. Targeted instruction-level checks catch cases the effects pass can't handle:
//    - MethodCall on frozen receivers
//    - PropertyStore on frozen values (hook returns, JSX-frozen)
//    - Mutations inside nested function bodies on frozen outer variables
//    - Hook call freezes captures of function arguments
//
// Freeze tracking uses IdentifierId (SSA-unique) rather than variable names.
// After SSA, each reassignment gets a new IdentifierId. This means freezing
// one SSA version of a variable does NOT freeze subsequent reassigned versions.
// This matches upstream's allocation-site-based tracking.

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{
    AliasingEffect, DestructureArrayItem, DestructurePattern, DestructureTarget, HIR, IdSet, IdVec,
    Identifier, IdentifierId, InstructionKind, InstructionValue, Place,
};
use crate::validation::validate_no_ref_access_in_render::is_ref_name;
use rustc_hash::{FxHashMap, FxHashSet};

/// SSA-unique key: (IdentifierId, ssa_version). After SSA, each reassignment
/// gets a new ssa_version while sharing the same IdentifierId. This composite
/// key ensures that freezing one version doesn't affect subsequent reassigned versions.
type SsaId = (IdentifierId, u32);

fn ssa_id(ident: &Identifier) -> SsaId {
    (ident.id, ident.ssa_version)
}

use crate::hir::types::InstructionId;

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

/// Resolve a place's name: check id_to_name first, fall back to identifier.name.
fn resolve_name<'a>(
    id: IdentifierId,
    id_to_name: &'a IdVec<IdentifierId, &'a str>,
    fallback_name: Option<&'a str>,
) -> Option<&'a str> {
    id_to_name.get(id).copied().or(fallback_name)
}

/// Check if an identifier is frozen by SSA-unique key (IdentifierId + ssa_version).
/// After SSA, each reassignment gets a new ssa_version. Using SSA-unique keys
/// means that `x = []; freeze(x); x = []; mutate(x)` correctly allows
/// the mutation because the new `x` has a different ssa_version.
fn is_ssa_frozen(ident: &Identifier, frozen_ids: &FxHashSet<SsaId>) -> bool {
    frozen_ids.contains(&ssa_id(ident))
}

/// Detect mutations to values that have been frozen (used in JSX or passed to hooks).
pub fn validate_no_mutation_after_freeze(
    hir: &HIR,
    errors: &mut ErrorCollector,
    param_names: &[String],
    param_ids: &[IdentifierId],
) {
    let mut id_to_name: IdVec<IdentifierId, &str> = IdVec::new();
    // Primary: ID-based freeze tracking. SSA guarantees each reassignment gets a
    // new IdentifierId, so freezing one version doesn't affect later versions.
    let mut frozen_ids: FxHashSet<SsaId> = FxHashSet::default();
    // Instruction ordering: tracks WHEN each freeze happened. Used by Check 2/3
    // to avoid false positives when block iteration order doesn't match source
    // order (e.g., loop bodies visited after the return block).
    let mut frozen_at: FxHashMap<SsaId, InstructionId> = FxHashMap::default();
    // Secondary: name-based freeze tracking. Used as fallback for phi nodes and
    // cross-SSA references where the same frozen value flows through different IDs.
    // Also used for hook-call-freezes-captures (which tracks by capture name).
    let mut frozen_names: FxHashSet<&str> = FxHashSet::default();
    let mut hook_return_ids: IdSet<IdentifierId> = IdSet::new();
    // Map: function lvalue ID -> captured variable names
    let mut func_captures: IdVec<IdentifierId, Vec<&str>> = IdVec::new();
    // Map: function name -> captured variable names
    let mut name_to_func_captures: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
    // Map: function lvalue ID -> reference to lowered function body
    let mut func_bodies: IdVec<IdentifierId, &HIR> = IdVec::new();

    // Pre-freeze function parameters (props, hook arguments) — by SsaId and name.
    // Params have ssa_version 0 (they're the first definition).
    // Use InstructionId(0) for pre-freeze ordering (before any instruction).
    for &pid in param_ids {
        frozen_ids.insert((pid, 0));
        frozen_at.insert((pid, 0), InstructionId(0));
    }
    for name in param_names {
        frozen_names.insert(name);
    }

    // First pass: build ID-to-name map, track hook returns and function captures
    for (_, block) in &hir.blocks {
        for phi in &block.phis {
            if let Some(name) = &phi.place.identifier.name {
                id_to_name.insert(phi.place.identifier.id, name);
            }
            for (_, operand) in &phi.operands {
                if let Some(name) = &operand.identifier.name
                    && !id_to_name.contains_key(operand.identifier.id)
                {
                    id_to_name.insert(operand.identifier.id, name);
                }
            }
        }

        for instr in &block.instructions {
            if let Some(name) = &instr.lvalue.identifier.name {
                id_to_name.insert(instr.lvalue.identifier.id, name);
            }
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::StoreContext { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(name) = &lvalue.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name);
                    }
                }
                InstructionValue::CallExpression { callee, .. } => {
                    let callee_name = id_to_name
                        .get(callee.identifier.id)
                        .copied()
                        .or(callee.identifier.name.as_deref());
                    if callee_name.is_some_and(is_hook_name) {
                        hook_return_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                InstructionValue::FunctionExpression { lowered_func, .. } => {
                    // Collect captured variable names
                    let mut captures: Vec<&str> = lowered_func
                        .context
                        .iter()
                        .filter_map(|p| {
                            id_to_name
                                .get(p.identifier.id)
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
                    func_bodies.insert(instr.lvalue.identifier.id, &lowered_func.body);
                }
                _ => {}
            }
        }
    }

    // Build the set of all variable names captured by nested functions.
    // Used by Check 6 to detect reassignment of frozen context variables.
    let closure_captured_names: FxHashSet<&str> =
        func_captures.values().flat_map(|names| names.iter().copied()).collect();

    // Collect IDs of FunctionExpressions that are immediately invoked (IIFEs).
    // IIFEs execute during render (same as inline code), so mutations inside
    // them should NOT be flagged as frozen mutations. Complex IIFEs that capture
    // outer variables are not inlined by Pass 6, so we must detect them here.
    //
    // DIVERGENCE: Previous implementation only detected IIFEs where the
    // FunctionExpression and CallExpression were consecutive instructions.
    // The HIR may insert intervening instructions (StoreLocal, etc.) between
    // the function definition and its invocation. Use a two-pass approach:
    // first collect all FunctionExpression lvalue IDs per block, then check
    // if any CallExpression in the same block calls one of them.
    let mut iife_ids: IdSet<IdentifierId> = IdSet::new();
    for (_, block) in &hir.blocks {
        // Pass 1: collect FunctionExpression lvalue IDs in this block
        let fn_expr_ids: IdSet<IdentifierId> = block
            .instructions
            .iter()
            .filter_map(|i| {
                if matches!(i.value, InstructionValue::FunctionExpression { .. }) {
                    Some(i.lvalue.identifier.id)
                } else {
                    None
                }
            })
            .collect();

        if fn_expr_ids.is_empty() {
            continue;
        }

        // Pass 2: any CallExpression whose callee was defined as a
        // FunctionExpression in the same block is an IIFE
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value
                && fn_expr_ids.contains(callee.identifier.id)
            {
                iife_ids.insert(callee.identifier.id);
            }
        }
    }

    // Collect function IDs passed to effect/callback hooks. Mutations inside
    // these lambdas happen after render (effect time / event time) and are safe.
    // Upstream's type system handles this via function effect signatures; we use
    // a name-based allowlist of hooks whose callbacks execute post-render.
    let mut effect_callback_ids: IdSet<IdentifierId> = IdSet::new();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, args, .. } = &instr.value {
                let callee_name = id_to_name
                    .get(callee.identifier.id)
                    .copied()
                    .or(callee.identifier.name.as_deref());
                if callee_name.is_some_and(is_effect_or_callback_hook) {
                    // The first argument to these hooks is the callback
                    if let Some(first_arg) = args.first() {
                        effect_callback_ids.insert(first_arg.identifier.id);
                        // Also track via StoreLocal chains (the arg might be a
                        // temp that was stored from a named function expression)
                        if let Some(name) = resolve_name(
                            first_arg.identifier.id,
                            &id_to_name,
                            first_arg.identifier.name.as_deref(),
                        ) {
                            // Find function IDs with this name
                            for (fid_idx, _) in func_bodies.iter() {
                                let fid = IdentifierId(fid_idx as u32);
                                let fname = id_to_name.get(fid).copied();
                                if fname == Some(name) {
                                    effect_callback_ids.insert(fid);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Pre-freeze hook return values (useState state, useReducer state, etc.)
    // Skip ref-typed values: useRef() returns are designed to be mutable
    // (ref.current can always be modified). Upstream's type system handles
    // this via Type::Ref; we check the lvalue type here.
    let mut ref_ids: IdSet<IdentifierId> = IdSet::new();
    let mut ref_hook_result_ids: IdSet<IdentifierId> = IdSet::new();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Type-based detection
            if matches!(instr.lvalue.identifier.type_, crate::hir::types::Type::Ref) {
                ref_ids.insert(instr.lvalue.identifier.id);
            }
            // useRef() call detection
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                let callee_name = id_to_name
                    .get(callee.identifier.id)
                    .copied()
                    .or(callee.identifier.name.as_deref());
                if callee_name == Some("useRef") {
                    ref_hook_result_ids.insert(instr.lvalue.identifier.id);
                }
            }
            // Track StoreLocal of useRef results
            if let InstructionValue::StoreLocal { lvalue, value, .. } = &instr.value {
                if ref_hook_result_ids.contains(value.identifier.id) {
                    ref_ids.insert(lvalue.identifier.id);
                }
                if matches!(lvalue.identifier.type_, crate::hir::types::Type::Ref) {
                    ref_ids.insert(lvalue.identifier.id);
                }
            }
        }
    }

    // Freeze hook return values by ID, propagating through StoreLocal/Destructure chains
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::StoreLocal { value, lvalue, .. }
                | InstructionValue::StoreContext { value, lvalue } => {
                    if hook_return_ids.contains(value.identifier.id) {
                        hook_return_ids.insert(instr.lvalue.identifier.id);
                        // Freeze by ID — skip refs
                        if !ref_ids.contains(lvalue.identifier.id) {
                            frozen_ids.insert(ssa_id(&lvalue.identifier));
                            if let Some(name) = &lvalue.identifier.name {
                                frozen_names.insert(name);
                            }
                        }
                    }
                }
                InstructionValue::Destructure { value, lvalue_pattern } => {
                    if hook_return_ids.contains(value.identifier.id) {
                        collect_frozen_ids_from_destructure(
                            lvalue_pattern,
                            &mut frozen_ids,
                            &mut frozen_names,
                            &ref_ids,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    // Remove any ref IDs that leaked into frozen_ids via other paths
    frozen_ids.retain(|&(id, _)| !ref_ids.contains(id));

    // Main pass: walk instructions, check for frozen mutations
    for (_, block) in &hir.blocks {
        // DIVERGENCE: Upstream tracks freeze through its type-based abstract interpretation.
        // We propagate freeze through phi nodes: if ANY operand is frozen (by ID or name),
        // the phi result is also frozen. This handles `x = cond ? frozenValue : {}`
        // where upstream marks the phi as "could be frozen" and rejects mutations.
        for phi in &block.phis {
            let any_operand_frozen = phi.operands.iter().any(|(_, operand)| {
                is_ssa_frozen(&operand.identifier, &frozen_ids)
                    || resolve_name(
                        operand.identifier.id,
                        &id_to_name,
                        operand.identifier.name.as_deref(),
                    )
                    .is_some_and(|n| frozen_names.contains(n))
            });
            if any_operand_frozen {
                let sid = ssa_id(&phi.place.identifier);
                frozen_ids.insert(sid);
                if let Some(name) = resolve_name(
                    phi.place.identifier.id,
                    &id_to_name,
                    phi.place.identifier.name.as_deref(),
                ) {
                    frozen_names.insert(name);
                }
            }
        }

        for instr in &block.instructions {
            // Check 1: MutateFrozen effects from the aliasing pass
            // Skip PrefixUpdate/PostfixUpdate: these are variable reassignments (x++),
            // not object mutations. The aliasing pass may generate MutateFrozen for
            // them when the variable is derived from a frozen value (e.g., `let x = props.x; x++`),
            // but reassigning a primitive copy is a local operation, not a mutation of the original.
            if let Some(ref effects) = instr.effects
                && !matches!(
                    instr.value,
                    InstructionValue::PrefixUpdate { .. } | InstructionValue::PostfixUpdate { .. }
                )
            {
                for effect in effects {
                    if matches!(effect, AliasingEffect::MutateFrozen { .. }) {
                        // DIVERGENCE: Skip MutateFrozen from CallExpression on IIFEs.
                        // IIFEs execute during render (not post-render), so mutations
                        // inside them are safe. The aliasing pass may generate
                        // MutateFrozen for the IIFE call when captured values are
                        // transitively frozen.
                        if let InstructionValue::CallExpression { callee, .. } = &instr.value
                            && iife_ids.contains(callee.identifier.id)
                        {
                            continue;
                        }
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            FROZEN_MUTATION_ERROR,
                            DiagnosticKind::ImmutabilityViolation,
                        ));
                        return;
                    }
                }
            }

            // Update frozen_ids from Freeze effects (for instruction-level checks)
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    if let AliasingEffect::Freeze { value, .. } = effect {
                        let sid = ssa_id(&value.identifier);
                        frozen_ids.insert(sid);
                        frozen_at.entry(sid).or_insert(instr.id);
                        if let Some(name) = resolve_name(
                            value.identifier.id,
                            &id_to_name,
                            value.identifier.name.as_deref(),
                        ) {
                            frozen_names.insert(name);
                        }
                    }
                }
            }

            // DIVERGENCE: Upstream propagates freeze transitively in its abstract interpreter.
            // We propagate freeze through load/store chains and property accesses.
            // When a frozen value is stored to a new variable, the new variable
            // is also frozen. When a property is loaded from a frozen object,
            // the loaded value is transitively frozen.
            match &instr.value {
                InstructionValue::StoreLocal { value, lvalue, .. }
                | InstructionValue::StoreContext { value, lvalue } => {
                    // Skip ref-typed lvalues: useRef() returns are mutable by design.
                    // Without this guard, storing a frozen value into a ref variable
                    // would incorrectly freeze it, causing false positives on ref.current writes.
                    if !ref_ids.contains(lvalue.identifier.id) {
                        let value_frozen_by_id = is_ssa_frozen(&value.identifier, &frozen_ids);
                        let value_frozen_by_name = !value_frozen_by_id
                            && resolve_name(
                                value.identifier.id,
                                &id_to_name,
                                value.identifier.name.as_deref(),
                            )
                            .is_some_and(|n| frozen_names.contains(n));
                        if value_frozen_by_id || value_frozen_by_name {
                            let sid = ssa_id(&lvalue.identifier);
                            frozen_ids.insert(sid);
                            frozen_at.entry(sid).or_insert(instr.id);
                            if let Some(name) = resolve_name(
                                lvalue.identifier.id,
                                &id_to_name,
                                lvalue.identifier.name.as_deref(),
                            ) {
                                frozen_names.insert(name);
                            }
                        }
                    }
                }
                InstructionValue::PropertyLoad { object, .. }
                | InstructionValue::ComputedLoad { object, .. } => {
                    let obj_frozen_by_id = is_ssa_frozen(&object.identifier, &frozen_ids);
                    let obj_frozen_by_name = !obj_frozen_by_id
                        && resolve_name(
                            object.identifier.id,
                            &id_to_name,
                            object.identifier.name.as_deref(),
                        )
                        .is_some_and(|n| frozen_names.contains(n));
                    if obj_frozen_by_id || obj_frozen_by_name {
                        // Freeze the loaded value by ID only — don't add the loaded
                        // property's name to frozen_names because that can cause
                        // name collisions with unrelated variables.
                        let sid = ssa_id(&instr.lvalue.identifier);
                        frozen_ids.insert(sid);
                        frozen_at.entry(sid).or_insert(instr.id);
                    }
                }
                // Propagate freeze through iterator chains:
                // `for (const x of frozenObj.items)` → GetIterator(frozen) → IteratorNext(frozen_iter)
                InstructionValue::GetIterator { collection } => {
                    if is_ssa_frozen(&collection.identifier, &frozen_ids)
                        || resolve_name(
                            collection.identifier.id,
                            &id_to_name,
                            collection.identifier.name.as_deref(),
                        )
                        .is_some_and(|n| frozen_names.contains(n))
                    {
                        let sid = ssa_id(&instr.lvalue.identifier);
                        frozen_ids.insert(sid);
                        frozen_at.entry(sid).or_insert(instr.id);
                    }
                }
                InstructionValue::IteratorNext { iterator, .. } => {
                    if is_ssa_frozen(&iterator.identifier, &frozen_ids) {
                        let sid = ssa_id(&instr.lvalue.identifier);
                        frozen_ids.insert(sid);
                        frozen_at.entry(sid).or_insert(instr.id);
                        if let Some(name) = resolve_name(
                            instr.lvalue.identifier.id,
                            &id_to_name,
                            instr.lvalue.identifier.name.as_deref(),
                        ) {
                            frozen_names.insert(name);
                        }
                    }
                }
                InstructionValue::NextPropertyOf { value } => {
                    if is_ssa_frozen(&value.identifier, &frozen_ids)
                        || resolve_name(
                            value.identifier.id,
                            &id_to_name,
                            value.identifier.name.as_deref(),
                        )
                        .is_some_and(|n| frozen_names.contains(n))
                    {
                        let sid = ssa_id(&instr.lvalue.identifier);
                        frozen_ids.insert(sid);
                        frozen_at.entry(sid).or_insert(instr.id);
                    }
                }
                // Propagate freeze through Destructure: if the destructured value
                // is frozen (e.g., props param), all output variables are also frozen.
                InstructionValue::Destructure { value, lvalue_pattern } => {
                    let val_frozen = is_ssa_frozen(&value.identifier, &frozen_ids)
                        || resolve_name(
                            value.identifier.id,
                            &id_to_name,
                            value.identifier.name.as_deref(),
                        )
                        .is_some_and(|n| frozen_names.contains(n));
                    if val_frozen {
                        collect_frozen_ids_from_destructure(
                            lvalue_pattern,
                            &mut frozen_ids,
                            &mut frozen_names,
                            &ref_ids,
                        );
                    }
                }
                _ => {}
            }

            // Hook calls freeze captured variables of function arguments.
            // Skip effect/callback hooks — their callbacks execute after render,
            // so we should not freeze their captures or check their bodies for
            // frozen mutations.
            if let InstructionValue::CallExpression { callee, args, .. } = &instr.value {
                let callee_name = resolve_name(
                    callee.identifier.id,
                    &id_to_name,
                    callee.identifier.name.as_deref(),
                );
                if callee_name.is_some_and(is_hook_name)
                    && !callee_name.is_some_and(is_effect_or_callback_hook)
                {
                    for arg in args {
                        let arg_captures = func_captures.get(arg.identifier.id).or_else(|| {
                            let arg_name = resolve_name(
                                arg.identifier.id,
                                &id_to_name,
                                arg.identifier.name.as_deref(),
                            )?;
                            name_to_func_captures.get(arg_name)
                        });
                        if let Some(captures) = arg_captures {
                            for &captured_name in captures {
                                // Skip ref-typed captures: useRef() returns are designed
                                // to be mutable. Freezing them causes false positives
                                // when callbacks write ref.current inside custom hooks.
                                let is_ref_capture = ref_ids.iter_indices().any(|rid_idx| {
                                    id_to_name.get(IdentifierId(rid_idx as u32)).copied()
                                        == Some(captured_name)
                                });
                                if !is_ref_capture {
                                    frozen_names.insert(captured_name);
                                }
                            }
                        }

                        // Re-check function body for mutations to now-frozen variables
                        if let Some(fn_body) = func_bodies.get(arg.identifier.id)
                            && has_mutation_on_frozen(fn_body, &frozen_ids, &frozen_names)
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

            // Check 2: MethodCall on frozen receiver — only flag if the method
            // is KNOWN to mutate. Read-only methods (.map(), .at(), .filter(),
            // .toString(), .foo()) are safe on frozen values.
            // Upstream uses method signatures; we use a conservative allowlist.
            // Also verify source ordering: only flag if the freeze happened
            // BEFORE this mutation in source order (instruction IDs are monotonic).
            if let InstructionValue::MethodCall { receiver, property, .. } = &instr.value
                && is_place_frozen(receiver, &frozen_ids)
                && is_frozen_before(receiver, instr.id, &frozen_at)
                && is_known_mutating_method(property)
            {
                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    FROZEN_MUTATION_ERROR,
                    DiagnosticKind::ImmutabilityViolation,
                ));
                return;
            }

            // Check 3: PropertyStore/ComputedStore/Delete on frozen values
            // Uses both SSA-id tracking (frozen_ids) and name-based fallback
            // (frozen_names) to catch cases where the object has a fresh SSA ID
            // but the underlying variable name is frozen.
            // NOTE: The name-based path intentionally skips the `is_frozen_before`
            // ordering check. Name-based freezes come from pre-frozen params
            // (always before any instruction) or hook-captures (which freeze at the
            // hook call site). In both cases, the freeze logically precedes any
            // subsequent mutation, so ordering is guaranteed by construction.
            // DIVERGENCE: Upstream uses abstract-interpretation-based freeze tracking
            // in InferMutationAliasingEffects.ts rather than this post-hoc validation.
            match &instr.value {
                InstructionValue::PropertyStore { object, .. }
                | InstructionValue::ComputedStore { object, .. }
                | InstructionValue::PropertyDelete { object, .. }
                | InstructionValue::ComputedDelete { object, .. } => {
                    let id_frozen = is_place_frozen(object, &frozen_ids)
                        && is_frozen_before(object, instr.id, &frozen_at);
                    let name_frozen =
                        !id_frozen && is_place_name_frozen(object, &frozen_names, &id_to_name);
                    if id_frozen || name_frozen {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            FROZEN_MUTATION_ERROR,
                            DiagnosticKind::ImmutabilityViolation,
                        ));
                        return;
                    }
                }
                _ => {}
            }

            // Check 4: Mutations inside nested function bodies on frozen outer variables.
            // Skip lambdas passed to effect/callback hooks — those execute after render,
            // so mutations inside them (e.g., ref.current = x in useEffect) are safe.
            // Skip IIFEs — they execute during render (same as inline code), so
            // their mutations are checked by the normal instruction-level checks.
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && (!frozen_ids.is_empty() || !frozen_names.is_empty())
                && !iife_ids.contains(instr.lvalue.identifier.id)
            {
                if effect_callback_ids.contains(instr.lvalue.identifier.id) {
                    // Check 4b: Even in effect callbacks, mutating frozen props
                    // is still an error. Props (and their destructured properties)
                    // are always frozen and should never be mutated even in effects.
                    // We use the full frozen set — refs are already excluded from
                    // frozen_ids (line 312), so `ref.current = x` in effects won't
                    // false-positive. But ref NAMES might still be in frozen_names,
                    // so we build a ref-excluded name set for safety.
                    let non_ref_frozen_names: FxHashSet<&str> =
                        frozen_names.iter().copied().filter(|n| !is_ref_name(n)).collect();
                    if has_mutation_on_frozen(
                        &lowered_func.body,
                        &frozen_ids,
                        &non_ref_frozen_names,
                    ) {
                        errors.push(CompilerError::invalid_react_with_kind(
                            instr.loc,
                            FROZEN_MUTATION_ERROR,
                            DiagnosticKind::ImmutabilityViolation,
                        ));
                        return;
                    }
                } else if has_mutation_on_frozen(&lowered_func.body, &frozen_ids, &frozen_names) {
                    errors.push(CompilerError::invalid_react_with_kind(
                        instr.loc,
                        FROZEN_MUTATION_ERROR,
                        DiagnosticKind::ImmutabilityViolation,
                    ));
                    return;
                }
            }

            // Check 5: Mutate effects on frozen values (catches cases where
            // the effects pass generates Mutate but the value isn't frozen in the heap).
            // For call instructions (CallExpression, MethodCall, NewExpression), only
            // check definite mutations — conditional ones come from Apply fallback and
            // cause false positives on frozen params passed to functions.
            //
            // Skip PrefixUpdate/PostfixUpdate: these are variable reassignments (a++),
            // not object mutations. The aliasing pass generates Mutate effects for them,
            // but reassigning a param is a local operation that doesn't modify the
            // original value (especially for primitives).
            if let Some(ref effects) = instr.effects {
                let is_call = matches!(
                    instr.value,
                    InstructionValue::CallExpression { .. }
                        | InstructionValue::MethodCall { .. }
                        | InstructionValue::NewExpression { .. }
                );
                let is_update = matches!(
                    instr.value,
                    InstructionValue::PrefixUpdate { .. } | InstructionValue::PostfixUpdate { .. }
                );
                if !is_update {
                    for effect in effects {
                        let mutated_frozen = match effect {
                            AliasingEffect::Mutate { value, .. } => {
                                is_ssa_frozen(&value.identifier, &frozen_ids)
                            }
                            // For call instructions, skip transitive and conditional
                            // mutation effects — these come from Apply fallback and
                            // over-approximate, causing false positives when frozen
                            // params are captured (e.g., `x = {a}` then `mutate(y)`
                            // where `y.x = x`). Direct Mutate effects still catch
                            // real mutations.
                            AliasingEffect::MutateTransitive { value } if !is_call => {
                                is_ssa_frozen(&value.identifier, &frozen_ids)
                            }
                            AliasingEffect::MutateConditionally { value }
                            | AliasingEffect::MutateTransitiveConditionally { value }
                                if !is_call =>
                            {
                                is_ssa_frozen(&value.identifier, &frozen_ids)
                            }
                            _ => false,
                        };
                        if mutated_frozen {
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

            // Check 6: Reassignment of frozen context variables.
            // In for-loops, the iterator variable (e.g. `i`) may be used in JSX
            // (which freezes it) and then reassigned in the loop update (`i += 1`).
            // When the variable is also captured in a closure (context variable),
            // the reassignment is effectively a mutation of shared state.
            // Upstream catches this via its context variable model; we detect it
            // by checking if a `StoreLocal { Reassign }` targets a name that is
            // both frozen and captured by a closure.
            // IMPORTANT: We also verify instruction ordering — the freeze must
            // have happened BEFORE this reassignment (via frozen_at).
            if let InstructionValue::StoreLocal {
                lvalue,
                type_: Some(InstructionKind::Reassign),
                ..
            } = &instr.value
                && let Some(name) = resolve_name(
                    lvalue.identifier.id,
                    &id_to_name,
                    lvalue.identifier.name.as_deref(),
                )
                && frozen_names.contains(name)
                && closure_captured_names.contains(name)
            {
                // Verify the freeze happened before this instruction.
                // Check if any SSA version of this variable was frozen
                // before the current instruction ID.
                let freeze_is_before = frozen_at.iter().any(|(&(fid, _), &freeze_id)| {
                    let frozen_name = id_to_name.get(fid).copied();
                    frozen_name == Some(name) && freeze_id < instr.id
                });
                if freeze_is_before {
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

/// Check if a Place refers to a frozen value by SSA-unique key.
/// Check if a place was frozen BEFORE the given instruction in source order.
/// Returns true if the freeze instruction ID is less than `current_instr_id`,
/// or if the freeze was from pre-freezing (params, which use InstructionId(0)).
/// Returns true if there's no ordering info (conservative: treat as frozen).
fn is_frozen_before(
    place: &Place,
    current_instr_id: InstructionId,
    frozen_at: &FxHashMap<SsaId, InstructionId>,
) -> bool {
    let sid = ssa_id(&place.identifier);
    match frozen_at.get(&sid) {
        Some(&freeze_id) => freeze_id < current_instr_id,
        // No ordering info (frozen via pre-freeze or name-based) — conservatively treat as frozen
        None => true,
    }
}

fn is_place_frozen(place: &Place, frozen_ids: &FxHashSet<SsaId>) -> bool {
    is_ssa_frozen(&place.identifier, frozen_ids)
}

/// Check if a place refers to a frozen value by name-based fallback.
fn is_place_name_frozen(
    place: &Place,
    frozen_names: &FxHashSet<&str>,
    id_to_name: &IdVec<IdentifierId, &str>,
) -> bool {
    let name = id_to_name.get(place.identifier.id).copied().or(place.identifier.name.as_deref());
    name.is_some_and(|n| frozen_names.contains(n))
}

/// Returns true if a hook name represents an effect or callback hook whose
/// first argument (the callback) executes after render, not during render.
/// Mutations inside these callbacks are safe and should not trigger
/// frozen-mutation errors.
fn is_effect_or_callback_hook(name: &str) -> bool {
    matches!(
        name,
        "useEffect"
            | "useLayoutEffect"
            | "useInsertionEffect"
            | "useCallback"
            | "useEffectEvent"
            | "useImperativeHandle"
    )
}

/// Returns true if a method name is known to mutate its receiver.
/// Used to distinguish mutating methods (.push(), .splice()) from
/// read-only ones (.map(), .at(), .filter()) when checking MethodCall
/// on frozen receivers.
fn is_known_mutating_method(method: &str) -> bool {
    matches!(
        method,
        // Array mutating methods
        "push" | "pop" | "shift" | "unshift" | "splice" | "sort" | "reverse"
        | "fill" | "copyWithin"
        // Set/Map mutating methods
        | "add" | "set" | "delete" | "clear"
        // Generic mutating patterns
        | "append" | "remove" | "insert" | "assign"
    )
}

/// Check if a nested function body contains mutations to any frozen value.
/// Uses both ID-based and name-based tracking for completeness.
/// Excludes names that are locally declared in the inner function, since
/// those shadow any outer frozen variable with the same name.
fn has_mutation_on_frozen(
    hir: &HIR,
    outer_frozen_ids: &FxHashSet<SsaId>,
    outer_frozen_names: &FxHashSet<&str>,
) -> bool {
    // Collect locally declared names to exclude from frozen name matching.
    // A local `let a = y;` shadows outer frozen `a` from params.
    let inner_declared = collect_inner_declared_names(hir);
    let effective_frozen_names: FxHashSet<&str> =
        outer_frozen_names.iter().copied().filter(|n| !inner_declared.contains(n)).collect();

    let mut local_id_map: IdVec<IdentifierId, &str> = IdVec::new();

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

            // Check MutateFrozen effects
            // For call instructions, only check definite mutations (not conditional).
            if let Some(ref effects) = instr.effects {
                let is_call = matches!(
                    instr.value,
                    InstructionValue::CallExpression { .. }
                        | InstructionValue::MethodCall { .. }
                        | InstructionValue::NewExpression { .. }
                );
                for effect in effects {
                    if matches!(effect, AliasingEffect::MutateFrozen { .. }) {
                        return true;
                    }
                    let is_mutation = match effect {
                        AliasingEffect::Mutate { value, .. }
                        | AliasingEffect::MutateTransitive { value } => is_inner_frozen(
                            value.identifier.id,
                            &local_id_map,
                            outer_frozen_ids,
                            &effective_frozen_names,
                        ),
                        AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitiveConditionally { value }
                            if !is_call =>
                        {
                            is_inner_frozen(
                                value.identifier.id,
                                &local_id_map,
                                outer_frozen_ids,
                                &effective_frozen_names,
                            )
                        }
                        _ => false,
                    };
                    if is_mutation {
                        return true;
                    }
                }
            }

            // Check instruction-level mutations
            let check_frozen = |id: &IdentifierId| -> bool {
                is_inner_frozen(*id, &local_id_map, outer_frozen_ids, &effective_frozen_names)
            };
            match &instr.value {
                InstructionValue::MethodCall { receiver, property, .. } => {
                    if check_frozen(&receiver.identifier.id) && is_known_mutating_method(property) {
                        return true;
                    }
                }
                InstructionValue::PropertyStore { object, .. }
                | InstructionValue::ComputedStore { object, .. }
                | InstructionValue::PropertyDelete { object, .. }
                | InstructionValue::ComputedDelete { object, .. } => {
                    if check_frozen(&object.identifier.id) {
                        return true;
                    }
                }
                InstructionValue::PrefixUpdate { lvalue, .. }
                | InstructionValue::PostfixUpdate { lvalue, .. } => {
                    if check_frozen(&lvalue.identifier.id) {
                        return true;
                    }
                }
                _ => {}
            }

            // Recurse into nested functions
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value
                && has_mutation_on_frozen(
                    &lowered_func.body,
                    outer_frozen_ids,
                    &effective_frozen_names,
                )
            {
                return true;
            }
        }
    }

    false
}

/// Check if an identifier in an inner function body refers to a frozen outer value.
/// Inner functions don't have SSA versions for outer variables, so we use name-based
/// fallback. The outer frozen_names set tracks names that were frozen in the outer scope.
fn is_inner_frozen(
    id: IdentifierId,
    local_id_map: &IdVec<IdentifierId, &str>,
    _outer_frozen_ids: &FxHashSet<SsaId>,
    outer_frozen_names: &FxHashSet<&str>,
) -> bool {
    // Name-based: resolve the inner ID to a name, check if that name is frozen
    // in the outer scope. This is necessary because inner functions have their own
    // SSA numbering -- their IdentifierIds don't match the outer scope's.
    local_id_map.get(id).is_some_and(|name| outer_frozen_names.contains(name))
}

/// Collect all variable names declared within an inner function body.
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

/// Collect identifiers from a destructure pattern and add to frozen sets (ID and name).
/// Skips ref-typed identifiers.
fn collect_frozen_ids_from_destructure<'a>(
    pattern: &'a DestructurePattern,
    frozen_ids: &mut FxHashSet<SsaId>,
    frozen_names: &mut FxHashSet<&'a str>,
    ref_ids: &IdSet<IdentifierId>,
) {
    match pattern {
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(DestructureTarget::Place(p)) => {
                        if !ref_ids.contains(p.identifier.id) {
                            frozen_ids.insert(ssa_id(&p.identifier));
                            if let Some(name) = &p.identifier.name {
                                frozen_names.insert(name);
                            }
                        }
                    }
                    DestructureArrayItem::Value(DestructureTarget::Pattern(nested)) => {
                        collect_frozen_ids_from_destructure(
                            nested,
                            frozen_ids,
                            frozen_names,
                            ref_ids,
                        );
                    }
                    DestructureArrayItem::Spread(p) => {
                        if !ref_ids.contains(p.identifier.id) {
                            frozen_ids.insert(ssa_id(&p.identifier));
                            if let Some(name) = &p.identifier.name {
                                frozen_names.insert(name);
                            }
                        }
                    }
                    DestructureArrayItem::Hole => {}
                }
            }
            if let Some(rest) = rest
                && !ref_ids.contains(rest.identifier.id)
            {
                frozen_ids.insert(ssa_id(&rest.identifier));
                if let Some(name) = &rest.identifier.name {
                    frozen_names.insert(name);
                }
            }
        }
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                match &prop.value {
                    DestructureTarget::Place(p) => {
                        if !ref_ids.contains(p.identifier.id) {
                            frozen_ids.insert(ssa_id(&p.identifier));
                            if let Some(name) = &p.identifier.name {
                                frozen_names.insert(name);
                            }
                        }
                    }
                    DestructureTarget::Pattern(nested) => {
                        collect_frozen_ids_from_destructure(
                            nested,
                            frozen_ids,
                            frozen_names,
                            ref_ids,
                        );
                    }
                }
            }
            if let Some(rest) = rest
                && !ref_ids.contains(rest.identifier.id)
            {
                frozen_ids.insert(ssa_id(&rest.identifier));
                if let Some(name) = &rest.identifier.name {
                    frozen_names.insert(name);
                }
            }
        }
    }
}
