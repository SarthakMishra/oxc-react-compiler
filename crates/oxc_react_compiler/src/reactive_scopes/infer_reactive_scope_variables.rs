use std::rc::Rc;

use crate::hir::types::{
    DeclarationId, DestructureArrayItem, DestructurePattern, DestructureTarget, HIR, IdentifierId,
    InstructionId, InstructionValue, MutableRange, Place, ReactiveScope, ScopeId, SourceLocation,
    Type,
};
use crate::utils::disjoint_set::DisjointSet;
use rustc_hash::{FxHashMap, FxHashSet};

// DIVERGENCE: Upstream InferReactiveScopeVariables uses a forward walk over
// instructions to group identifiers into scopes by mutable-range overlap.
// This implementation uses a union-find (DisjointSet) data structure, which
// is algorithmically equivalent but avoids repeated linear scans when merging
// scope groups.
/// Infer reactive scope variables using DisjointSet (union-find).
///
/// Algorithm:
/// 1. For each instruction with mutable_range > 1 or that allocates:
///    - Union the lvalue with all mutable operands
///    - If any operand is reactive, the set becomes reactive
/// 2. For phi nodes with mutated values, union all operands
/// 3. Each disjoint set becomes a ReactiveScope
pub fn infer_reactive_scope_variables(
    hir: &mut HIR,
    param_ids: &[IdentifierId],
    use_mutable_range: bool,
) -> Vec<ReactiveScope> {
    let param_id_set: FxHashSet<IdentifierId> = param_ids.iter().copied().collect();
    let mut dsu: DisjointSet<IdentifierId> = DisjointSet::new();
    let mut ranges: FxHashMap<IdentifierId, MutableRange> = FxHashMap::default();
    let mut is_reactive: FxHashMap<IdentifierId, bool> = FxHashMap::default();
    // is_allocating_id: for sentinel scope detection — only literal allocations
    // (objects, arrays, JSX, functions) and non-hook, non-primitive calls whose
    // result escapes. This is MORE restrictive than may_allocate.
    let mut is_allocating_id: FxHashSet<IdentifierId> = FxHashSet::default();
    // is_mutable_id: broader — any instruction that produces a potentially mutable
    // value (includes all may_allocate instructions).
    let mut is_mutable_id: FxHashSet<IdentifierId> = FxHashSet::default();

    // Build id-to-name map for resolving callee names through LoadGlobal/LoadLocal.
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
        }
    }

    // Map from identifier to its last_use instruction (for effective range computation)
    let mut last_use_map: FxHashMap<IdentifierId, InstructionId> = FxHashMap::default();

    // Phase 1: Collect all identifiers and their mutable ranges
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let id = instr.lvalue.identifier.id;
            dsu.make_set(id);
            ranges.insert(id, instr.lvalue.identifier.mutable_range);
            last_use_map.insert(id, instr.lvalue.identifier.last_use);
            is_reactive.insert(id, instr.lvalue.reactive);
            if is_allocating_for_sentinel(
                &instr.value,
                &instr.lvalue.identifier.type_,
                &id_to_name,
                instr.lvalue.identifier.last_use,
                instr.id,
            ) {
                is_allocating_id.insert(id);
                is_mutable_id.insert(id);
            }
        }
        for phi in &block.phis {
            let id = phi.place.identifier.id;
            dsu.make_set(id);
            ranges.insert(id, phi.place.identifier.mutable_range);
            is_reactive.insert(id, phi.place.reactive);
        }
    }

    // Collect IDs produced by param destructures — these should not be unioned
    // into reactive scopes because they represent function parameter values that
    // should remain external scope dependencies. Upstream achieves this by placing
    // param destructures outside reactive scope boundaries.
    let mut param_destructure_target_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::Destructure { value, lvalue_pattern } = &instr.value
                && param_id_set.contains(&value.identifier.id)
            {
                let target_ids = collect_destructure_target_ids(lvalue_pattern);
                for tid in target_ids {
                    param_destructure_target_ids.insert(tid);
                }
            }
        }
    }

    // DIVERGENCE: Upstream treats property paths (e.g. `props.a` vs `props.b`) as
    // independent scoping units even though they share the same root identifier.
    // We achieve this by assigning virtual IdentifierIds to each unique
    // (root_id, property_path) combination. The union-find operates on these virtual
    // IDs, and we map them back to root IDs when building scopes.
    let max_real_id = {
        let mut max_id = 0u32;
        for (_, block) in &hir.blocks {
            for instr in &block.instructions {
                max_id = max_id.max(instr.lvalue.identifier.id.0);
                for place in collect_operand_places(&instr.value) {
                    max_id = max_id.max(place.identifier.id.0);
                }
            }
            for phi in &block.phis {
                max_id = max_id.max(phi.place.identifier.id.0);
                for (_, operand) in &phi.operands {
                    max_id = max_id.max(operand.identifier.id.0);
                }
            }
        }
        max_id
    };
    let mut next_virtual_id = max_real_id + 1;
    // Map from (root_id, property_path_strings) -> virtual IdentifierId
    let mut path_to_virtual: FxHashMap<(IdentifierId, Vec<String>), IdentifierId> =
        FxHashMap::default();
    // Map from virtual IdentifierId -> root IdentifierId
    let mut virtual_to_root: FxHashMap<IdentifierId, IdentifierId> = FxHashMap::default();

    /// Resolve a Place to a (possibly virtual) IdentifierId.
    /// If the Place has a non-empty property_path, returns a virtual ID unique to
    /// that (root_id, path) combination. Otherwise returns the root ID directly.
    fn resolve_place_id(
        place: &Place,
        path_to_virtual: &mut FxHashMap<(IdentifierId, Vec<String>), IdentifierId>,
        virtual_to_root: &mut FxHashMap<IdentifierId, IdentifierId>,
        next_virtual_id: &mut u32,
    ) -> IdentifierId {
        if place.property_path.is_empty() {
            place.identifier.id
        } else {
            let key = (
                place.identifier.id,
                place.property_path.iter().map(|e| e.property.clone()).collect::<Vec<_>>(),
            );
            *path_to_virtual.entry(key).or_insert_with(|| {
                let vid = IdentifierId(*next_virtual_id);
                *next_virtual_id += 1;
                virtual_to_root.insert(vid, place.identifier.id);
                vid
            })
        }
    }

    // Pre-scan: register virtual IDs for all operand places with property paths,
    // and populate ranges/last_use_map for virtual IDs (copied from root).
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            for place in collect_operand_places(&instr.value) {
                if !place.property_path.is_empty() {
                    let vid = resolve_place_id(
                        place,
                        &mut path_to_virtual,
                        &mut virtual_to_root,
                        &mut next_virtual_id,
                    );
                    let root_id = place.identifier.id;
                    // Copy range and last_use from root identifier to virtual ID
                    if let Some(&range) = ranges.get(&root_id) {
                        ranges.entry(vid).or_insert(range);
                    }
                    if let Some(&last_use) = last_use_map.get(&root_id) {
                        last_use_map.entry(vid).or_insert(last_use);
                    }
                    if let Some(&reactive) = is_reactive.get(&root_id) {
                        is_reactive.entry(vid).or_insert(reactive);
                    }
                    dsu.make_set(vid);
                }
            }
        }
    }

    // Declarations map: DeclarationId -> IdentifierId, used for phi handling.
    // Upstream tracks this to union phi places with their declaration identifiers.
    let mut declarations: FxHashMap<DeclarationId, IdentifierId> = FxHashMap::default();

    // Helper: check if operand is mutable at instruction (upstream's isMutable)
    // isMutable(instr, place) ≡ instr.id >= range.start && instr.id < range.end
    let is_mutable = |instr_id: InstructionId, op_id: IdentifierId| -> bool {
        if let Some(&op_range) = ranges.get(&op_id) {
            let effective_end = if use_mutable_range {
                op_range.end.0
            } else {
                let op_last_use = last_use_map.get(&op_id).copied().unwrap_or(InstructionId(0));
                op_range.end.0.max(if op_last_use > InstructionId(0) {
                    op_last_use.0 + 1
                } else {
                    0
                })
            };
            instr_id.0 >= op_range.start.0 && instr_id.0 < effective_end
        } else {
            false
        }
    };

    // Phase 2: Union identifiers that should be in the same scope.
    // Matches upstream findDisjointMutableValues with isMutable checks on operands
    // and declarations map for phi handling.
    for (_, block) in &hir.blocks {
        // Phi handling: union phi operands when the phi's mutable range extends
        // beyond its definition (non-trivial range).
        for phi in &block.phis {
            let phi_id = phi.place.identifier.id;
            let phi_range = phi.place.identifier.mutable_range;
            if phi_range.end.0 > phi_range.start.0 + 1 {
                for (_, operand) in &phi.operands {
                    dsu.make_set(operand.identifier.id);
                    let _ = dsu.union(phi_id, operand.identifier.id);
                }
            }
        }

        for instr in &block.instructions {
            let lvalue_id = instr.lvalue.identifier.id;
            let lvalue_range = instr.lvalue.identifier.mutable_range;
            let instr_id = instr.id;

            let mut operands: Vec<IdentifierId> = Vec::new();

            // Upstream: if lvalue has non-trivial range OR instruction allocates,
            // add the lvalue to operands
            let lvalue_effective_end = if use_mutable_range {
                lvalue_range.end.0
            } else {
                let lvalue_last_use = instr.lvalue.identifier.last_use;
                lvalue_range.end.0.max(if lvalue_last_use > InstructionId(0) {
                    lvalue_last_use.0 + 1
                } else {
                    0
                })
            };
            if lvalue_effective_end > lvalue_range.start.0 + 1
                || is_allocating_for_sentinel(
                    &instr.value,
                    &instr.lvalue.identifier.type_,
                    &id_to_name,
                    instr.lvalue.identifier.last_use,
                    instr.id,
                )
            {
                operands.push(lvalue_id);
            }

            // Collect operands and declarations per instruction kind.
            // First, handle declarations for StoreLocal/StoreContext/DeclareLocal/DeclareContext
            // (matching upstream's declareIdentifier calls).
            match &instr.value {
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    if let Some(decl_id) = lvalue.identifier.declaration_id {
                        declarations.entry(decl_id).or_insert(lvalue.identifier.id);
                    }
                }
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::StoreContext { lvalue, .. } => {
                    if let Some(decl_id) = lvalue.identifier.declaration_id {
                        declarations.entry(decl_id).or_insert(lvalue.identifier.id);
                    }
                }
                _ => {}
            }

            // Collect operand IDs and filter by isMutable + start > 0.
            // Upstream: for each operand, check isMutable(instr, operand) &&
            // operand.identifier.mutableRange.start > 0 (exclude globals).
            // Use resolve_place_id to get virtual IDs for property-path operands,
            // so that `props.a` and `props.b` are treated as independent scoping units.
            let all_places = collect_operand_places(&instr.value);
            for place in &all_places {
                let op_id = resolve_place_id(
                    place,
                    &mut path_to_virtual,
                    &mut virtual_to_root,
                    &mut next_virtual_id,
                );
                if is_mutable(instr_id, op_id) {
                    operands.push(op_id);
                }
            }

            // Filter out param destructure targets
            operands.retain(|id| !param_destructure_target_ids.contains(id));

            if !operands.is_empty() {
                dsu.union_many(&operands);
            }
        }
    }

    // Phase 3: Build ReactiveScopes from disjoint sets and map identifiers to scope
    // indices. We store indices into the `scopes` vec rather than cloning
    // ReactiveScope for every member, avoiding O(members) heap allocations.
    let sets = dsu.sets();
    let mut scope_id_counter = 0u32;
    let mut scopes = Vec::new();
    let mut id_to_scope_idx: FxHashMap<IdentifierId, usize> = FxHashMap::default();

    for (_, members) in sets {
        // Compute merged range for the scope
        let mut merged_range =
            MutableRange { start: InstructionId(u32::MAX), end: InstructionId(0) };

        for &member in &members {
            if let Some(&range) = ranges.get(&member) {
                let effective_end = if use_mutable_range {
                    range.end.0
                } else {
                    let member_last_use =
                        last_use_map.get(&member).copied().unwrap_or(InstructionId(0));
                    range.end.0.max(if member_last_use > InstructionId(0) {
                        member_last_use.0 + 1
                    } else {
                        0
                    })
                };
                if range.start.0 > 0 {
                    merged_range.start = InstructionId(merged_range.start.0.min(range.start.0));
                }
                merged_range.end = InstructionId(merged_range.end.0.max(effective_end));
            }
        }

        let mut any_reactive = false;
        for &member in &members {
            if is_reactive.get(&member).copied().unwrap_or(false) {
                any_reactive = true;
                break;
            }
        }

        // Check if any member is an allocating instruction (for sentinel scopes)
        let any_allocating = members.iter().any(|m| is_allocating_id.contains(m));
        let any_mutable = members.iter().any(|m| is_mutable_id.contains(m));

        // Create scope if:
        // - any_allocating (sentinel scope for identity memoization), OR
        // - any_reactive AND any_mutable (reactive computation producing a mutable value)
        if (any_allocating || (any_reactive && any_mutable))
            && merged_range.end.0 > merged_range.start.0
            && merged_range.start.0 > 0
        {
            let scope_idx = scopes.len();
            let scope = ReactiveScope {
                id: ScopeId(scope_id_counter),
                range: merged_range,
                dependencies: Vec::new(),
                declarations: Vec::new(),
                reassignments: Vec::new(),
                early_return_value: None,
                merged: Vec::new(),
                loc: SourceLocation::default(),
                is_allocating: any_allocating && !any_reactive,
            };
            scopes.push(scope);
            for &member in &members {
                // Map virtual IDs back to their root real IDs for scope membership.
                // Both the virtual ID and the root ID get the scope assignment so
                // that Phase 4 (propagation) can look up either form.
                let real_id = virtual_to_root.get(&member).copied().unwrap_or(member);
                id_to_scope_idx.insert(real_id, scope_idx);
                if virtual_to_root.contains_key(&member) {
                    // Also keep the virtual ID mapped for consistency
                    id_to_scope_idx.insert(member, scope_idx);
                }
            }
            scope_id_counter += 1;
        }
    }

    // Phase 4: Propagate scope membership to consuming instructions.
    // If an instruction uses a scoped operand, the instruction's lvalue should also be
    // in the same scope. Also propagate through Destructure pattern targets.
    //
    // PERF: The previous implementation used a `while changed` fixed-point loop that
    // re-scanned all blocks on each iteration, yielding O(N*K) work where K is the
    // longest chain of scope-propagating instructions (worst-case O(N^2) for deeply
    // nested JSX trees like canvas-sidebar). Because HIR blocks and their instructions
    // are in forward data-flow order (SSA), a single forward pass is sufficient:
    // by the time we visit an instruction, all of its operands have already been
    // processed, so scope membership propagates transitively in one sweep.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let lvalue_id = instr.lvalue.identifier.id;

            // If this instruction is already scoped, propagate to Destructure pattern targets.
            // Exception: don't propagate into targets of param destructures — these values
            // come from function parameters and should remain external scope dependencies.
            // Upstream places param destructures outside scope boundaries; we achieve the
            // same effect by excluding their targets from scope membership.
            if let Some(&scope_idx) = id_to_scope_idx.get(&lvalue_id) {
                if let InstructionValue::Destructure { lvalue_pattern, value } = &instr.value
                    && !param_id_set.contains(&value.identifier.id)
                {
                    let target_ids = collect_destructure_target_ids(lvalue_pattern);
                    for tid in target_ids {
                        id_to_scope_idx.entry(tid).or_insert(scope_idx);
                    }
                }
                continue;
            }

            // Check if any operand is in a scope
            let operand_ids = collect_operand_ids(&instr.value);
            for op_id in &operand_ids {
                if let Some(&scope_idx) = id_to_scope_idx.get(op_id) {
                    id_to_scope_idx.insert(lvalue_id, scope_idx);
                    break;
                }
            }
        }
    }

    // Phase 5: Assign scopes back to identifiers in the HIR.
    // Wrap each scope in Rc once, then share via Rc::clone (cheap pointer copy)
    // instead of deep-cloning the entire ReactiveScope struct per identifier.
    let rc_scopes: Vec<Rc<ReactiveScope>> = scopes.iter().map(|s| Rc::new(s.clone())).collect();
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(&idx) = id_to_scope_idx.get(&instr.lvalue.identifier.id) {
                instr.lvalue.identifier.scope = Some(Rc::clone(&rc_scopes[idx]));
            }
        }
        for phi in &mut block.phis {
            if let Some(&idx) = id_to_scope_idx.get(&phi.place.identifier.id) {
                phi.place.identifier.scope = Some(Rc::clone(&rc_scopes[idx]));
            }
        }
    }

    scopes
}

/// Pull unscoped instructions into their consuming scope.
///
/// After `infer_reactive_scope_variables` assigns scopes based on mutable ranges,
/// some instructions that produce values consumed exclusively within one scope
/// are NOT members of that scope. This pass pulls those "orphan" instructions
/// into their consuming scope.
///
/// Algorithm:
///   loop until no changes:
///     for each instruction I with scope = None:
///       consumers = all instructions that use I.lvalue.identifier.id as an operand
///       if consumers is empty: skip
///       consumer_scopes = unique set of scope IDs from consumers (ignoring None)
///       if consumer_scopes has exactly 1 scope S:
///         I.lvalue.identifier.scope = Some(S)
///         changed = true
///
/// Key constraints:
/// - Only pull into a scope if ALL consumers with scopes are in that SAME scope
/// - If consumers are in different scopes, the instruction stays unscoped
/// - NEVER override an existing scope assignment
///
/// Upstream: PropagateScopeDependenciesHIR (scope membership propagation aspect)
pub fn propagate_scope_membership_hir(hir: &mut HIR) {
    // Phase 1: Build a map from IdentifierId → ScopeId for all currently-scoped identifiers.
    // Also collect a ReactiveScope clone per ScopeId so we can assign it to newly-scoped IDs.
    let mut id_to_scope_id: FxHashMap<IdentifierId, ScopeId> = FxHashMap::default();
    let mut scope_by_id: FxHashMap<ScopeId, Rc<ReactiveScope>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                id_to_scope_id.insert(instr.lvalue.identifier.id, scope.id);
                scope_by_id.entry(scope.id).or_insert_with(|| Rc::clone(scope));
            }
        }
        for phi in &block.phis {
            if let Some(ref scope) = phi.place.identifier.scope {
                id_to_scope_id.insert(phi.place.identifier.id, scope.id);
                scope_by_id.entry(scope.id).or_insert_with(|| Rc::clone(scope));
            }
        }
    }

    // Phase 2: Build a consumer map: producer_id → set of consumer lvalue IDs.
    // A "consumer" of producer P is any instruction whose operands include P.
    let mut consumers: FxHashMap<IdentifierId, Vec<IdentifierId>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let consumer_id = instr.lvalue.identifier.id;
            let operand_ids = collect_operand_ids(&instr.value);
            for op_id in operand_ids {
                consumers.entry(op_id).or_default().push(consumer_id);
            }
        }
        // Terminal operands are also consumers. Terminals don't have lvalues,
        // so we use a sentinel: if a terminal consumes an ID, that consumer
        // has scope = None (terminals are not scoped instructions).
        // We don't need to track terminal consumers explicitly -- if an ID
        // is only consumed by a terminal and nothing else, it has no scoped
        // consumers, so it stays unscoped. If it's consumed by both a terminal
        // and a scoped instruction, the terminal adds a "None" consumer which
        // means the scopes won't be unanimous -> stays unscoped. This is correct.
    }

    // Phase 3: Fixed-point iteration to propagate scope membership.
    loop {
        let mut changed = false;

        for (_, block) in &hir.blocks {
            for instr in &block.instructions {
                let id = instr.lvalue.identifier.id;

                // Skip if already scoped
                if id_to_scope_id.contains_key(&id) {
                    continue;
                }

                // Get all consumers of this instruction's lvalue
                let Some(consumer_ids) = consumers.get(&id) else {
                    continue;
                };

                if consumer_ids.is_empty() {
                    continue;
                }

                // Collect unique scope IDs from consumers (ignoring unscoped consumers)
                let mut unique_scope: Option<ScopeId> = None;
                let mut has_unscoped_consumer = false;
                let mut multiple_scopes = false;

                for consumer_id in consumer_ids {
                    match id_to_scope_id.get(consumer_id) {
                        Some(&scope_id) => match unique_scope {
                            None => unique_scope = Some(scope_id),
                            Some(existing) if existing == scope_id => {} // same scope
                            Some(_) => {
                                multiple_scopes = true;
                                break;
                            }
                        },
                        None => {
                            has_unscoped_consumer = true;
                        }
                    }
                }

                // Only assign if ALL consumers that have scopes agree on the same scope,
                // there is at least one scoped consumer, and no unscoped consumers remain.
                //
                // DIVERGENCE: We use a conservative approach -- only pull in
                // when ALL consumers are scoped AND agree on the same scope.
                // This avoids premature assignment that could be wrong if
                // unscoped consumers later get assigned to a different scope.
                if !multiple_scopes
                    && !has_unscoped_consumer
                    && let Some(scope_id) = unique_scope
                {
                    id_to_scope_id.insert(id, scope_id);
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    // Phase 4: Write back scope assignments to the HIR.
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            let id = instr.lvalue.identifier.id;
            if instr.lvalue.identifier.scope.is_none()
                && let Some(&scope_id) = id_to_scope_id.get(&id)
                && let Some(scope) = scope_by_id.get(&scope_id)
            {
                instr.lvalue.identifier.scope = Some(Rc::clone(scope));
            }
        }
        for phi in &mut block.phis {
            let id = phi.place.identifier.id;
            if phi.place.identifier.scope.is_none()
                && let Some(&scope_id) = id_to_scope_id.get(&id)
                && let Some(scope) = scope_by_id.get(&scope_id)
            {
                phi.place.identifier.scope = Some(Rc::clone(scope));
            }
        }
    }
}

/// Allocating check for sentinel scope detection and scope creation gating.
/// Excludes hook calls and calls whose results don't escape.
/// Used for the `is_allocating` flag on ReactiveScope.
fn is_allocating_for_sentinel(
    value: &InstructionValue,
    lvalue_type: &Type,
    id_to_name: &FxHashMap<IdentifierId, String>,
    last_use: InstructionId,
    instr_id: InstructionId,
) -> bool {
    match value {
        InstructionValue::ObjectExpression { .. }
        | InstructionValue::ArrayExpression { .. }
        | InstructionValue::JsxExpression { .. }
        | InstructionValue::JsxFragment { .. }
        | InstructionValue::NewExpression { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. } => true,
        InstructionValue::CallExpression { callee, .. } => {
            if matches!(lvalue_type, Type::Primitive(_)) {
                return false;
            }
            let name = callee
                .identifier
                .name
                .as_deref()
                .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));
            if name.is_some_and(|n| n.starts_with("use") && n.len() > 3) {
                return false;
            }
            last_use > instr_id
        }
        InstructionValue::MethodCall { property, .. } => {
            if matches!(lvalue_type, Type::Primitive(_)) {
                return false;
            }
            if property.starts_with("use") && property.len() > 3 {
                return false;
            }
            last_use > instr_id
        }
        InstructionValue::TaggedTemplateExpression { .. } => {
            if matches!(lvalue_type, Type::Primitive(_)) {
                return false;
            }
            last_use > instr_id
        }
        _ => false,
    }
}

/// Collect all identifier IDs referenced as operands in an instruction value.
fn collect_operand_ids(value: &InstructionValue) -> Vec<IdentifierId> {
    let mut ids = Vec::new();

    match value {
        InstructionValue::LoadLocal { place } => {
            ids.push(place.identifier.id);
        }
        InstructionValue::StoreLocal { lvalue, value, .. } => {
            ids.push(lvalue.identifier.id);
            ids.push(value.identifier.id);
        }
        InstructionValue::LoadContext { place } => {
            ids.push(place.identifier.id);
        }
        InstructionValue::StoreContext { lvalue, value } => {
            ids.push(lvalue.identifier.id);
            ids.push(value.identifier.id);
        }
        InstructionValue::DeclareLocal { lvalue, .. } => {
            ids.push(lvalue.identifier.id);
        }
        InstructionValue::DeclareContext { lvalue } => {
            ids.push(lvalue.identifier.id);
        }
        InstructionValue::Destructure { value, .. } => {
            ids.push(value.identifier.id);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            ids.push(left.identifier.id);
            ids.push(right.identifier.id);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            ids.push(value.identifier.id);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            ids.push(lvalue.identifier.id);
        }
        InstructionValue::CallExpression { callee, args, .. } => {
            ids.push(callee.identifier.id);
            for arg in args {
                ids.push(arg.identifier.id);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            ids.push(receiver.identifier.id);
            for arg in args {
                ids.push(arg.identifier.id);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            ids.push(callee.identifier.id);
            for arg in args {
                ids.push(arg.identifier.id);
            }
        }
        InstructionValue::PropertyLoad { object, .. } => {
            ids.push(object.identifier.id);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            ids.push(object.identifier.id);
            ids.push(value.identifier.id);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            ids.push(object.identifier.id);
            ids.push(property.identifier.id);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            ids.push(object.identifier.id);
            ids.push(property.identifier.id);
            ids.push(value.identifier.id);
        }
        InstructionValue::PropertyDelete { object, .. } => {
            ids.push(object.identifier.id);
        }
        InstructionValue::ComputedDelete { object, property } => {
            ids.push(object.identifier.id);
            ids.push(property.identifier.id);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                ids.push(prop.value.identifier.id);
                if let crate::hir::types::ObjectPropertyKey::Computed(place) = &prop.key {
                    ids.push(place.identifier.id);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Expression(p)
                    | crate::hir::types::ArrayElement::Spread(p) => {
                        ids.push(p.identifier.id);
                    }
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            ids.push(tag.identifier.id);
            for attr in props {
                ids.push(attr.value.identifier.id);
            }
            for child in children {
                ids.push(child.identifier.id);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                ids.push(child.identifier.id);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                ids.push(sub.identifier.id);
            }
        }
        InstructionValue::Await { value } => {
            ids.push(value.identifier.id);
        }
        InstructionValue::GetIterator { collection } => {
            ids.push(collection.identifier.id);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            ids.push(iterator.identifier.id);
        }
        InstructionValue::NextPropertyOf { value } => {
            ids.push(value.identifier.id);
        }
        InstructionValue::TypeCastExpression { value, .. } => {
            ids.push(value.identifier.id);
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            ids.push(tag.identifier.id);
            for sub in &value.subexpressions {
                ids.push(sub.identifier.id);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => {
            ids.push(value.identifier.id);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            ids.push(decl.identifier.id);
            for dep in deps {
                ids.push(dep.identifier.id);
            }
        }
        // No operands
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. } => {}
    }

    ids
}

/// Collect references to all operand Places in an instruction value.
/// Unlike `collect_operand_ids`, this preserves property_path information
/// so callers can distinguish `props.a` from `props.b`.
fn collect_operand_places(value: &InstructionValue) -> Vec<&Place> {
    let mut places = Vec::new();

    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            places.push(place);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            places.push(lvalue);
            places.push(value);
        }
        InstructionValue::DeclareLocal { lvalue, .. }
        | InstructionValue::DeclareContext { lvalue } => {
            places.push(lvalue);
        }
        InstructionValue::Destructure { value, .. } => {
            places.push(value);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            places.push(left);
            places.push(right);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            places.push(value);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            places.push(lvalue);
        }
        InstructionValue::CallExpression { callee, args, .. } => {
            places.push(callee);
            for arg in args {
                places.push(arg);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            places.push(receiver);
            for arg in args {
                places.push(arg);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            places.push(callee);
            for arg in args {
                places.push(arg);
            }
        }
        InstructionValue::PropertyLoad { object, .. } => {
            places.push(object);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            places.push(object);
            places.push(value);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            places.push(object);
            places.push(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            places.push(object);
            places.push(property);
            places.push(value);
        }
        InstructionValue::PropertyDelete { object, .. } => {
            places.push(object);
        }
        InstructionValue::ComputedDelete { object, property } => {
            places.push(object);
            places.push(property);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                places.push(&prop.value);
                if let crate::hir::types::ObjectPropertyKey::Computed(place) = &prop.key {
                    places.push(place);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Expression(p)
                    | crate::hir::types::ArrayElement::Spread(p) => {
                        places.push(p);
                    }
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            places.push(tag);
            for attr in props {
                places.push(&attr.value);
            }
            for child in children {
                places.push(child);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                places.push(child);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                places.push(sub);
            }
        }
        InstructionValue::Await { value } => {
            places.push(value);
        }
        InstructionValue::GetIterator { collection } => {
            places.push(collection);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            places.push(iterator);
        }
        InstructionValue::NextPropertyOf { value } => {
            places.push(value);
        }
        InstructionValue::TypeCastExpression { value, .. } => {
            places.push(value);
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            places.push(tag);
            for sub in &value.subexpressions {
                places.push(sub);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => {
            places.push(value);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            places.push(decl);
            for dep in deps {
                places.push(dep);
            }
        }
        // No operands
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. } => {}
    }

    places
}

/// Collect all identifier IDs from a destructure pattern's targets.
/// This extracts IDs from all bindings created by a destructuring assignment,
/// including nested patterns and rest elements.
fn collect_destructure_target_ids(pattern: &DestructurePattern) -> Vec<IdentifierId> {
    let mut ids = Vec::new();
    collect_destructure_target_ids_inner(pattern, &mut ids);
    ids
}

fn collect_destructure_target_ids_inner(pattern: &DestructurePattern, ids: &mut Vec<IdentifierId>) {
    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                collect_destructure_target_inner(&prop.value, ids);
            }
            if let Some(rest_place) = rest {
                ids.push(rest_place.identifier.id);
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(target) => {
                        collect_destructure_target_inner(target, ids);
                    }
                    DestructureArrayItem::Spread(place) => {
                        ids.push(place.identifier.id);
                    }
                    DestructureArrayItem::Hole => {}
                }
            }
            if let Some(rest_place) = rest {
                ids.push(rest_place.identifier.id);
            }
        }
    }
}

fn collect_destructure_target_inner(target: &DestructureTarget, ids: &mut Vec<IdentifierId>) {
    match target {
        DestructureTarget::Place(place) => {
            ids.push(place.identifier.id);
        }
        DestructureTarget::Pattern(nested) => {
            collect_destructure_target_ids_inner(nested, ids);
        }
    }
}
