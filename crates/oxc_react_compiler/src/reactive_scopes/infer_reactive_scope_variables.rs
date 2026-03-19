use crate::hir::types::{
    DestructureArrayItem, DestructurePattern, DestructureTarget, HIR, IdentifierId, InstructionId,
    InstructionValue, MutableRange, ReactiveScope, ScopeId, SourceLocation, Type,
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
) -> Vec<ReactiveScope> {
    let param_id_set: FxHashSet<IdentifierId> = param_ids.iter().copied().collect();
    let mut dsu: DisjointSet<IdentifierId> = DisjointSet::new();
    let mut ranges: FxHashMap<IdentifierId, MutableRange> = FxHashMap::default();
    let mut is_reactive: FxHashMap<IdentifierId, bool> = FxHashMap::default();
    let mut is_allocating_id: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut is_mutable_id: FxHashSet<IdentifierId> = FxHashSet::default();

    // Build id-to-name map for resolving callee names through LoadGlobal/LoadLocal.
    // This is needed to determine if a CallExpression is a hook call.
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

    // Phase 1: Collect all identifiers and their mutable ranges
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let id = instr.lvalue.identifier.id;
            dsu.make_set(id);
            ranges.insert(id, instr.lvalue.identifier.mutable_range);
            is_reactive.insert(id, instr.lvalue.reactive);
            if is_allocating_instruction(
                &instr.value,
                &instr.lvalue.identifier.type_,
                &id_to_name,
                instr.lvalue.identifier.mutable_range,
            ) {
                is_allocating_id.insert(id);
            }
            if is_mutable_instruction(&instr.value) {
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

    // Phase 2: Union identifiers that should be in the same scope.
    //
    // Upstream uses `isMutable(instr, operand)` to check whether an operand is
    // still within its mutable range at the current instruction. An operand
    // whose mutable range has already ended is NOT unioned — it becomes a
    // dependency of the scope rather than a member.
    //
    // `isMutable(instr, place)` ≡ `instr.id >= range.start && instr.id < range.end`
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let lvalue_id = instr.lvalue.identifier.id;
            let lvalue_range = instr.lvalue.identifier.mutable_range;
            let instr_id = instr.id;

            // If the lvalue has a non-trivial mutable range OR the instruction
            // allocates, collect mutable operands and union them together.
            // This matches upstream: `range.end > range.start + 1 || mayAllocate(env, instr)`
            if lvalue_range.end.0 > lvalue_range.start.0 + 1
                || is_allocating_instruction(
                    &instr.value,
                    &instr.lvalue.identifier.type_,
                    &id_to_name,
                    instr.lvalue.identifier.mutable_range,
                )
            {
                let operand_ids = collect_operand_ids(&instr.value);
                for op_id in operand_ids {
                    if param_destructure_target_ids.contains(&op_id) {
                        continue;
                    }
                    // Upstream: `isMutable(instr, operand) && operand.identifier.mutableRange.start > 0`
                    // The `start > 0` check excludes globals (which have start = 0 in upstream).
                    // In our representation, globals typically have trivial ranges (start == end),
                    // so the isMutable check already excludes them.
                    if let Some(&op_range) = ranges.get(&op_id)
                        && instr_id.0 >= op_range.start.0
                        && instr_id.0 < op_range.end.0
                    {
                        // Both lvalue_id and op_id are registered via make_set in Phase 1
                        let _ = dsu.union(lvalue_id, op_id);
                    }
                }
            }
        }

        // Union phi operands when the phi's mutable range extends beyond its
        // definition (matching upstream's condition).
        for phi in &block.phis {
            let phi_id = phi.place.identifier.id;
            let phi_range = phi.place.identifier.mutable_range;
            // Upstream: phi range is non-trivial AND extends past the first instruction
            // of the block. We approximate with: range spans more than 1 instruction.
            if phi_range.end.0 > phi_range.start.0 + 1 {
                for (_, operand) in &phi.operands {
                    dsu.make_set(operand.identifier.id);
                    let _ = dsu.union(phi_id, operand.identifier.id);
                }
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
        let mut any_reactive = false;

        for &member in &members {
            if let Some(&range) = ranges.get(&member) {
                merged_range.start = InstructionId(merged_range.start.0.min(range.start.0));
                merged_range.end = InstructionId(merged_range.end.0.max(range.end.0));
            }
            if is_reactive.get(&member).copied().unwrap_or(false) {
                any_reactive = true;
            }
        }

        // Check if any member is an allocating instruction (for sentinel scopes)
        let any_allocating = members.iter().any(|m| is_allocating_id.contains(m));
        // Check if any member produces a mutable value (objects, arrays, calls, etc.)
        // Reactive-only sets (primitives, LoadLocal, arithmetic) don't need scopes
        // because those values are cheap to recompute and have no identity.
        let any_mutable = members.iter().any(|m| is_mutable_id.contains(m));

        // Create scope if:
        // - any_allocating (sentinel scope for identity memoization), OR
        // - any_reactive AND any_mutable (reactive computation producing a mutable value)
        // Pure-reactive but non-mutable sets (e.g., `return x` where x is a param)
        // don't get scopes — matches upstream's ValueKind.Mutable check.
        if (any_allocating || (any_reactive && any_mutable))
            && merged_range.end.0 > merged_range.start.0
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
            // Map all member identifiers to this scope index (cheap u64 copy, no clone)
            for &member in &members {
                id_to_scope_idx.insert(member, scope_idx);
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
    // Only here do we clone + box, once per identifier that needs a scope.
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(&idx) = id_to_scope_idx.get(&instr.lvalue.identifier.id) {
                instr.lvalue.identifier.scope = Some(Box::new(scopes[idx].clone()));
            }
        }
        for phi in &mut block.phis {
            if let Some(&idx) = id_to_scope_idx.get(&phi.place.identifier.id) {
                phi.place.identifier.scope = Some(Box::new(scopes[idx].clone()));
            }
        }
    }

    scopes
}

/// Returns true if an instruction value creates a new heap allocation.
/// Used for sentinel scope detection: allocating expressions get scopes
/// even without reactive deps (for identity memoization).
///
/// Matches upstream's `mayAllocate(env, instruction)` which checks:
/// 1. The instruction kind (objects, arrays, calls, etc.)
/// 2. The result type -- if the type is known to be primitive, the call
///    does NOT allocate (e.g., `Math.max(1, 2)` returns a number).
///
/// Hook calls (useXxx) are excluded: they execute every render and their
/// return values are reactive inputs, not allocating values.
fn is_allocating_instruction(
    value: &InstructionValue,
    lvalue_type: &Type,
    id_to_name: &FxHashMap<IdentifierId, String>,
    lvalue_range: MutableRange,
) -> bool {
    match value {
        InstructionValue::ObjectExpression { .. }
        | InstructionValue::ArrayExpression { .. }
        | InstructionValue::JsxExpression { .. }
        | InstructionValue::JsxFragment { .. }
        | InstructionValue::NewExpression { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. } => true,
        // Calls may allocate unless the return type is known to be primitive
        // or the callee is a hook (hooks are reactive inputs, not allocations).
        //
        // DIVERGENCE: Upstream's `mayAllocate` unconditionally returns true for
        // calls with non-primitive return types, and relies on downstream passes
        // (PropagateScopeDependenciesHIR, PruneNonEscapingScopes) to prune
        // unnecessary sentinel scopes. We lack those passes, so we apply a
        // heuristic: only treat a call as allocating if its result has a
        // non-trivial mutable range (end > start + 1), meaning the value is
        // used beyond the instruction that creates it. Calls whose results are
        // immediately consumed (trivial range) don't benefit from identity
        // memoization and shouldn't create sentinel scopes.
        InstructionValue::CallExpression { callee, .. } => {
            if matches!(lvalue_type, Type::Primitive(_)) {
                return false;
            }
            // Resolve callee name through LoadGlobal/LoadLocal for temporaries
            let name = callee
                .identifier
                .name
                .as_deref()
                .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));
            // Hook calls (useXxx) are reactive inputs, not allocations
            if name.is_some_and(|n| n.starts_with("use") && n.len() > 3) {
                return false;
            }
            // Only allocating if the result escapes (non-trivial mutable range)
            lvalue_range.end.0 > lvalue_range.start.0 + 1
        }
        // Method calls: check both return type and hook pattern (React.useXxx)
        InstructionValue::MethodCall { property, .. } => {
            if matches!(lvalue_type, Type::Primitive(_)) {
                return false;
            }
            // Hook methods like React.useState, React.useRef
            if property.starts_with("use") && property.len() > 3 {
                return false;
            }
            // Only allocating if the result escapes (non-trivial mutable range)
            lvalue_range.end.0 > lvalue_range.start.0 + 1
        }
        InstructionValue::TaggedTemplateExpression { .. } => {
            if matches!(lvalue_type, Type::Primitive(_)) {
                return false;
            }
            // Only allocating if the result escapes (non-trivial mutable range)
            lvalue_range.end.0 > lvalue_range.start.0 + 1
        }
        _ => false,
    }
}

/// Returns true if an instruction value produces a potentially mutable value
/// (one that benefits from memoization when reactive).
///
/// This is broader than `is_allocating_instruction`: it also includes
/// CallExpression and MethodCall, whose return types are unknown and
/// conservatively treated as mutable (they may return objects/arrays).
///
/// Used to gate scope creation: a reactive-only set (primitives, LoadLocal,
/// arithmetic) doesn't need a scope because those values are cheap to
/// recompute and have no identity. A set with a mutable value (call result,
/// object literal, etc.) DOES need a scope.
///
/// Matches upstream's `ValueKind.Mutable` in InferReactiveScopeVariables.ts.
fn is_mutable_instruction(value: &InstructionValue) -> bool {
    matches!(
        value,
        InstructionValue::ObjectExpression { .. }
            | InstructionValue::ArrayExpression { .. }
            | InstructionValue::JsxExpression { .. }
            | InstructionValue::JsxFragment { .. }
            | InstructionValue::NewExpression { .. }
            | InstructionValue::FunctionExpression { .. }
            | InstructionValue::ObjectMethod { .. }
            | InstructionValue::CallExpression { .. }
            | InstructionValue::MethodCall { .. }
    )
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
        InstructionValue::CallExpression { callee, args } => {
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
        InstructionValue::ComputedLoad { object, property } => {
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
