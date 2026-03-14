use crate::hir::types::{
    HIR, IdentifierId, InstructionValue, ReactiveScopeDeclaration, ReactiveScopeDependency,
    ScopeId, Type,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// Propagate scope dependencies through the HIR.
///
/// For each reactive scope, determine which external values it depends on.
/// These become the "deps" that are checked at runtime to decide whether
/// to recompute the scope's output.
pub fn propagate_scope_dependencies_hir(hir: &mut HIR) {
    // Phase 0: Collect identifiers that should NOT be scope dependencies:
    // - Global values (from LoadGlobal) — never change between renders
    // - Primitive constants (from Primitive/JSXText) — immutable by definition
    let mut non_reactive_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    // Map from identifier name to whether it's known to be non-reactive.
    // Used to propagate non-reactivity through StoreLocal/LoadLocal chains.
    let mut non_reactive_names: FxHashSet<String> = FxHashSet::default();

    // First pass: seed non-reactive IDs from globals, primitives, and stable types
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadGlobal { binding } => {
                    non_reactive_ids.insert(instr.lvalue.identifier.id);
                    // Also register the global binding name so that downstream
                    // LoadLocal instructions referencing the same global (with a
                    // different SSA Place ID from make_named_place) are caught by
                    // the name-based fallback in the fixpoint propagation loop.
                    // TODO(4f): Theoretical shadowing risk if a local variable
                    // has the same name as a global. Same trade-off as SetState/Ref.
                    non_reactive_names.insert(binding.name.clone());
                }
                InstructionValue::Primitive { .. } | InstructionValue::JSXText { .. } => {
                    non_reactive_ids.insert(instr.lvalue.identifier.id);
                }
                _ => {
                    // Stable hook returns (setState, dispatch, ref) are never reactive deps
                    if matches!(instr.lvalue.identifier.type_, Type::SetState | Type::Ref) {
                        non_reactive_ids.insert(instr.lvalue.identifier.id);
                        if let Some(name) = &instr.lvalue.identifier.name {
                            non_reactive_names.insert(name.clone());
                        }
                    }
                }
            }
        }
    }

    // Second pass: propagate non-reactivity through StoreLocal, LoadLocal,
    // and PropertyLoad chains (e.g., Math → Math.max, console → console.log).
    // Uses fixpoint iteration for property chains of arbitrary depth.
    let mut changed = true;
    while changed {
        changed = false;
        for (_, block) in &hir.blocks {
            for instr in &block.instructions {
                if non_reactive_ids.contains(&instr.lvalue.identifier.id) {
                    continue;
                }
                let should_add = match &instr.value {
                    InstructionValue::StoreLocal { value, .. }
                    | InstructionValue::StoreContext { value, .. } => {
                        non_reactive_ids.contains(&value.identifier.id)
                    }
                    InstructionValue::LoadLocal { place }
                    | InstructionValue::LoadContext { place } => {
                        non_reactive_ids.contains(&place.identifier.id)
                            || place
                                .identifier
                                .name
                                .as_deref()
                                .is_some_and(|n| non_reactive_names.contains(n))
                    }
                    // Property access of a non-reactive object (e.g., Math.max,
                    // console.log, JSON.parse) produces a non-reactive value.
                    InstructionValue::PropertyLoad { object, .. } => {
                        non_reactive_ids.contains(&object.identifier.id)
                    }
                    // Destructure of a non-reactive value: all targets are non-reactive.
                    // e.g., const {getNumber} = require('shared-runtime')
                    InstructionValue::Destructure { value, .. } => {
                        non_reactive_ids.contains(&value.identifier.id)
                    }
                    // DIVERGENCE: Upstream doesn't have this rule. We treat CallExpression
                    // results as non-reactive when callee and all args are non-reactive.
                    // This primarily handles `require('shared-runtime')` returning a
                    // stable module object. Safe because truly reactive calls (hooks)
                    // have their return types set separately (Type::SetState, Type::Ref).
                    InstructionValue::CallExpression { callee, args } => {
                        non_reactive_ids.contains(&callee.identifier.id)
                            && args.iter().all(|a| non_reactive_ids.contains(&a.identifier.id))
                    }
                    _ => false,
                };
                if should_add {
                    non_reactive_ids.insert(instr.lvalue.identifier.id);
                    if let Some(name) = &instr.lvalue.identifier.name {
                        non_reactive_names.insert(name.clone());
                    }
                    // Also propagate store/context target names
                    match &instr.value {
                        InstructionValue::StoreLocal { lvalue, .. }
                        | InstructionValue::StoreContext { lvalue, .. } => {
                            if let Some(name) = &lvalue.identifier.name {
                                non_reactive_names.insert(name.clone());
                            }
                        }
                        // Propagate non-reactivity to all destructure targets
                        InstructionValue::Destructure { lvalue_pattern, .. } => {
                            collect_destructure_target_ids(
                                lvalue_pattern,
                                &mut non_reactive_ids,
                                &mut non_reactive_names,
                            );
                        }
                        _ => {}
                    }
                    changed = true;
                }
            }
        }
    }

    // Phase 1: Build maps of scope_id -> identifier IDs and declaration IDs that belong to the scope.
    // We track both IdentifierId (SSA-unique) and DeclarationId (shared across SSA versions of the
    // same source variable). This ensures that when a scope writes `x = a + 1`, subsequent reads
    // of `x` within the scope are NOT treated as external dependencies, even though the read may
    // use a different SSA IdentifierId than the write.
    let mut scope_ids: FxHashMap<ScopeId, FxHashSet<IdentifierId>> = FxHashMap::default();
    let mut scope_written_names: FxHashMap<ScopeId, FxHashSet<String>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                scope_ids.entry(scope.id).or_default().insert(instr.lvalue.identifier.id);
                // Track names of variables written to by store instructions
                match &instr.value {
                    InstructionValue::StoreLocal { lvalue, .. }
                    | InstructionValue::StoreContext { lvalue, .. } => {
                        scope_ids.entry(scope.id).or_default().insert(lvalue.identifier.id);
                        if let Some(name) = &lvalue.identifier.name {
                            scope_written_names.entry(scope.id).or_default().insert(name.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Phase 1.5: Build temporary resolution map.
    //
    // DIVERGENCE: Upstream `PropagateScopeDependencies.ts` uses `collectTemporaries()` to
    // follow LoadLocal → PropertyLoad → ComputedLoad chains, resolving each SSA temporary
    // to its root named variable + property path. Our HIR creates fresh IDs per Place, so
    // we need this mapping to produce proper property-path dependencies like `props.x`
    // instead of just `props`.
    //
    // Map: temp_lvalue_id → (root_identifier, property_path)
    // For LoadLocal { place: x } → temp:  temp → (x.identifier, [])
    // For PropertyLoad { object: temp, property: "x" } → temp2:  temp2 → (x.identifier, ["x"])
    // For ComputedLoad { object: temp, property: p } → temp2:  chain stops (dynamic key)
    use crate::hir::types::DependencyPathEntry;

    /// Lightweight resolution info for SSA temporaries.
    /// Stores only the fields needed for dependency resolution (id, name, loc),
    /// avoiding the full Identifier clone overhead (which includes
    /// Option<Box<ReactiveScope>>, MutableRange, Type, etc.).
    struct TemporaryInfo {
        root_id: IdentifierId,
        root_name: Option<String>,
        root_reactive: bool,
        root_loc: oxc_span::Span,
        path: Vec<DependencyPathEntry>,
    }

    impl TemporaryInfo {
        /// Reconstruct a minimal Identifier for use in ReactiveScopeDependency.
        fn to_identifier(&self) -> crate::hir::types::Identifier {
            crate::hir::types::Identifier {
                id: self.root_id,
                declaration_id: None,
                name: self.root_name.clone(),
                mutable_range: crate::hir::types::MutableRange {
                    start: crate::hir::types::InstructionId(0),
                    end: crate::hir::types::InstructionId(0),
                },
                scope: None,
                type_: crate::hir::types::Type::default(),
                loc: self.root_loc,
            }
        }
    }

    let mut temp_map: FxHashMap<IdentifierId, TemporaryInfo> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    // LoadLocal { place: x } → temp: map temp to root=x, path=[]
                    // But also check if the source place itself is a resolved temp
                    if let Some(resolved) = temp_map.get(&place.identifier.id) {
                        // Chain through: if x was itself a resolved temp, propagate
                        temp_map.insert(
                            instr.lvalue.identifier.id,
                            TemporaryInfo {
                                root_id: resolved.root_id,
                                root_name: resolved.root_name.clone(),
                                root_reactive: resolved.root_reactive,
                                root_loc: resolved.root_loc,
                                path: resolved.path.clone(),
                            },
                        );
                    } else if place.identifier.name.is_some() {
                        // Root named variable
                        temp_map.insert(
                            instr.lvalue.identifier.id,
                            TemporaryInfo {
                                root_id: place.identifier.id,
                                root_name: place.identifier.name.clone(),
                                root_reactive: place.reactive,
                                root_loc: place.identifier.loc,
                                path: Vec::new(),
                            },
                        );
                    }
                }
                InstructionValue::PropertyLoad { object, property } => {
                    // PropertyLoad { object: temp, property: "x" } → temp2
                    // Resolve temp → root, then temp2 → (root, path ++ ["x"])
                    if let Some(resolved) = temp_map.get(&object.identifier.id) {
                        let mut new_path = resolved.path.clone();
                        new_path.push(DependencyPathEntry {
                            property: property.clone(),
                            optional: false,
                        });
                        temp_map.insert(
                            instr.lvalue.identifier.id,
                            TemporaryInfo {
                                root_id: resolved.root_id,
                                root_name: resolved.root_name.clone(),
                                root_reactive: resolved.root_reactive,
                                root_loc: resolved.root_loc,
                                path: new_path,
                            },
                        );
                    } else if object.identifier.name.is_some() {
                        // Direct property load of a named variable (no LoadLocal intermediate)
                        temp_map.insert(
                            instr.lvalue.identifier.id,
                            TemporaryInfo {
                                root_id: object.identifier.id,
                                root_name: object.identifier.name.clone(),
                                root_reactive: object.reactive,
                                root_loc: object.identifier.loc,
                                path: vec![DependencyPathEntry {
                                    property: property.clone(),
                                    optional: false,
                                }],
                            },
                        );
                    }
                }
                // StoreLocal/StoreContext: propagate resolution to the store target
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    if let Some(resolved) = temp_map.get(&value.identifier.id) {
                        // If the value being stored resolves to a root, map the lvalue too
                        // (only for const assignments — reassignments break the chain)
                        if lvalue.identifier.name.is_none() {
                            temp_map.insert(
                                instr.lvalue.identifier.id,
                                TemporaryInfo {
                                    root_id: resolved.root_id,
                                    root_name: resolved.root_name.clone(),
                                    root_reactive: resolved.root_reactive,
                                    root_loc: resolved.root_loc,
                                    path: resolved.path.clone(),
                                },
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Phase 2: For each instruction in a scope, find operands from outside the scope.
    // Uses temp_map to resolve SSA temporaries to root named variables with property paths.
    let mut scope_deps: FxHashMap<ScopeId, Vec<ReactiveScopeDependency>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let scope_id = match &instr.lvalue.identifier.scope {
                Some(scope) => scope.id,
                None => continue,
            };

            let declared_ids = scope_ids.get(&scope_id);
            let written_names = scope_written_names.get(&scope_id);

            // Check if an operand belongs to this scope by:
            // 1. Exact SSA IdentifierId match (instruction lvalues + StoreLocal targets), or
            // 2. Name match against variables written to inside the scope (handles SSA versioning
            //    where the LoadLocal of `x` has a different ID than the StoreLocal that wrote `x`)
            let is_scope_internal = |place: &crate::hir::types::Place| -> bool {
                if declared_ids.is_some_and(|s| s.contains(&place.identifier.id)) {
                    return true;
                }
                if let Some(name) = &place.identifier.name
                    && written_names.is_some_and(|s| s.contains(name))
                {
                    return true;
                }
                false
            };

            // Check if a resolved root identifier is scope-internal by name
            let is_root_scope_internal = |identifier: &crate::hir::types::Identifier| -> bool {
                if declared_ids.is_some_and(|s| s.contains(&identifier.id)) {
                    return true;
                }
                if let Some(name) = &identifier.name
                    && written_names.is_some_and(|s| s.contains(name))
                {
                    return true;
                }
                false
            };

            // Collect READ operands — only values that are consumed (not written to).
            // DIVERGENCE: Skip PropertyLoad's object operand from regular dep collection.
            // PropertyLoad is a "path-building" instruction — its object is an intermediate
            // in a property chain (e.g., `props` in `props.x`). The full property path
            // (e.g., `props.x`) will be resolved through temp_map when the downstream
            // consumer references the PropertyLoad's result. Adding the bare object here
            // would create a dep like `{props, []}` that subsumes `{props, ["x"]}` in
            // derive_minimal_dependencies, losing the property path information.
            let operands = collect_read_operand_places_for_deps(&instr.value);
            for place in operands {
                // Try to resolve through temp_map first
                if let Some(resolved) = temp_map.get(&place.identifier.id) {
                    // Resolved to a root named variable with property path
                    let root = resolved.to_identifier();
                    if !is_root_scope_internal(&root) && !non_reactive_ids.contains(&root.id) {
                        let deps = scope_deps.entry(scope_id).or_default();
                        // Check if already have a dep for this root+path
                        let already = deps
                            .iter()
                            .any(|d| d.identifier.id == root.id && d.path == resolved.path);
                        if !already {
                            deps.push(ReactiveScopeDependency {
                                identifier: root,
                                reactive: resolved.root_reactive,
                                path: resolved.path.clone(),
                            });
                        }
                    }
                } else {
                    // No resolution — use as-is (named variable or unresolved temp)
                    let op_id = place.identifier.id;
                    if !is_scope_internal(place) && !non_reactive_ids.contains(&place.identifier.id)
                    {
                        let deps = scope_deps.entry(scope_id).or_default();
                        let already_added =
                            deps.iter().any(|d| d.identifier.id == op_id && d.path.is_empty());
                        if !already_added {
                            deps.push(ReactiveScopeDependency {
                                identifier: place.identifier.clone(),
                                reactive: place.reactive,
                                path: Vec::new(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Phase 3: Determine declarations (identifiers defined in scope, used outside)
    // Build reverse-use maps:
    //   operand_id → consumer scope IDs (or None if outside scope)
    //   operand_name → consumer scope IDs (for cross-SSA-ID matching)
    // DIVERGENCE: Upstream uses DeclarationId to match across SSA versions. We use
    // both IdentifierId and name-based matching because our HIR creates fresh IDs
    // per Place, so `doubled` in StoreLocal and `doubled` in LoadLocal have
    // different IDs but the same name.
    let mut operand_consumers: FxHashMap<IdentifierId, Vec<Option<ScopeId>>> = FxHashMap::default();
    let mut name_consumers: FxHashMap<String, Vec<Option<ScopeId>>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let consumer_scope = instr.lvalue.identifier.scope.as_ref().map(|s| s.id);
            let operands = collect_operand_places(&instr.value);
            for place in operands {
                operand_consumers.entry(place.identifier.id).or_default().push(consumer_scope);
                if let Some(name) = &place.identifier.name {
                    name_consumers.entry(name.clone()).or_default().push(consumer_scope);
                }
            }
        }
        // Terminal uses are always "outside" any scope (scope = None)
        match &block.terminal {
            crate::hir::types::Terminal::Return { value }
            | crate::hir::types::Terminal::Throw { value } => {
                operand_consumers.entry(value.identifier.id).or_default().push(None);
                if let Some(name) = &value.identifier.name {
                    name_consumers.entry(name.clone()).or_default().push(None);
                }
            }
            crate::hir::types::Terminal::If { test, .. }
            | crate::hir::types::Terminal::Branch { test, .. } => {
                operand_consumers.entry(test.identifier.id).or_default().push(None);
                if let Some(name) = &test.identifier.name {
                    name_consumers.entry(name.clone()).or_default().push(None);
                }
            }
            _ => {}
        }
    }

    // Build scope declarations: identifiers defined inside a scope that are used
    // by instructions outside that scope (or in terminals)
    let mut scope_decls: FxHashMap<ScopeId, Vec<(IdentifierId, ReactiveScopeDeclaration)>> =
        FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                let id = instr.lvalue.identifier.id;
                // Check if this identifier is used by any consumer outside this scope
                let used_outside = operand_consumers.get(&id).is_some_and(|consumers| {
                    consumers.iter().any(|consumer_scope| *consumer_scope != Some(scope.id))
                });

                if used_outside {
                    let decls = scope_decls.entry(scope.id).or_default();
                    if !decls.iter().any(|(did, _)| *did == id) {
                        decls.push((
                            id,
                            ReactiveScopeDeclaration {
                                identifier: instr.lvalue.identifier.clone(),
                                scope: scope.id,
                            },
                        ));
                    }
                }

                // Also check StoreLocal/StoreContext targets: named variables like
                // `doubled` are written via `StoreLocal { lvalue: doubled, value: temp }`.
                // The instruction-level lvalue is an SSA temp, but the actual named
                // variable (`doubled`) is the store target. If it's used outside this
                // scope, it should be a declaration.
                match &instr.value {
                    InstructionValue::StoreLocal { lvalue, .. }
                    | InstructionValue::StoreContext { lvalue, .. } => {
                        let target_id = lvalue.identifier.id;
                        // Check by ID first, then fall back to name-based matching
                        // (needed because StoreLocal target and LoadLocal source
                        // may have different SSA IDs for the same variable).
                        // TODO(4f): Name-based matching can false-positive on
                        // shadowed/reused variable names. DeclarationId alignment
                        // will fix this — same limitation as scope_written_names
                        // and dep_key_set throughout the codebase.
                        let target_used_outside =
                            operand_consumers.get(&target_id).is_some_and(|consumers| {
                                consumers
                                    .iter()
                                    .any(|consumer_scope| *consumer_scope != Some(scope.id))
                            }) || lvalue.identifier.name.as_ref().is_some_and(|name| {
                                name_consumers.get(name).is_some_and(|consumers| {
                                    consumers
                                        .iter()
                                        .any(|consumer_scope| *consumer_scope != Some(scope.id))
                                })
                            });
                        if target_used_outside {
                            let decls = scope_decls.entry(scope.id).or_default();
                            if !decls.iter().any(|(did, _)| *did == target_id) {
                                decls.push((
                                    target_id,
                                    ReactiveScopeDeclaration {
                                        identifier: lvalue.identifier.clone(),
                                        scope: scope.id,
                                    },
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Phase 3.5: Resolve transitive dependencies.
    //
    // When scope A declares `doubled` (computed from `value`) and scope B depends
    // on `doubled`, substitute B's dep on `doubled` with scope A's root deps (`value`).
    // This enables `can_merge_scopes` to detect identical dep sets and merge scopes
    // that operate on the same reactive inputs.
    //
    // DIVERGENCE: Upstream resolves transitive deps during collection via
    // `collectTemporariesSidemap` + `visitOperand`. We do it as a post-pass because
    // our temp_map only chains through unnamed SSA temporaries, not through named
    // scope-declared variables. This achieves the same result.
    //
    // Uses a fixpoint loop to handle multi-level transitivity:
    //   scope A: deps=[value], declares doubled
    //   scope B: deps=[doubled], declares tripled
    //   scope C: deps=[tripled]
    // After pass 1: C's deps become [doubled], after pass 2: C's deps become [value].
    {
        // Map: declared_variable_name → (declaring_scope_id, that scope's deps)
        let mut decl_deps_map: FxHashMap<String, (ScopeId, Vec<ReactiveScopeDependency>)> =
            FxHashMap::default();

        let max_iterations = 10;
        for _iter in 0..max_iterations {
            // Rebuild decl_deps_map from current scope_deps
            decl_deps_map.clear();
            for (scope_id, decl_vec) in &scope_decls {
                if let Some(deps) = scope_deps.get(scope_id)
                    && !deps.is_empty()
                {
                    for (_, decl) in decl_vec {
                        if let Some(name) = &decl.identifier.name {
                            decl_deps_map.insert(name.clone(), (*scope_id, deps.clone()));
                        }
                    }
                }
            }

            // Substitute transitive deps in each scope
            let mut changed = false;
            let scope_ids_vec: Vec<ScopeId> = scope_deps.keys().copied().collect();
            for sid in scope_ids_vec {
                let deps = match scope_deps.get(&sid) {
                    Some(d) => d.clone(),
                    None => continue,
                };
                let mut new_deps: Vec<ReactiveScopeDependency> = Vec::with_capacity(deps.len());
                let mut substituted = false;

                // Helper: check if a dep with the same name+path is already in new_deps
                let has_dep = |new_deps: &[ReactiveScopeDependency],
                               dep: &ReactiveScopeDependency| {
                    new_deps.iter().any(|d| {
                        d.identifier.name.as_ref() == dep.identifier.name.as_ref()
                            && d.path == dep.path
                    })
                };

                for dep in &deps {
                    if dep.path.is_empty()
                        && let Some(name) = &dep.identifier.name
                        && let Some((declaring_scope, root_deps)) = decl_deps_map.get(name)
                        && *declaring_scope != sid
                    {
                        // Transitive: replace with root deps
                        for root_dep in root_deps {
                            if !has_dep(&new_deps, root_dep) {
                                new_deps.push(root_dep.clone());
                            }
                        }
                        substituted = true;
                        continue;
                    }
                    // Keep as-is (direct dep or has property path)
                    if !has_dep(&new_deps, dep) {
                        new_deps.push(dep.clone());
                    }
                }

                if substituted {
                    scope_deps.insert(sid, new_deps);
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }
    }

    // Sort dependencies by identifier name (alphabetical) to match upstream ordering.
    // Babel's PropagateScopeDependencies outputs deps in a stable name-based order,
    // while our insertion-order walk depends on HIR instruction sequence.
    for deps in scope_deps.values_mut() {
        deps.sort_by(|a, b| {
            let a_name = a.identifier.name.as_deref().unwrap_or("");
            let b_name = b.identifier.name.as_deref().unwrap_or("");
            a_name.cmp(b_name)
        });
    }

    // Phase 4: Write the dependencies and declarations back onto ALL instructions
    // in each scope (not just the first one), because `find_scope_in_block` in
    // `build_reactive_function` may read the scope from any instruction.
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(ref mut scope) = instr.lvalue.identifier.scope {
                if let Some(deps) = scope_deps.get(&scope.id) {
                    scope.dependencies.clone_from(deps);
                }
                if let Some(decls) = scope_decls.get(&scope.id) {
                    scope.declarations.clone_from(decls);
                }
            }
        }
    }
}

/// Like `collect_read_operand_places`, but skips PropertyLoad's object operand.
///
/// PropertyLoad is a "path-building" instruction. Its object is an intermediate
/// in a property chain (e.g., `props` in `props.x`). The complete property path
/// will be resolved through the temp_map when the downstream consumer references
/// the PropertyLoad result. Including the bare object here would create a shallow
/// dependency (`props`) that subsumes the deeper one (`props.x`) during
/// `derive_minimal_dependencies`, losing property-path granularity.
fn collect_read_operand_places_for_deps(
    value: &InstructionValue,
) -> Vec<&crate::hir::types::Place> {
    match value {
        // Skip PropertyLoad/PropertyDelete — their object is an intermediate
        // in a property chain, handled via temp_map resolution
        InstructionValue::PropertyLoad { .. } | InstructionValue::PropertyDelete { .. } => {
            Vec::new()
        }
        // Skip LoadLocal/LoadContext — these are "path-building" instructions that
        // load a named variable into an SSA temp. The dependency will be resolved
        // when downstream instructions reference the temp through temp_map.
        // Including the source place here would add a bare dep like `{props, []}`
        // that subsumes deeper property paths like `{props, ["x"]}`.
        InstructionValue::LoadLocal { .. } | InstructionValue::LoadContext { .. } => Vec::new(),
        _ => collect_read_operand_places(value),
    }
}

/// Collect only READ operands — places that are read by the instruction.
/// This excludes write targets (StoreLocal lvalue, DeclareLocal lvalue, etc.)
/// because writes don't constitute dependencies: the scope produces these values,
/// it doesn't consume them.
fn collect_read_operand_places(value: &InstructionValue) -> Vec<&crate::hir::types::Place> {
    let mut places = Vec::new();

    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            places.push(place);
        }
        InstructionValue::StoreLocal { value, .. }
        | InstructionValue::StoreContext { value, .. } => {
            // Only the value being stored is a read; the lvalue is a write target
            places.push(value);
        }
        InstructionValue::DeclareLocal { .. } | InstructionValue::DeclareContext { .. } => {
            // Declarations are pure writes — no read operands
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
            // These read AND write the lvalue — include as read
            places.push(lvalue);
        }
        InstructionValue::CallExpression { callee, args } => {
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
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            places.push(object);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            places.push(object);
            places.push(value);
        }
        InstructionValue::ComputedLoad { object, property } => {
            places.push(object);
            places.push(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            places.push(object);
            places.push(property);
            places.push(value);
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
        InstructionValue::Await { value }
        | InstructionValue::GetIterator { collection: value }
        | InstructionValue::NextPropertyOf { value }
        | InstructionValue::TypeCastExpression { value, .. } => {
            places.push(value);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            places.push(iterator);
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

/// Recursively collect all target place identifiers from a destructure pattern
/// and mark them as non-reactive.
fn collect_destructure_target_ids(
    pattern: &crate::hir::types::DestructurePattern,
    non_reactive_ids: &mut FxHashSet<IdentifierId>,
    non_reactive_names: &mut FxHashSet<String>,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};

    fn mark_place(
        place: &crate::hir::types::Place,
        ids: &mut FxHashSet<IdentifierId>,
        names: &mut FxHashSet<String>,
    ) {
        ids.insert(place.identifier.id);
        if let Some(name) = &place.identifier.name {
            names.insert(name.clone());
        }
    }

    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                match &prop.value {
                    DestructureTarget::Place(place) => {
                        mark_place(place, non_reactive_ids, non_reactive_names);
                    }
                    DestructureTarget::Pattern(nested) => {
                        collect_destructure_target_ids(
                            nested,
                            non_reactive_ids,
                            non_reactive_names,
                        );
                    }
                }
            }
            if let Some(rest) = rest {
                mark_place(rest, non_reactive_ids, non_reactive_names);
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(DestructureTarget::Place(place)) => {
                        mark_place(place, non_reactive_ids, non_reactive_names);
                    }
                    DestructureArrayItem::Value(DestructureTarget::Pattern(nested)) => {
                        collect_destructure_target_ids(
                            nested,
                            non_reactive_ids,
                            non_reactive_names,
                        );
                    }
                    DestructureArrayItem::Spread(place) => {
                        mark_place(place, non_reactive_ids, non_reactive_names);
                    }
                    DestructureArrayItem::Hole => {}
                }
            }
            if let Some(rest) = rest {
                mark_place(rest, non_reactive_ids, non_reactive_names);
            }
        }
    }
}

/// Collect all places referenced as operands in an instruction value (both reads and writes).
/// Used for consumer tracking where we need to know ALL uses of an identifier.
fn collect_operand_places(value: &InstructionValue) -> Vec<&crate::hir::types::Place> {
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
        InstructionValue::CallExpression { callee, args } => {
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
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            places.push(object);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            places.push(object);
            places.push(value);
        }
        InstructionValue::ComputedLoad { object, property } => {
            places.push(object);
            places.push(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            places.push(object);
            places.push(property);
            places.push(value);
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
        InstructionValue::Await { value }
        | InstructionValue::GetIterator { collection: value }
        | InstructionValue::NextPropertyOf { value }
        | InstructionValue::TypeCastExpression { value, .. } => {
            places.push(value);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            places.push(iterator);
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
