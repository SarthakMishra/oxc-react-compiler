use crate::hir::types::{
    HIR, IdentifierId, InstructionValue, ReactiveScopeDeclaration, ReactiveScopeDependency, ScopeId,
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

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadGlobal { .. }
                | InstructionValue::Primitive { .. }
                | InstructionValue::JSXText { .. } => {
                    non_reactive_ids.insert(instr.lvalue.identifier.id);
                }
                // When a non-reactive value is stored to a local variable, the variable
                // name itself becomes non-reactive.
                InstructionValue::StoreLocal { lvalue, value, .. } => {
                    if non_reactive_ids.contains(&value.identifier.id) {
                        non_reactive_ids.insert(lvalue.identifier.id);
                        if let Some(name) = &lvalue.identifier.name {
                            non_reactive_names.insert(name.clone());
                        }
                    }
                }
                // LoadLocal of a non-reactive name produces a non-reactive value.
                InstructionValue::LoadLocal { place } => {
                    if let Some(name) = &place.identifier.name
                        && non_reactive_names.contains(name) {
                            non_reactive_ids.insert(instr.lvalue.identifier.id);
                        }
                }
                _ => {}
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

    // Phase 2: For each instruction in a scope, find operands from outside the scope
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
                    && written_names.is_some_and(|s| s.contains(name)) {
                        return true;
                    }
                false
            };

            // Collect READ operands — only values that are consumed (not written to)
            let operands = collect_read_operand_places(&instr.value);
            for place in operands {
                let op_id = place.identifier.id;
                // If this operand is not declared/written within the same scope, and not a global, it's a dependency
                if !is_scope_internal(place) && !non_reactive_ids.contains(&place.identifier.id) {
                    // Check if already added
                    let deps = scope_deps.entry(scope_id).or_default();
                    let already_added = deps.iter().any(|d| d.identifier.id == op_id);
                    if !already_added {
                        deps.push(ReactiveScopeDependency {
                            identifier: place.identifier.clone(),
                            reactive: place.reactive,
                            path: Vec::new(),
                        });
                    }
                }
            }

            // Handle property loads: build dependency paths
            if let InstructionValue::PropertyLoad { object, property } = &instr.value {
                let obj_id = object.identifier.id;
                if !is_scope_internal(object) && !non_reactive_ids.contains(&obj_id) {
                    let deps = scope_deps.entry(scope_id).or_default();
                    // Check if we already have a dep for this object and can extend its path
                    let existing = deps.iter_mut().find(|d| d.identifier.id == obj_id);
                    if let Some(dep) = existing {
                        dep.path.push(crate::hir::types::DependencyPathEntry {
                            property: property.clone(),
                            optional: false,
                        });
                    }
                }
            }
        }
    }

    // Phase 3: Determine declarations (identifiers defined in scope, used outside)
    // Build a reverse-use map: operand_id -> set of consumer scope IDs (or None if outside scope)
    let mut operand_consumers: FxHashMap<IdentifierId, Vec<Option<ScopeId>>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let consumer_scope = instr.lvalue.identifier.scope.as_ref().map(|s| s.id);
            let operands = collect_operand_places(&instr.value);
            for place in operands {
                operand_consumers.entry(place.identifier.id).or_default().push(consumer_scope);
            }
        }
        // Terminal uses are always "outside" any scope (scope = None)
        match &block.terminal {
            crate::hir::types::Terminal::Return { value }
            | crate::hir::types::Terminal::Throw { value } => {
                operand_consumers.entry(value.identifier.id).or_default().push(None);
            }
            crate::hir::types::Terminal::If { test, .. }
            | crate::hir::types::Terminal::Branch { test, .. } => {
                operand_consumers.entry(test.identifier.id).or_default().push(None);
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
