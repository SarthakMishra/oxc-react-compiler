use crate::hir::types::{
    DeclarationId, HIR, IdentifierId, InstructionValue, ReactiveScopeDeclaration,
    ReactiveScopeDependency, ScopeId, Type,
};
use rustc_hash::{FxHashMap, FxHashSet};

/// Propagate scope dependencies through the HIR.
///
/// For each reactive scope, determine which external values it depends on.
/// These become the "deps" that are checked at runtime to decide whether
/// to recompute the scope's output.
pub fn propagate_scope_dependencies_hir(hir: &mut HIR, param_names: &[String]) {
    // Phase 0: Collect identifiers that should NOT be scope dependencies:
    // - Global values (from LoadGlobal) — never change between renders
    // - Primitive constants (from Primitive/JSXText) — immutable by definition
    // - Free variables (not defined in the function) — module-scope imports/constants
    let mut non_reactive_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    // Map from identifier name to whether it's known to be non-reactive.
    // Used to propagate non-reactivity through StoreLocal/LoadLocal chains.
    let mut non_reactive_names: FxHashSet<String> = FxHashSet::default();

    // Collect all names that are locally defined in the function body.
    // A name is "locally defined" if it appears as the target of a StoreLocal,
    // DeclareLocal, or Destructure instruction, or is a function parameter.
    // Names NOT in this set are free variables (module-scope imports, globals
    // not in is_global_name) that never change between renders → non-reactive.
    let mut locally_defined_names: FxHashSet<String> = param_names.iter().cloned().collect();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::StoreContext { lvalue, .. }
                | InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name {
                        locally_defined_names.insert(name.clone());
                    }
                }
                InstructionValue::Destructure { lvalue_pattern, .. } => {
                    collect_destructure_names(lvalue_pattern, &mut locally_defined_names);
                }
                _ => {}
            }
        }
    }

    // First pass: seed non-reactive IDs from globals, primitives, stable types,
    // and free variables (names loaded but never locally defined).
    // Also check ALL named operand places (including CallExpression callees,
    // MethodCall receivers/args, etc.) for free variable detection. After the
    // LoadLocal inline pass + DCE, LoadLocal instructions for free variables
    // may be eliminated, leaving only their named references in consumers.
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
                // Free variable detection: LoadLocal/LoadContext for a name that is
                // never written to (not in locally_defined_names) is a free variable —
                // a module-scope import or constant that never changes between renders.
                // DIVERGENCE: Upstream doesn't need this because its scope chain
                // correctly identifies free variables during BuildHIR. Our HIR builder
                // emits LoadLocal for all non-global identifier references, including
                // imports and module-scope values, since it lacks OXC scoping data.
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name
                        && !locally_defined_names.contains(name)
                    {
                        non_reactive_ids.insert(instr.lvalue.identifier.id);
                        non_reactive_ids.insert(place.identifier.id);
                        non_reactive_names.insert(name.clone());
                    }
                    // Still check for stable types
                    if matches!(instr.lvalue.identifier.type_, Type::SetState | Type::Ref) {
                        non_reactive_ids.insert(instr.lvalue.identifier.id);
                        non_reactive_ids.insert(place.identifier.id);
                        if let Some(name) = &instr.lvalue.identifier.name {
                            non_reactive_names.insert(name.clone());
                        }
                    }
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

    // Additional free variable detection: after the LoadLocal inline + DCE,
    // some free variables no longer have LoadLocal instructions (the inline
    // substituted their temps, then DCE removed the LoadLocal). Check ALL
    // named operand places in ALL instructions for names that aren't locally
    // defined — these are free variables that should be non-reactive.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let operands = collect_read_operand_places(&instr.value);
            for place in operands {
                if let Some(name) = &place.identifier.name
                    && !locally_defined_names.contains(name.as_str())
                    && !non_reactive_ids.contains(&place.identifier.id)
                {
                    non_reactive_ids.insert(place.identifier.id);
                    non_reactive_names.insert(name.clone());
                }
            }
        }
    }

    // Build a map from identifier ID to name for hook detection in CallExpression.
    // When a LoadLocal loads a named variable (like `useState`), the result goes
    // to an unnamed temp. We need to trace back from the callee's ID to the
    // original variable name to check if it's a hook call.
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name.clone());
                    }
                }
                InstructionValue::LoadGlobal { binding } => {
                    id_to_name.insert(instr.lvalue.identifier.id, binding.name.clone());
                }
                _ => {}
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
                            || value
                                .identifier
                                .name
                                .as_deref()
                                .is_some_and(|n| non_reactive_names.contains(n))
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
                    // results as non-reactive when callee and all args are non-reactive,
                    // BUT only if the callee is not a hook (name starting with "use").
                    // This primarily handles `require('shared-runtime')` returning a
                    // stable module object. Hook calls are excluded because their return
                    // values (state, refs, etc.) are reactive even when the hook itself
                    // is a non-reactive import — e.g., useState(0) returns reactive state.
                    InstructionValue::CallExpression { callee, args } => {
                        let is_hook = id_to_name.get(&callee.identifier.id).is_some_and(|n| {
                            n.starts_with("use")
                                && n.len() > 3
                                && n.as_bytes()[3].is_ascii_uppercase()
                        });
                        !is_hook
                            && non_reactive_ids.contains(&callee.identifier.id)
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
    let mut scope_written_decl_ids: FxHashMap<ScopeId, FxHashSet<DeclarationId>> =
        FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                scope_ids.entry(scope.id).or_default().insert(instr.lvalue.identifier.id);
                // Track names of variables written to by store instructions
                match &instr.value {
                    InstructionValue::StoreLocal { lvalue, .. }
                    | InstructionValue::StoreContext { lvalue, .. } => {
                        scope_ids.entry(scope.id).or_default().insert(lvalue.identifier.id);
                        if let Some(decl_id) = lvalue.identifier.declaration_id {
                            scope_written_decl_ids.entry(scope.id).or_default().insert(decl_id);
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
        root_declaration_id: Option<DeclarationId>,
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
                ssa_version: 0,
                declaration_id: self.root_declaration_id,
                name: self.root_name.clone(),
                mutable_range: crate::hir::types::MutableRange {
                    start: crate::hir::types::InstructionId(0),
                    end: crate::hir::types::InstructionId(0),
                },
                last_use: crate::hir::types::InstructionId(0),
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
                                root_declaration_id: resolved.root_declaration_id,
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
                                root_declaration_id: place.identifier.declaration_id,
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
                                root_declaration_id: resolved.root_declaration_id,
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
                                root_declaration_id: object.identifier.declaration_id,
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
                                    root_declaration_id: resolved.root_declaration_id,
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
            let written_decl_ids = scope_written_decl_ids.get(&scope_id);

            // Check if an operand belongs to this scope by:
            // 1. Exact SSA IdentifierId match (instruction lvalues + StoreLocal targets), or
            // 2. DeclarationId match against variables written inside the scope (handles SSA
            //    versioning where LoadLocal and StoreLocal have different IdentifierIds but
            //    share the same DeclarationId for the same source variable)
            let is_scope_internal = |place: &crate::hir::types::Place| -> bool {
                if declared_ids.is_some_and(|s| s.contains(&place.identifier.id)) {
                    return true;
                }
                if let Some(decl_id) = place.identifier.declaration_id
                    && written_decl_ids.is_some_and(|s| s.contains(&decl_id))
                {
                    return true;
                }
                false
            };

            // Check if a resolved root identifier is scope-internal by DeclarationId
            let is_root_scope_internal = |identifier: &crate::hir::types::Identifier| -> bool {
                if declared_ids.is_some_and(|s| s.contains(&identifier.id)) {
                    return true;
                }
                if let Some(decl_id) = identifier.declaration_id
                    && written_decl_ids.is_some_and(|s| s.contains(&decl_id))
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
                    if !is_root_scope_internal(&root)
                        && !non_reactive_ids.contains(&root.id)
                        && !root.name.as_deref().is_some_and(|n| non_reactive_names.contains(n))
                    {
                        let deps = scope_deps.entry(scope_id).or_default();
                        // Dedup by DeclarationId+path (or IdentifierId fallback for unnamed temps)
                        let already = deps.iter().any(|d| {
                            d.path == resolved.path
                                && match (d.identifier.declaration_id, resolved.root_declaration_id)
                                {
                                    (Some(a), Some(b)) => a == b,
                                    _ => d.identifier.id == root.id,
                                }
                        });
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
                    if !is_scope_internal(place)
                        && !non_reactive_ids.contains(&place.identifier.id)
                        && !place
                            .identifier
                            .name
                            .as_deref()
                            .is_some_and(|n| non_reactive_names.contains(n))
                    {
                        let deps = scope_deps.entry(scope_id).or_default();
                        let already_added = deps.iter().any(|d| {
                            d.path.is_empty()
                                && match (
                                    d.identifier.declaration_id,
                                    place.identifier.declaration_id,
                                ) {
                                    (Some(a), Some(b)) => a == b,
                                    _ => d.identifier.id == place.identifier.id,
                                }
                        });
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
    //   operand IdentifierId → consumer scope IDs (or None if outside scope)
    //   operand DeclarationId → consumer scope IDs (for cross-SSA-ID matching)
    // Match across SSA versions using both IdentifierId (for exact SSA match) and
    // DeclarationId (for cross-SSA-ID matching of the same source variable).
    let mut operand_consumers: FxHashMap<IdentifierId, Vec<Option<ScopeId>>> = FxHashMap::default();
    let mut decl_id_consumers: FxHashMap<DeclarationId, Vec<Option<ScopeId>>> =
        FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let consumer_scope = instr.lvalue.identifier.scope.as_ref().map(|s| s.id);
            let operands = collect_operand_places(&instr.value);
            for place in operands {
                operand_consumers.entry(place.identifier.id).or_default().push(consumer_scope);
                if let Some(decl_id) = place.identifier.declaration_id {
                    decl_id_consumers.entry(decl_id).or_default().push(consumer_scope);
                }
            }
        }
        // Terminal uses are always "outside" any scope (scope = None)
        match &block.terminal {
            crate::hir::types::Terminal::Return { value }
            | crate::hir::types::Terminal::Throw { value } => {
                operand_consumers.entry(value.identifier.id).or_default().push(None);
                if let Some(decl_id) = value.identifier.declaration_id {
                    decl_id_consumers.entry(decl_id).or_default().push(None);
                }
            }
            crate::hir::types::Terminal::If { test, .. }
            | crate::hir::types::Terminal::Branch { test, .. } => {
                operand_consumers.entry(test.identifier.id).or_default().push(None);
                if let Some(decl_id) = test.identifier.declaration_id {
                    decl_id_consumers.entry(decl_id).or_default().push(None);
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
                        let target_used_outside =
                            operand_consumers.get(&target_id).is_some_and(|consumers| {
                                consumers
                                    .iter()
                                    .any(|consumer_scope| *consumer_scope != Some(scope.id))
                            }) || lvalue.identifier.declaration_id.is_some_and(|decl_id| {
                                decl_id_consumers.get(&decl_id).is_some_and(|consumers| {
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

    // Phase 3b: Catch "unscooped but enclosed" variables.
    //
    // Some instructions within a scope's block range don't have a scope assigned
    // (e.g., primitive computations like `remaining = users.length - max`). These
    // variables don't get scopes during InferReactiveScopeVariables because they
    // are non-mutable primitives. However, they end up inside the scope's body in
    // the ReactiveFunction IR because AlignScopesToBlockBoundaries places them
    // within the scope's block range. If such a variable is used outside the scope
    // (e.g., as a dependency of another scope), it must be declared as a scope
    // output so it gets hoisted to function level and cached.
    //
    // DIVERGENCE: Upstream handles this via its HIR→ReactiveIR conversion which
    // inserts scope declarations for all variables computed within a scope body
    // that are referenced outside. Our HIR-level Phase 3 only checks instructions
    // with scope assignments, missing primitives. This extra pass compensates.
    //
    // We use the Scope terminals in the HIR to build a block→scope map, then
    // determine which scope encloses each unscooped instruction.
    {
        use crate::hir::types::{BlockId, Terminal};

        // Build block→scope map by walking Scope terminals.
        // Inner scopes override outer scopes (entry().or_insert avoids overwriting).
        let mut block_to_scope: FxHashMap<BlockId, ScopeId> = FxHashMap::default();

        fn collect_scope_blocks(
            hir: &HIR,
            start: BlockId,
            stop: BlockId,
            scope_id: ScopeId,
            block_to_scope: &mut FxHashMap<BlockId, ScopeId>,
            visited: &mut FxHashSet<BlockId>,
        ) {
            if start == stop || !visited.insert(start) {
                return;
            }
            // Only insert if not already claimed by a tighter (inner) scope
            block_to_scope.entry(start).or_insert(scope_id);

            let block = match hir.blocks.iter().find(|(id, _)| *id == start).map(|(_, b)| b) {
                Some(b) => b,
                None => return,
            };

            match &block.terminal {
                Terminal::Scope {
                    block: inner_block,
                    fallthrough: inner_ft,
                    scope: inner_scope,
                } => {
                    // Inner scope blocks belong to the inner scope, not us
                    collect_scope_blocks(
                        hir,
                        *inner_block,
                        *inner_ft,
                        *inner_scope,
                        block_to_scope,
                        visited,
                    );
                    // Continue with the inner scope's fallthrough (still in our scope)
                    collect_scope_blocks(hir, *inner_ft, stop, scope_id, block_to_scope, visited);
                }
                Terminal::Goto { block: next } => {
                    collect_scope_blocks(hir, *next, stop, scope_id, block_to_scope, visited);
                }
                Terminal::If { consequent, alternate, fallthrough, .. }
                | Terminal::Ternary { consequent, alternate, fallthrough, .. } => {
                    collect_scope_blocks(
                        hir,
                        *consequent,
                        *fallthrough,
                        scope_id,
                        block_to_scope,
                        visited,
                    );
                    collect_scope_blocks(
                        hir,
                        *alternate,
                        *fallthrough,
                        scope_id,
                        block_to_scope,
                        visited,
                    );
                    collect_scope_blocks(
                        hir,
                        *fallthrough,
                        stop,
                        scope_id,
                        block_to_scope,
                        visited,
                    );
                }
                Terminal::Logical { right, fallthrough, .. }
                | Terminal::Optional { consequent: right, fallthrough, .. } => {
                    collect_scope_blocks(
                        hir,
                        *right,
                        *fallthrough,
                        scope_id,
                        block_to_scope,
                        visited,
                    );
                    collect_scope_blocks(
                        hir,
                        *fallthrough,
                        stop,
                        scope_id,
                        block_to_scope,
                        visited,
                    );
                }
                Terminal::Sequence { blocks, fallthrough } => {
                    for bid in blocks {
                        collect_scope_blocks(
                            hir,
                            *bid,
                            *fallthrough,
                            scope_id,
                            block_to_scope,
                            visited,
                        );
                    }
                    collect_scope_blocks(
                        hir,
                        *fallthrough,
                        stop,
                        scope_id,
                        block_to_scope,
                        visited,
                    );
                }
                Terminal::MaybeThrow { continuation, .. } => {
                    collect_scope_blocks(
                        hir,
                        *continuation,
                        stop,
                        scope_id,
                        block_to_scope,
                        visited,
                    );
                }
                Terminal::PrunedScope { fallthrough, .. } => {
                    collect_scope_blocks(
                        hir,
                        *fallthrough,
                        stop,
                        scope_id,
                        block_to_scope,
                        visited,
                    );
                }
                _ => {
                    // Return, Throw, Switch, loops, etc. — stop recursing
                }
            }
        }

        // Walk all blocks to find Scope terminals
        for (_, block) in &hir.blocks {
            if let Terminal::Scope { scope, block: scope_block, fallthrough } = &block.terminal {
                let mut visited = FxHashSet::default();
                collect_scope_blocks(
                    hir,
                    *scope_block,
                    *fallthrough,
                    *scope,
                    &mut block_to_scope,
                    &mut visited,
                );
            }
        }

        // For unscooped instructions, check if their block is inside a scope
        for (block_id, block) in &hir.blocks {
            let enclosing_scope_id = match block_to_scope.get(block_id) {
                Some(sid) => *sid,
                None => continue,
            };

            for instr in &block.instructions {
                if instr.lvalue.identifier.scope.is_some() {
                    continue; // Already handled in Phase 3
                }

                // Check StoreLocal/StoreContext targets and Destructure bindings
                match &instr.value {
                    InstructionValue::StoreLocal { lvalue, .. }
                    | InstructionValue::StoreContext { lvalue, .. } => {
                        let target_id = lvalue.identifier.id;

                        // Check if the target is used outside this scope
                        let target_used_outside =
                            operand_consumers.get(&target_id).is_some_and(|consumers| {
                                consumers.iter().any(|cs| *cs != Some(enclosing_scope_id))
                            }) || lvalue.identifier.declaration_id.is_some_and(|decl_id| {
                                decl_id_consumers.get(&decl_id).is_some_and(|consumers| {
                                    consumers.iter().any(|cs| *cs != Some(enclosing_scope_id))
                                })
                            });

                        if target_used_outside {
                            let decls = scope_decls.entry(enclosing_scope_id).or_default();
                            if !decls.iter().any(|(did, _)| *did == target_id) {
                                decls.push((
                                    target_id,
                                    ReactiveScopeDeclaration {
                                        identifier: lvalue.identifier.clone(),
                                        scope: enclosing_scope_id,
                                    },
                                ));
                            }
                        }
                    }
                    InstructionValue::Destructure { lvalue_pattern, .. } => {
                        // Check each destructured binding
                        let places = collect_destructure_places(lvalue_pattern);
                        for place in &places {
                            let target_id = place.identifier.id;
                            let target_used_outside =
                                operand_consumers.get(&target_id).is_some_and(|consumers| {
                                    consumers.iter().any(|cs| *cs != Some(enclosing_scope_id))
                                }) || place.identifier.declaration_id.is_some_and(|decl_id| {
                                    decl_id_consumers.get(&decl_id).is_some_and(|consumers| {
                                        consumers.iter().any(|cs| *cs != Some(enclosing_scope_id))
                                    })
                                });

                            if target_used_outside {
                                let decls = scope_decls.entry(enclosing_scope_id).or_default();
                                if !decls.iter().any(|(did, _)| *did == target_id) {
                                    decls.push((
                                        target_id,
                                        ReactiveScopeDeclaration {
                                            identifier: place.identifier.clone(),
                                            scope: enclosing_scope_id,
                                        },
                                    ));
                                }
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
        // Map: DeclarationId → (declaring_scope_id, that scope's deps)
        let mut decl_deps_map: FxHashMap<DeclarationId, (ScopeId, Vec<ReactiveScopeDependency>)> =
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
                        if let Some(decl_id) = decl.identifier.declaration_id {
                            decl_deps_map.insert(decl_id, (*scope_id, deps.clone()));
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

                // Helper: check if a dep with the same identity+path is already in new_deps
                let has_dep = |new_deps: &[ReactiveScopeDependency],
                               dep: &ReactiveScopeDependency| {
                    new_deps.iter().any(|d| {
                        d.path == dep.path
                            && match (d.identifier.declaration_id, dep.identifier.declaration_id) {
                                (Some(a), Some(b)) => a == b,
                                _ => d.identifier.name.as_ref() == dep.identifier.name.as_ref(),
                            }
                    })
                };

                for dep in &deps {
                    if dep.path.is_empty()
                        && let Some(dep_decl_id) = dep.identifier.declaration_id
                        && let Some((declaring_scope, root_deps)) = decl_deps_map.get(&dep_decl_id)
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
            a_name
                .cmp(b_name)
                .then_with(|| a.identifier.declaration_id.cmp(&b.identifier.declaration_id))
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

/// Collect all named targets from a destructure pattern into a set of names.
///
/// Used for free variable detection: names defined by destructuring are
/// locally-defined and should not be treated as non-reactive free variables.
fn collect_destructure_names(
    pattern: &crate::hir::types::DestructurePattern,
    names: &mut FxHashSet<String>,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};

    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                match &prop.value {
                    DestructureTarget::Place(place) => {
                        if let Some(name) = &place.identifier.name {
                            names.insert(name.clone());
                        }
                    }
                    DestructureTarget::Pattern(nested) => {
                        collect_destructure_names(nested, names);
                    }
                }
            }
            if let Some(rest_place) = rest
                && let Some(name) = &rest_place.identifier.name
            {
                names.insert(name.clone());
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(DestructureTarget::Place(place)) => {
                        if let Some(name) = &place.identifier.name {
                            names.insert(name.clone());
                        }
                    }
                    DestructureArrayItem::Value(DestructureTarget::Pattern(nested)) => {
                        collect_destructure_names(nested, names);
                    }
                    DestructureArrayItem::Spread(place) => {
                        if let Some(name) = &place.identifier.name {
                            names.insert(name.clone());
                        }
                    }
                    DestructureArrayItem::Hole => {}
                }
            }
            if let Some(rest_place) = rest
                && let Some(name) = &rest_place.identifier.name
            {
                names.insert(name.clone());
            }
        }
    }
}

/// Collect all Place references from a DestructurePattern.
/// Used by Phase 3b to find destructured bindings that may need to be scope declarations.
fn collect_destructure_places(
    pattern: &crate::hir::types::DestructurePattern,
) -> Vec<crate::hir::types::Place> {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};

    let mut places = Vec::new();

    fn collect(pattern: &DestructurePattern, places: &mut Vec<crate::hir::types::Place>) {
        match pattern {
            DestructurePattern::Object { properties, rest } => {
                for prop in properties {
                    match &prop.value {
                        DestructureTarget::Place(place) => places.push(place.clone()),
                        DestructureTarget::Pattern(nested) => collect(nested, places),
                    }
                }
                if let Some(rest_place) = rest {
                    places.push(rest_place.clone());
                }
            }
            DestructurePattern::Array { items, rest } => {
                for item in items {
                    match item {
                        DestructureArrayItem::Value(DestructureTarget::Place(place)) => {
                            places.push(place.clone());
                        }
                        DestructureArrayItem::Value(DestructureTarget::Pattern(nested)) => {
                            collect(nested, places);
                        }
                        DestructureArrayItem::Spread(place) => places.push(place.clone()),
                        DestructureArrayItem::Hole => {}
                    }
                }
                if let Some(rest_place) = rest {
                    places.push(rest_place.clone());
                }
            }
        }
    }

    collect(pattern, &mut places);
    places
}
