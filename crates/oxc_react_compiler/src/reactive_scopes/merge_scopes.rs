use crate::hir::types::{
    AliasingEffect, ArrayElement, DependencyPathEntry, DestructurePattern, DestructureTarget, HIR,
    IdentifierId, Instruction, InstructionId, InstructionKind, InstructionValue, ObjectPropertyKey,
    Place, ReactiveBlock, ReactiveFunction, ReactiveScope, ReactiveTerminal, ScopeId,
};
use crate::utils::disjoint_set::DisjointSet;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Sub-task 4d: Safety checks for intermediate instructions between scopes
// ---------------------------------------------------------------------------

/// Map from IdentifierId to the last instruction ID at which it was read.
/// Matches upstream `FindLastUsageVisitor` from
/// `MergeReactiveScopesThatInvalidateTogether.ts`.
type LastUsageMap = FxHashMap<IdentifierId, u32>;

/// Build a map of last-usage instruction IDs for all identifiers in the
/// reactive function tree. For each identifier, records the maximum
/// instruction ID at which it appears as a read operand.
fn build_last_usage_map(reactive_fn: &ReactiveFunction) -> LastUsageMap {
    let mut map: LastUsageMap = FxHashMap::default();
    collectlast_usage_in_block(&reactive_fn.body, &mut map);
    map
}

fn collectlast_usage_in_block(block: &ReactiveBlock, map: &mut LastUsageMap) {
    for instr in &block.instructions {
        match instr {
            crate::hir::types::ReactiveInstruction::Instruction(instr) => {
                visit_instruction_read_places(&instr.value, instr.id.0, map);
            }
            crate::hir::types::ReactiveInstruction::Scope(scope_block) => {
                collectlast_usage_in_block(&scope_block.instructions, map);
            }
            crate::hir::types::ReactiveInstruction::Terminal(terminal) => {
                collectlast_usage_in_terminal(terminal, map);
            }
        }
    }
}

fn collectlast_usage_in_terminal(terminal: &ReactiveTerminal, map: &mut LastUsageMap) {
    match terminal {
        ReactiveTerminal::If { test, consequent, alternate, .. } => {
            // test is a read place — use u32::MAX since terminals don't have explicit IDs
            record_place_usage(test, u32::MAX, map);
            collectlast_usage_in_block(consequent, map);
            collectlast_usage_in_block(alternate, map);
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            collectlast_usage_in_block(init, map);
            collectlast_usage_in_block(test, map);
            if let Some(upd) = update {
                collectlast_usage_in_block(upd, map);
            }
            collectlast_usage_in_block(body, map);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            collectlast_usage_in_block(init, map);
            collectlast_usage_in_block(test, map);
            collectlast_usage_in_block(body, map);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            collectlast_usage_in_block(test, map);
            collectlast_usage_in_block(body, map);
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            record_place_usage(test, u32::MAX, map);
            for (case_test, block) in cases {
                if let Some(ct) = case_test {
                    record_place_usage(ct, u32::MAX, map);
                }
                collectlast_usage_in_block(block, map);
            }
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            collectlast_usage_in_block(block, map);
            collectlast_usage_in_block(handler, map);
        }
        ReactiveTerminal::Label { block, .. } => {
            collectlast_usage_in_block(block, map);
        }
        ReactiveTerminal::Return { value, .. } => {
            record_place_usage(value, u32::MAX, map);
        }
        ReactiveTerminal::Throw { value, .. } => {
            record_place_usage(value, u32::MAX, map);
        }
    }
}

/// Record that `place` is read at `instr_id`, updating the max.
fn record_place_usage(place: &Place, instr_id: u32, map: &mut LastUsageMap) {
    map.entry(place.identifier.id).and_modify(|v| *v = (*v).max(instr_id)).or_insert(instr_id);
}

/// Visit all read-operand Places in an InstructionValue and record their usage.
fn visit_instruction_read_places(value: &InstructionValue, instr_id: u32, map: &mut LastUsageMap) {
    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            record_place_usage(place, instr_id, map);
        }
        InstructionValue::StoreLocal { value, .. }
        | InstructionValue::StoreContext { value, .. } => {
            record_place_usage(value, instr_id, map);
        }
        InstructionValue::Destructure { value, lvalue_pattern } => {
            record_place_usage(value, instr_id, map);
            visit_destructure_pattern_reads(lvalue_pattern, instr_id, map);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            record_place_usage(left, instr_id, map);
            record_place_usage(right, instr_id, map);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            record_place_usage(value, instr_id, map);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            record_place_usage(lvalue, instr_id, map);
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                record_place_usage(sub, instr_id, map);
            }
        }
        InstructionValue::CallExpression { callee, args } => {
            record_place_usage(callee, instr_id, map);
            for arg in args {
                record_place_usage(arg, instr_id, map);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            record_place_usage(receiver, instr_id, map);
            for arg in args {
                record_place_usage(arg, instr_id, map);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            record_place_usage(callee, instr_id, map);
            for arg in args {
                record_place_usage(arg, instr_id, map);
            }
        }
        InstructionValue::PropertyLoad { object, .. } => {
            record_place_usage(object, instr_id, map);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            record_place_usage(object, instr_id, map);
            record_place_usage(value, instr_id, map);
        }
        InstructionValue::ComputedLoad { object, property } => {
            record_place_usage(object, instr_id, map);
            record_place_usage(property, instr_id, map);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            record_place_usage(object, instr_id, map);
            record_place_usage(property, instr_id, map);
            record_place_usage(value, instr_id, map);
        }
        InstructionValue::PropertyDelete { object, .. } => {
            record_place_usage(object, instr_id, map);
        }
        InstructionValue::ComputedDelete { object, property } => {
            record_place_usage(object, instr_id, map);
            record_place_usage(property, instr_id, map);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                record_place_usage(&prop.value, instr_id, map);
                if let ObjectPropertyKey::Computed(key_place) = &prop.key {
                    record_place_usage(key_place, instr_id, map);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    ArrayElement::Expression(p) | ArrayElement::Spread(p) => {
                        record_place_usage(p, instr_id, map);
                    }
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            record_place_usage(tag, instr_id, map);
            for attr in props {
                record_place_usage(&attr.value, instr_id, map);
            }
            for child in children {
                record_place_usage(child, instr_id, map);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                record_place_usage(child, instr_id, map);
            }
        }
        InstructionValue::TypeCastExpression { value, .. }
        | InstructionValue::Await { value }
        | InstructionValue::NextPropertyOf { value } => {
            record_place_usage(value, instr_id, map);
        }
        InstructionValue::GetIterator { collection } => {
            record_place_usage(collection, instr_id, map);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            record_place_usage(iterator, instr_id, map);
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            record_place_usage(tag, instr_id, map);
            for sub in &value.subexpressions {
                record_place_usage(sub, instr_id, map);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => {
            record_place_usage(value, instr_id, map);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            record_place_usage(decl, instr_id, map);
            for dep in deps {
                record_place_usage(dep, instr_id, map);
            }
        }
        // These have no read operands
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::DeclareLocal { .. }
        | InstructionValue::DeclareContext { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

/// Visit read Places inside a destructure pattern (for last-usage tracking).
fn visit_destructure_pattern_reads(
    pattern: &DestructurePattern,
    instr_id: u32,
    map: &mut LastUsageMap,
) {
    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                if let DestructureTarget::Pattern(nested) = &prop.value {
                    visit_destructure_pattern_reads(nested, instr_id, map);
                }
                // Place targets are writes, not reads
            }
            // rest is a write target, not recorded as a read
            let _ = rest;
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    crate::hir::types::DestructureArrayItem::Value(DestructureTarget::Pattern(
                        nested,
                    )) => {
                        visit_destructure_pattern_reads(nested, instr_id, map);
                    }
                    _ => {} // write targets or holes
                }
            }
            let _ = rest; // write target, not recorded as a read
        }
    }
}

/// Returns whether an instruction value is "simple" and can be absorbed
/// into a merged reactive scope without changing observable behavior.
/// Matches the upstream allowlist in `MergeReactiveScopesThatInvalidateTogether.ts`.
fn is_simple_instruction(value: &InstructionValue) -> bool {
    matches!(
        value,
        InstructionValue::BinaryExpression { .. }
            | InstructionValue::ComputedLoad { .. }
            | InstructionValue::JSXText { .. }
            | InstructionValue::LoadGlobal { .. }
            | InstructionValue::LoadLocal { .. }
            | InstructionValue::Primitive { .. }
            | InstructionValue::PropertyLoad { .. }
            | InstructionValue::TemplateLiteral { .. }
            | InstructionValue::UnaryExpression { .. }
    )
}

/// Returns true if this is a `StoreLocal` with `Const` binding kind (safe to absorb).
/// `Reassign` and other kinds are side-effecting and must reset the merge candidate.
fn is_const_store_local(value: &InstructionValue) -> bool {
    matches!(value, InstructionValue::StoreLocal { type_: Some(InstructionKind::Const), .. })
}

/// Tracks intermediate instructions between two reactive scope candidates.
/// Accumulates the lvalues produced by those instructions and a map of
/// LoadLocal aliases (for the output-to-input chain check in Sub-task 4b).
struct IntermediateAccumulator {
    /// lvalue IdentifierIds produced by intermediate instructions.
    /// Used by `are_lvalues_last_used_by_scope` to verify no escape.
    lvalues: FxHashSet<IdentifierId>,
    /// Alias map: intermediate LoadLocal lvalue.id -> source place.id.
    /// Used by `can_merge_scopes` for the output-to-input check (Sub-task 4b).
    temporaries: FxHashMap<IdentifierId, IdentifierId>,
}

impl IntermediateAccumulator {
    fn new() -> Self {
        Self { lvalues: FxHashSet::default(), temporaries: FxHashMap::default() }
    }

    fn clear(&mut self) {
        self.lvalues.clear();
        self.temporaries.clear();
    }
}

/// Attempts to absorb an intermediate instruction into the accumulator.
/// Returns `true` if the instruction is safe (accumulator updated),
/// `false` if it resets the merge candidate (caller must call `clear()`).
fn accumulate_intermediate_instruction(
    instr: &Instruction,
    acc: &mut IntermediateAccumulator,
) -> bool {
    if is_simple_instruction(&instr.value) {
        acc.lvalues.insert(instr.lvalue.identifier.id);
        // Track LoadLocal alias for output-to-input chain detection (Sub-task 4b)
        if let InstructionValue::LoadLocal { place } = &instr.value {
            acc.temporaries.insert(instr.lvalue.identifier.id, place.identifier.id);
        }
        true
    } else if is_const_store_local(&instr.value) {
        acc.lvalues.insert(instr.lvalue.identifier.id);
        if let InstructionValue::StoreLocal { lvalue: store_lvalue, value: store_value, .. } =
            &instr.value
        {
            // Chain alias: StoreLocal(Const) x = y → x aliases whatever y aliases.
            // If y has no prior alias in temporaries, use y's own id (it's the root).
            let aliased = acc
                .temporaries
                .get(&store_value.identifier.id)
                .copied()
                .unwrap_or(store_value.identifier.id);
            acc.temporaries.insert(store_lvalue.identifier.id, aliased);
        }
        true
    } else {
        false
    }
}

/// Returns true if all intermediate lvalues are last-used strictly before
/// the end of `scope` (i.e., they do not escape beyond the merged boundary).
/// Corresponds to `areLValuesLastUsedByScope` in the upstream TypeScript.
fn are_lvalues_last_used_by_scope(
    scope: &ReactiveScope,
    lvalues: &FxHashSet<IdentifierId>,
    last_usage: &LastUsageMap,
) -> bool {
    let scope_end = scope.range.end.0;
    for &id in lvalues {
        if let Some(&last_used_at) = last_usage.get(&id)
            && last_used_at >= scope_end
        {
            return false;
        }
        // If the id has no recorded usage, it's safe (never read = not escaped)
    }
    true
}

/// Returns whether a reactive scope is eligible to be merged into another scope.
///
/// A scope is eligible when:
/// 1. At least one of its declarations has an "always-invalidating" type
///    (Object or Function — these always create new references, guaranteeing
///    dependent scopes must re-execute). JSX elements are typed as Object.
/// 2. The scope has no reassignments (no cross-scope StoreLocal mutations).
///
/// Matches upstream `scopeIsEligibleForMerging` in
/// `MergeReactiveScopesThatInvalidateTogether.ts`.
fn scope_is_eligible_for_merging(scope_block: &crate::hir::types::ReactiveScopeBlock) -> bool {
    use crate::hir::types::Type;

    let scope = &scope_block.scope;

    // A scope with reassignments is not eligible — cross-scope mutations
    // make the merge unsafe.
    if !scope.reassignments.is_empty() {
        return false;
    }

    // Check if at least one declaration has an always-invalidating type.
    // Objects (including arrays, JSX elements) and Functions always produce
    // new references, so any scope depending on them will always re-execute.
    scope
        .declarations
        .iter()
        .any(|(_, decl)| matches!(decl.identifier.type_, Type::Object | Type::Function))
}

/// Merge overlapping reactive scopes in the HIR.
///
/// Uses an active-scope-stack algorithm matching the upstream
/// `MergeOverlappingReactiveScopesHIR.ts`. Three phases:
///
/// 1. **Index** — Build maps of scope starts/ends per instruction ID,
///    place-to-scope ownership, and mutation sites.
/// 2. **Stack walk** — Walk instructions in ascending ID order maintaining
///    an active scope stack. Merge scopes on geometric overlap (a scope
///    ends while not at stack top) and cross-scope mutation (a mutation
///    targets a place owned by a scope different from the stack top).
/// 3. **Rewrite** — Compute merged ranges and update all identifier scope
///    annotations in-place.
pub fn merge_overlapping_reactive_scopes_hir(hir: &mut HIR) {
    // -----------------------------------------------------------------------
    // Phase 1: Build index
    // -----------------------------------------------------------------------

    // scope_data: canonical (start, end) for each ScopeId
    let mut scope_data: FxHashMap<ScopeId, (u32, u32)> = FxHashMap::default();
    // scope_starts[instr_id] = set of ScopeIds starting at that instruction
    let mut scope_starts: FxHashMap<u32, Vec<ScopeId>> = FxHashMap::default();
    // scope_ends[instr_id] = set of ScopeIds ending at that instruction
    let mut scope_ends: FxHashMap<u32, Vec<ScopeId>> = FxHashMap::default();
    // place_scope: which scope owns each identifier
    let mut place_scope: FxHashMap<IdentifierId, ScopeId> = FxHashMap::default();
    // mutated_at[instr_id] = list of IdentifierIds mutated at that instruction
    let mut mutated_at: FxHashMap<u32, Vec<IdentifierId>> = FxHashMap::default();
    // Track which ScopeIds we've already indexed (avoid duplicate entries)
    let mut seen_scopes: FxHashSet<ScopeId> = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Index scope annotations
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                let sid = scope.id;
                let start = scope.range.start.0;
                let end = scope.range.end.0;

                // Map place to its owning scope
                place_scope.insert(instr.lvalue.identifier.id, sid);

                // Only index each scope once (many identifiers share the same scope)
                if seen_scopes.insert(sid) {
                    scope_data.insert(sid, (start, end));
                    scope_starts.entry(start).or_default().push(sid);
                    scope_ends.entry(end).or_default().push(sid);
                }
            }

            // Index mutation effects
            if let Some(ref effects) = instr.effects {
                let iid = instr.id.0;
                for effect in effects {
                    let mutated_place = match effect {
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value } => {
                            Some(&value.identifier.id)
                        }
                        _ => None,
                    };
                    if let Some(id) = mutated_place {
                        mutated_at.entry(iid).or_default().push(*id);
                    }
                }
            }
        }
    }

    if scope_data.is_empty() {
        return;
    }

    // -----------------------------------------------------------------------
    // Phase 2: Active-scope-stack walk with union-find
    // -----------------------------------------------------------------------

    let mut dsu: DisjointSet<ScopeId> = DisjointSet::new();
    for &sid in scope_data.keys() {
        dsu.make_set(sid);
    }

    // Collect all interesting instruction IDs and sort them
    let mut all_ids: BTreeSet<u32> = BTreeSet::new();
    for &id in scope_starts.keys() {
        all_ids.insert(id);
    }
    for &id in scope_ends.keys() {
        all_ids.insert(id);
    }
    for &id in mutated_at.keys() {
        all_ids.insert(id);
    }

    // Active scope stack: entries are (ScopeId, end_instr_id).
    // The stack top (last element) is the innermost/shallowest scope.
    let mut active: Vec<(ScopeId, u32)> = Vec::new();

    for instr_id in all_ids {
        // 1. Process scope ends at this instruction
        if let Some(ending_scopes) = scope_ends.get(&instr_id) {
            for &ending_scope in ending_scopes {
                let ending_rep = match dsu.find(ending_scope) {
                    Some(r) => r,
                    None => continue,
                };

                // Find this scope's representative in the active stack (rightmost)
                let stack_pos =
                    active.iter().rposition(|(sid, _)| dsu.find(*sid) == Some(ending_rep));

                if let Some(pos) = stack_pos {
                    // Merge everything above pos with ending_scope
                    for &(above_sid, _) in &active[(pos + 1)..] {
                        dsu.union(ending_scope, above_sid);
                    }
                    // Pop pos and everything above it
                    active.truncate(pos);
                }
            }
        }

        // 2. Process scope starts at this instruction
        if let Some(starting_scopes) = scope_starts.get(&instr_id) {
            // Sort descending by end time; when pushed in order, the earliest-ending
            // scope ends up at the stack top (last element = innermost active scope).
            let mut to_push: Vec<(ScopeId, u32)> = starting_scopes
                .iter()
                .filter_map(|&sid| scope_data.get(&sid).map(|&(_, end)| (sid, end)))
                .collect();
            to_push.sort_by(|a, b| b.1.cmp(&a.1));

            for (sid, end) in to_push {
                // Check for identical-range scope already on stack → union instead of push
                let existing = active.iter().find(|(s, e)| {
                    *e == end && scope_data.get(s).is_some_and(|&(start, _)| start == instr_id)
                });
                if let Some(&(existing_sid, _)) = existing {
                    dsu.union(sid, existing_sid);
                } else {
                    active.push((sid, end));
                }
            }
        }

        // 3. Process mutations at this instruction
        if let Some(mutated_ids) = mutated_at.get(&instr_id)
            && let Some(&(top_sid, _)) = active.last()
        {
            let Some(top_rep) = dsu.find(top_sid) else { continue };
            for &identifier_id in mutated_ids {
                if let Some(&mutated_scope_id) = place_scope.get(&identifier_id) {
                    let mutated_rep = match dsu.find(mutated_scope_id) {
                        Some(r) => r,
                        None => continue,
                    };
                    if mutated_rep != top_rep {
                        // Find the mutated scope's position in the stack
                        let stack_pos =
                            active.iter().rposition(|(sid, _)| dsu.find(*sid) == Some(mutated_rep));
                        if let Some(pos) = stack_pos {
                            // Merge everything above pos with the mutated scope
                            for &(above_sid, _) in &active[(pos + 1)..] {
                                dsu.union(mutated_scope_id, above_sid);
                            }
                            // Do NOT pop — the scope is still active; only merge
                        }
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 3: Compute merged ranges and rewrite
    // -----------------------------------------------------------------------

    let groups = dsu.sets();

    // For each group, compute merged (min_start, max_end)
    let mut group_range: FxHashMap<ScopeId, (u32, u32)> = FxHashMap::default();
    for (rep, members) in &groups {
        let (min_start, max_end) = members
            .iter()
            .filter_map(|sid| scope_data.get(sid))
            .fold((u32::MAX, 0u32), |(ms, me), &(s, e)| (ms.min(s), me.max(e)));
        group_range.insert(*rep, (min_start, max_end));
    }

    // Build remap: old ScopeId → (representative ScopeId, new_start, new_end)
    let mut remap: FxHashMap<ScopeId, (ScopeId, u32, u32)> = FxHashMap::default();
    for (rep, members) in &groups {
        if let Some(&(min_s, max_e)) = group_range.get(rep) {
            for &member in members {
                remap.insert(member, (*rep, min_s, max_e));
            }
        }
    }

    // Apply remap to all identifier scope annotations
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(ref mut scope) = instr.lvalue.identifier.scope
                && let Some(&(rep, new_start, new_end)) = remap.get(&scope.id)
            {
                scope.id = rep;
                scope.range.start = InstructionId(new_start);
                scope.range.end = InstructionId(new_end);
            }
        }
    }
}

/// Merge reactive scopes that invalidate together.
///
/// If two scopes have the same set of dependencies, they should be merged
/// because they'll always recompute at the same time.
pub fn merge_reactive_scopes_that_invalidate_together(reactive_fn: &mut ReactiveFunction) {
    let last_usage = build_last_usage_map(reactive_fn);
    merge_scopes_in_block(&mut reactive_fn.body, &last_usage);
}

/// Canonical dependency key for comparing scope deps by name + path,
/// not by IdentifierId (which is SSA-unique per Place reference).
/// DIVERGENCE: Upstream uses identifier name + property path for dep comparison.
/// Our HIR creates fresh IdentifierIds per Place, so ID comparison would
/// make almost all scopes appear to have different deps.
type DepKey = (Option<String>, Vec<DependencyPathEntry>);

fn dep_key_set(scope: &crate::hir::types::ReactiveScope) -> BTreeSet<DepKey> {
    scope.dependencies.iter().map(|d| (d.identifier.name.clone(), d.path.clone())).collect()
}

/// Check if two scopes can be merged. Returns true under two conditions:
/// 1. Identical dependencies (same dep key sets, both non-empty)
/// 2. Output-to-input chain: prev's declarations are next's dependencies,
///    and all matched declarations have always-invalidating types.
///
/// Matches upstream `canMergeScopes` in `MergeReactiveScopesThatInvalidateTogether.ts`.
fn can_merge_scopes(prev: &ReactiveScope, next: &ReactiveScope) -> bool {
    use crate::hir::types::Type;

    // Reassignment guard — scopes with cross-scope reassignments cannot merge
    if !prev.reassignments.is_empty() || !next.reassignments.is_empty() {
        return false;
    }

    // Branch 1: identical deps (both non-empty)
    let prev_deps = dep_key_set(prev);
    let next_deps = dep_key_set(next);
    if !prev_deps.is_empty() && prev_deps == next_deps {
        return true;
    }

    // Branch 2: output-to-input chain
    // All next-deps must have empty paths (direct variable references)
    if !next.dependencies.iter().all(|d| d.path.is_empty()) {
        return false;
    }

    // Every dep of next must match a declaration of prev with an always-invalidating type
    // DIVERGENCE: Upstream uses DeclarationId for matching; we use name-based matching
    // as a workaround until Sub-task 4f aligns DeclarationIds.
    // TODO(4f): use declarationId via temporaries for precise alias resolution
    if next.dependencies.is_empty() {
        return false;
    }

    for dep in &next.dependencies {
        let dep_name = &dep.identifier.name;
        let matched = prev.declarations.iter().any(|(_, decl)| {
            &decl.identifier.name == dep_name
                && matches!(decl.identifier.type_, Type::Object | Type::Function)
        });
        if !matched {
            return false;
        }
    }

    true
}

/// Merge dependencies from absorbee into winner, deduplicating by name+path.
fn merge_scope_deps(winner: &mut ReactiveScope, absorbee: &ReactiveScope) {
    let existing_keys = dep_key_set(winner);
    for dep in &absorbee.dependencies {
        let key = (dep.identifier.name.clone(), dep.path.clone());
        if !existing_keys.contains(&key) {
            winner.dependencies.push(dep.clone());
        }
    }
}

fn merge_scopes_in_block(block: &mut ReactiveBlock, last_usage: &LastUsageMap) {
    // -----------------------------------------------------------------------
    // Pass 1: Recurse into nested blocks first (inner blocks must be simplified
    // before outer merge decisions are made)
    // -----------------------------------------------------------------------
    for instr in &mut block.instructions {
        match instr {
            crate::hir::types::ReactiveInstruction::Scope(scope_block) => {
                merge_scopes_in_block(&mut scope_block.instructions, last_usage);
            }
            crate::hir::types::ReactiveInstruction::Terminal(terminal) => {
                merge_scopes_in_terminal(terminal, last_usage);
            }
            crate::hir::types::ReactiveInstruction::Instruction(_) => {}
        }
    }

    // -----------------------------------------------------------------------
    // Pass 2: Build merge plan by walking instructions with a MergeCandidate
    // state machine
    // -----------------------------------------------------------------------
    struct MergeRecord {
        winner_idx: usize,
        gap_indices: Vec<usize>,
        absorbee_idx: usize,
    }

    let mut merge_records: Vec<MergeRecord> = Vec::new();
    let mut candidate: Option<(usize, IntermediateAccumulator, Vec<usize>)> = None;
    // (scope_index, accumulator, gap_indices)

    for i in 0..block.instructions.len() {
        match &block.instructions[i] {
            crate::hir::types::ReactiveInstruction::Terminal(_) => {
                // Terminals reset the merge candidate
                candidate = None;
            }
            crate::hir::types::ReactiveInstruction::Instruction(instr) => {
                if let Some((_, ref mut acc, ref mut gap_indices)) = candidate {
                    if accumulate_intermediate_instruction(instr, acc) {
                        gap_indices.push(i);
                    } else {
                        candidate = None;
                    }
                }
                // If no candidate, plain instructions are ignored
            }
            crate::hir::types::ReactiveInstruction::Scope(scope_block) => {
                let should_merge = if let Some((cand_idx, ref acc, _)) = candidate {
                    // Get the candidate scope to compare against
                    if let crate::hir::types::ReactiveInstruction::Scope(cand_scope) =
                        &block.instructions[cand_idx]
                    {
                        can_merge_scopes(&cand_scope.scope, &scope_block.scope)
                            && are_lvalues_last_used_by_scope(
                                &scope_block.scope,
                                &acc.lvalues,
                                last_usage,
                            )
                    } else {
                        false
                    }
                } else {
                    false
                };

                if should_merge {
                    let (cand_idx, acc, gap_indices) = candidate.as_mut().unwrap();
                    // Record the merge
                    merge_records.push(MergeRecord {
                        winner_idx: *cand_idx,
                        gap_indices: std::mem::take(gap_indices),
                        absorbee_idx: i,
                    });
                    // Clear accumulator for potential further consecutive merges
                    acc.clear();
                    // Keep the same winner as candidate if the current scope is
                    // eligible (allows chaining A+B+C)
                    if !scope_is_eligible_for_merging(scope_block) {
                        candidate = None;
                    }
                } else {
                    // Start a new candidate if this scope is eligible
                    if scope_is_eligible_for_merging(scope_block) {
                        candidate = Some((i, IntermediateAccumulator::new(), Vec::new()));
                    } else {
                        candidate = None;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Pass 3: Reconstruct block.instructions by applying merge records
    // -----------------------------------------------------------------------
    if merge_records.is_empty() {
        return;
    }

    // Build set of absorbed indices (gap instructions + absorbed scopes)
    let mut absorbed: FxHashSet<usize> = FxHashSet::default();
    for record in &merge_records {
        absorbed.insert(record.absorbee_idx);
        for &gi in &record.gap_indices {
            absorbed.insert(gi);
        }
    }

    // Take ownership of all instructions
    let mut indexed: Vec<Option<crate::hir::types::ReactiveInstruction>> =
        std::mem::take(&mut block.instructions).into_iter().map(Some).collect();

    // Apply merges in order
    for record in &merge_records {
        // Extract gap instructions
        let gap_instrs: Vec<crate::hir::types::ReactiveInstruction> =
            record.gap_indices.iter().filter_map(|&gi| indexed[gi].take()).collect();

        // Extract absorbee scope
        let absorbee = indexed[record.absorbee_idx].take();
        let Some(crate::hir::types::ReactiveInstruction::Scope(absorbee_scope)) = absorbee else {
            continue;
        };

        // Mutate winner scope to absorb gap + absorbee
        if let Some(crate::hir::types::ReactiveInstruction::Scope(ref mut winner)) =
            indexed[record.winner_idx]
        {
            // Extend winner scope range
            winner.scope.range.end = absorbee_scope.scope.range.end;
            // Absorb gap instructions into winner's body
            winner.instructions.instructions.extend(gap_instrs);
            // Absorb absorbee's instructions
            winner.instructions.instructions.extend(absorbee_scope.instructions.instructions);
            // Union dependencies
            merge_scope_deps(&mut winner.scope, &absorbee_scope.scope);
            // Union declarations
            winner.scope.declarations.extend(absorbee_scope.scope.declarations);
            // Track merged scope ID
            winner.scope.merged.push(absorbee_scope.scope.id);
        }
    }

    // Collect remaining (non-absorbed) instructions in original order
    block.instructions = indexed.into_iter().flatten().collect();
}

fn merge_scopes_in_terminal(
    terminal: &mut crate::hir::types::ReactiveTerminal,
    last_usage: &LastUsageMap,
) {
    use crate::hir::types::ReactiveTerminal;
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            merge_scopes_in_block(consequent, last_usage);
            merge_scopes_in_block(alternate, last_usage);
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            merge_scopes_in_block(init, last_usage);
            merge_scopes_in_block(test, last_usage);
            if let Some(upd) = update {
                merge_scopes_in_block(upd, last_usage);
            }
            merge_scopes_in_block(body, last_usage);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            merge_scopes_in_block(init, last_usage);
            merge_scopes_in_block(test, last_usage);
            merge_scopes_in_block(body, last_usage);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            merge_scopes_in_block(test, last_usage);
            merge_scopes_in_block(body, last_usage);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                merge_scopes_in_block(block, last_usage);
            }
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            merge_scopes_in_block(block, last_usage);
            merge_scopes_in_block(handler, last_usage);
        }
        ReactiveTerminal::Label { block, .. } => {
            merge_scopes_in_block(block, last_usage);
        }
        ReactiveTerminal::Return { .. } | ReactiveTerminal::Throw { .. } => {}
    }
}
