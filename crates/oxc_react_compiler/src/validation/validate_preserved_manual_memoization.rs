use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{
    DependencyPathEntry, IdentifierId, Instruction, InstructionValue, ManualMemoDependency,
    ManualMemoDependencyRoot, ReactiveBlock, ReactiveFunction, ReactiveInstruction,
    ReactiveScopeDependency, ScopeId,
};
use rustc_hash::{FxHashMap, FxHashSet};

const UNMEMOIZED_ERROR: &str = "Existing memoization could not be preserved. React Compiler \
has skipped optimizing this component because the existing manual memoization \
could not be preserved. This value was memoized in source but not in \
compilation output.";

const INFERRED_DEP_ERROR: &str = "Existing memoization could not be preserved. React Compiler \
has skipped optimizing this component because the existing manual memoization \
could not be preserved. The inferred dependencies did not match the manually \
specified dependencies, which could cause the value to change more or less \
frequently than expected.";

/// Validate that compiler-generated memoization preserves manual memoization
/// guarantees from `useMemo` / `useCallback`.
///
/// Upstream: ValidatePreservedManualMemoization.ts
///
/// After reactive scope inference and RF optimization passes, walk the reactive
/// function in evaluation order. For each manual memoization region (StartMemoize/
/// FinishMemoize pair), perform two checks:
///
/// 1. **Scope completion** (Check 1): The memoized value's identifier must have
///    a scope that completed (exists in `completed_scopes` post-order set).
///    Upstream: `isUnmemoized(operand, scopes)` — fires when
///    `operand.scope != null && !scopes.has(operand.scope.id)`.
///
/// 2. **validateInferredDep** (Check 2): For each reactive scope inside a memo
///    region, verify that every inferred scope dependency matches one of the
///    source dependencies from the useMemo/useCallback dep array.
///
/// DIVERGENCE: Upstream's Check 3 ("dep mutated later") requires `identifier.scope`
/// populated on StartMemoize operands, which we don't currently store. Skipped.
pub(crate) fn validate_preserved_manual_memoization(
    func: &ReactiveFunction,
    errors: &mut ErrorCollector,
    pre_inline_temporaries: Option<&TempResolutionMap>,
) {
    // Pass 1: Build the full temporaries map from ALL instructions.
    // If a pre-computed map (built before inline_load_locals) is provided,
    // merge it with the current map so we can resolve temps whose LoadLocal
    // instructions were removed by inlining.
    let mut temporaries: FxHashMap<IdentifierId, ResolvedDep> = FxHashMap::default();
    if let Some(pre) = pre_inline_temporaries {
        temporaries.extend(pre.iter().map(|(k, v)| (*k, v.clone())));
    }
    build_temporaries_map(&func.body, &mut temporaries);

    // Pass 2: Walk in evaluation order, checking memo regions and scope deps
    let mut state = WalkerState {
        memo_regions: FxHashMap::default(),
        active_memos: FxHashSet::default(),
        temporaries,
        decls_within_memos: FxHashSet::default(),
        active_source_deps: FxHashMap::default(),
        completed_scopes: FxHashSet::default(),
    };
    walk_reactive_block(&func.body, &mut state);

    for region in state.memo_regions.values() {
        // Skip pruned memoizations
        if region.pruned {
            continue;
        }

        // Skip if ValidateExhaustiveDependencies already flagged invalid deps.
        if region.has_invalid_deps {
            continue;
        }

        // Check 1: The memoized value's scope did not complete — value is unmemoized.
        // Upstream: `isUnmemoized(operand, scopes)` — checks that the identifier's
        // scope exists AND is in the completed_scopes set (post-order traversal).
        // DIVERGENCE: Upstream checks all FinishMemoize operands (decl + deps),
        // not just decl. We only check decl because our deps are often unresolved
        // tN temps without scope information. This is equivalent for the common case
        // where decl carries the scope of the memoized output.
        let value_unmemoized = match region.decl_scope_id {
            Some(scope_id) => !state.completed_scopes.contains(&scope_id),
            None => true, // FinishMemoize was never seen or decl has no scope
        };
        if value_unmemoized {
            errors.push(CompilerError::invalid_react_with_kind(
                region.loc,
                UNMEMOIZED_ERROR,
                DiagnosticKind::MemoizationPreservation,
            ));
            continue;
        }

        // Check 2: Inferred deps don't match source deps
        if region.has_dep_mismatch {
            errors.push(CompilerError::invalid_react_with_kind(
                region.loc,
                INFERRED_DEP_ERROR,
                DiagnosticKind::MemoizationPreservation,
            ));
        }
    }
}

/// Tracks the reactive scope context of a StartMemoize/FinishMemoize pair.
#[derive(Debug)]
struct MemoRegion {
    /// The scope ID from FinishMemoize's `decl.identifier.scope`. Used for Check 1:
    /// if this scope did not complete (not in `completed_scopes`), the memoized
    /// value was not preserved. Only the ScopeId is stored to avoid cloning the
    /// full Identifier (which contains heavyweight nested ReactiveScope).
    decl_scope_id: Option<ScopeId>,
    pruned: bool,
    has_invalid_deps: bool,
    has_dep_mismatch: bool,
    loc: oxc_span::Span,
}

/// Resolved temporary: maps a temp identifier to its named source dep.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedDep {
    pub(crate) root_name: String,
    pub(crate) is_global: bool,
    pub(crate) path: Vec<DependencyPathEntry>,
}

/// Type alias for the pre-computed temp resolution map.
pub(crate) type TempResolutionMap = FxHashMap<IdentifierId, ResolvedDep>;

/// State threaded through the reactive block walker (pass 2).
#[derive(Debug)]
struct WalkerState {
    memo_regions: FxHashMap<u32, MemoRegion>,
    active_memos: FxHashSet<u32>,
    temporaries: FxHashMap<IdentifierId, ResolvedDep>,
    decls_within_memos: FxHashSet<IdentifierId>,
    active_source_deps: FxHashMap<u32, Vec<ManualMemoDependency>>,
    /// Scope IDs that have been fully traversed (post-order).
    /// Used for Check 1: a memoized value's scope must be in this set
    /// for it to count as "memoized in compilation output".
    /// Upstream: `scopes: Set<ScopeId>` in `Visitor`.
    completed_scopes: FxHashSet<ScopeId>,
}

// ── Pass 1: Build temporaries map ──────────────────────────────────────────

/// Walk reactive function instructions to build the temporaries resolution map.
///
/// Public so the pipeline can pre-compute this before `inline_load_locals`
/// removes LoadLocal instructions needed for temp resolution.
fn build_temporaries_map(
    block: &ReactiveBlock,
    temporaries: &mut FxHashMap<IdentifierId, ResolvedDep>,
) {
    for item in &block.instructions {
        match item {
            ReactiveInstruction::Instruction(instr) => {
                record_temporary(instr, temporaries);
            }
            ReactiveInstruction::Scope(scope_block) => {
                build_temporaries_map(&scope_block.instructions, temporaries);
            }
            ReactiveInstruction::Terminal(terminal) => {
                build_temporaries_terminal(terminal, temporaries);
            }
        }
    }
}

/// Record a single instruction's contribution to the temporaries map.
fn record_temporary(instr: &Instruction, temporaries: &mut FxHashMap<IdentifierId, ResolvedDep>) {
    let lv_id = instr.lvalue.identifier.id;
    match &instr.value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            if let Some(ref name) = place.identifier.name {
                temporaries.insert(
                    lv_id,
                    ResolvedDep { root_name: name.clone(), is_global: false, path: Vec::new() },
                );
            }
        }
        InstructionValue::LoadGlobal { binding } => {
            temporaries.insert(
                lv_id,
                ResolvedDep { root_name: binding.name.clone(), is_global: true, path: Vec::new() },
            );
        }
        InstructionValue::PropertyLoad { object, property, .. } => {
            if let Some(base) = temporaries.get(&object.identifier.id).cloned() {
                let mut path = base.path;
                path.push(DependencyPathEntry { property: property.clone(), optional: false });
                temporaries.insert(
                    lv_id,
                    ResolvedDep { root_name: base.root_name, is_global: base.is_global, path },
                );
            }
        }
        InstructionValue::StoreLocal { lvalue, value, .. } => {
            if let Some(ref name) = lvalue.identifier.name {
                if let Some(resolved) = temporaries.get(&value.identifier.id).cloned() {
                    temporaries.insert(lvalue.identifier.id, resolved);
                } else {
                    temporaries.insert(
                        lvalue.identifier.id,
                        ResolvedDep { root_name: name.clone(), is_global: false, path: Vec::new() },
                    );
                }
            }
        }
        InstructionValue::DeclareLocal { lvalue, .. } => {
            if let Some(ref name) = lvalue.identifier.name {
                temporaries.insert(
                    lvalue.identifier.id,
                    ResolvedDep { root_name: name.clone(), is_global: false, path: Vec::new() },
                );
            }
        }
        _ => {}
    }
}

/// Build a temp resolution map from an HIR (CFG form), before RF conversion.
///
/// Captures LoadLocal → named-local mappings that will be lost after
/// `build_reactive_function` and `inline_load_locals`.
pub(crate) fn build_temporaries_map_from_hir(
    hir: &crate::hir::types::HIR,
    temporaries: &mut FxHashMap<IdentifierId, ResolvedDep>,
) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            record_temporary(instr, temporaries);
        }
    }
}

/// Walk terminals for pass 1.
fn build_temporaries_terminal(
    terminal: &crate::hir::types::ReactiveTerminal,
    temporaries: &mut FxHashMap<IdentifierId, ResolvedDep>,
) {
    use crate::hir::types::ReactiveTerminal;
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            build_temporaries_map(consequent, temporaries);
            build_temporaries_map(alternate, temporaries);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, case_block) in cases {
                build_temporaries_map(case_block, temporaries);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            build_temporaries_map(init, temporaries);
            build_temporaries_map(test, temporaries);
            if let Some(upd) = update {
                build_temporaries_map(upd, temporaries);
            }
            build_temporaries_map(body, temporaries);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            build_temporaries_map(init, temporaries);
            build_temporaries_map(test, temporaries);
            build_temporaries_map(body, temporaries);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            build_temporaries_map(test, temporaries);
            build_temporaries_map(body, temporaries);
        }
        ReactiveTerminal::Label { block, .. } => {
            build_temporaries_map(block, temporaries);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            build_temporaries_map(block, temporaries);
            build_temporaries_map(handler, temporaries);
        }
        ReactiveTerminal::Logical { right, .. } => {
            build_temporaries_map(right, temporaries);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
    }
}

// ── Pass 2: Walk and validate ──────────────────────────────────────────────

/// Recursively walk a reactive block, tracking scope membership and memo regions.
fn walk_reactive_block(block: &ReactiveBlock, state: &mut WalkerState) {
    for item in &block.instructions {
        match item {
            ReactiveInstruction::Instruction(instr) => {
                track_memo_decls(instr, state);
                check_instruction(instr, state);
            }
            ReactiveInstruction::Scope(scope_block) => {
                // Check scope deps against active memos' source deps (Check 2)
                validate_scope_deps(scope_block, state);
                // Recurse into scope block, then mark completed (post-order)
                walk_reactive_block(&scope_block.instructions, state);

                // Post-order: mark this scope (and its merged scopes) as completed.
                // Upstream: `this.scopes.add(scopeBlock.scope.id)` in visitScope.
                state.completed_scopes.insert(scope_block.scope.id);
                for &merged_id in &scope_block.scope.merged {
                    state.completed_scopes.insert(merged_id);
                }
            }
            ReactiveInstruction::Terminal(terminal) => {
                walk_terminal_blocks(terminal, state);
            }
        }
    }
}

/// Track declarations inside active memo blocks.
fn track_memo_decls(instr: &Instruction, state: &mut WalkerState) {
    if state.active_memos.is_empty() {
        return;
    }
    match &instr.value {
        InstructionValue::StoreLocal { lvalue, .. } => {
            if lvalue.identifier.name.is_some() {
                state.decls_within_memos.insert(lvalue.identifier.id);
            }
        }
        InstructionValue::DeclareLocal { lvalue, .. } => {
            state.decls_within_memos.insert(lvalue.identifier.id);
        }
        _ => {}
    }
}

/// Check a single instruction for StartMemoize / FinishMemoize markers.
fn check_instruction(instr: &Instruction, state: &mut WalkerState) {
    match &instr.value {
        InstructionValue::StartMemoize { manual_memo_id, has_invalid_deps, source_deps } => {
            let entry = state.memo_regions.entry(*manual_memo_id).or_insert(MemoRegion {
                decl_scope_id: None,
                pruned: false,
                has_invalid_deps: *has_invalid_deps,
                has_dep_mismatch: false,
                loc: instr.loc,
            });
            entry.has_invalid_deps = *has_invalid_deps;

            // Activate memo region for dep checking
            if let Some(deps) = source_deps {
                state.active_memos.insert(*manual_memo_id);
                state.active_source_deps.insert(*manual_memo_id, deps.clone());
            }
        }
        InstructionValue::FinishMemoize { manual_memo_id, pruned, decl, .. } => {
            let entry = state.memo_regions.entry(*manual_memo_id).or_insert(MemoRegion {
                decl_scope_id: None,
                pruned: *pruned,
                has_invalid_deps: false,
                has_dep_mismatch: false,
                loc: instr.loc,
            });
            entry.decl_scope_id = decl.identifier.scope.as_ref().map(|s| s.id);
            entry.pruned = *pruned;

            // Deactivate memo region
            state.active_memos.remove(manual_memo_id);
            state.active_source_deps.remove(manual_memo_id);

            // Clear per-memo declaration tracking when no more active memos
            if state.active_memos.is_empty() {
                state.decls_within_memos.clear();
            }
        }
        _ => {}
    }
}

/// Validate inferred scope dependencies against source deps from active memo regions.
fn validate_scope_deps(
    scope_block: &crate::hir::types::ReactiveScopeBlock,
    state: &mut WalkerState,
) {
    if state.active_memos.is_empty() {
        return;
    }

    // Collect mismatched memo IDs first to avoid borrow conflicts
    let active_ids: Vec<u32> = state.active_memos.iter().copied().collect();
    let mut mismatched_ids: Vec<u32> = Vec::new();

    for memo_id in active_ids {
        let source_deps = match state.active_source_deps.get(&memo_id) {
            Some(deps) => deps,
            None => continue,
        };

        let has_mismatch = scope_block.scope.dependencies.iter().any(|scope_dep| {
            // Skip deps declared within the memo block
            if state.decls_within_memos.contains(&scope_dep.identifier.id) {
                return false;
            }

            // Resolve the scope dep through temporaries
            let resolved = match resolve_scope_dep(scope_dep, &state.temporaries) {
                Some(r) => r,
                None => return false,
            };

            // Compare against source deps
            !matches_any_source_dep(&resolved, source_deps)
        });

        if has_mismatch {
            mismatched_ids.push(memo_id);
        }
    }

    // Apply mismatches
    for memo_id in mismatched_ids {
        if let Some(region) = state.memo_regions.get_mut(&memo_id) {
            region.has_dep_mismatch = true;
        }
    }
}

/// Resolve a scope dependency through the temporaries map.
fn resolve_scope_dep(
    dep: &ReactiveScopeDependency,
    temporaries: &FxHashMap<IdentifierId, ResolvedDep>,
) -> Option<ResolvedDep> {
    // First try resolving through temporaries (for unnamed temps)
    if let Some(resolved) = temporaries.get(&dep.identifier.id) {
        let mut result = resolved.clone();
        // Append the scope dep's own path
        result.path.extend(dep.path.iter().cloned());
        return Some(result);
    }

    // If the dep has a name, use it directly
    if let Some(ref name) = dep.identifier.name {
        return Some(ResolvedDep {
            root_name: name.clone(),
            is_global: false,
            path: dep.path.clone(),
        });
    }

    None
}

/// Check if a resolved dep matches any source dep.
fn matches_any_source_dep(resolved: &ResolvedDep, source_deps: &[ManualMemoDependency]) -> bool {
    source_deps.iter().any(|src| {
        // DIVERGENCE: Upstream emits a separate ref-access diagnostic for
        // RefAccessDifference. We treat it as a match to avoid false positives
        // until we port the ref-access-specific error path.
        matches!(
            compare_deps(resolved, src),
            CompareDepsResult::Ok | CompareDepsResult::RefAccessDifference
        )
    })
}

/// Result of comparing an inferred dep against a source dep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompareDepsResult {
    Ok,
    RootDifference,
    PathDifference,
    Subpath,
    RefAccessDifference,
}

/// Compare an inferred/resolved dep against a source dep.
fn compare_deps(inferred: &ResolvedDep, source: &ManualMemoDependency) -> CompareDepsResult {
    // 1. Check root equality
    let roots_equal = match &source.root {
        ManualMemoDependencyRoot::NamedLocal { name } => {
            !inferred.is_global && inferred.root_name == *name
        }
        ManualMemoDependencyRoot::Global { name } => {
            inferred.is_global && inferred.root_name == *name
        }
    };
    if !roots_equal {
        return CompareDepsResult::RootDifference;
    }

    // 2. Walk the shorter path, checking property names match
    let inferred_path = &inferred.path;
    let source_path = &source.path;
    let min_len = inferred_path.len().min(source_path.len());

    for i in 0..min_len {
        if inferred_path[i].property != source_path[i].property {
            return CompareDepsResult::PathDifference;
        }
    }

    // 3. Determine result based on path lengths
    if inferred_path.len() == source_path.len() {
        let has_current = inferred_path.iter().any(|e| e.property == "current")
            || source_path.iter().any(|e| e.property == "current");
        if has_current { CompareDepsResult::RefAccessDifference } else { CompareDepsResult::Ok }
    } else if inferred_path.len() < source_path.len() {
        // Inferred is less specific than source (e.g. inferred `propA` vs source
        // `propA.x`). Upstream treats this as Subpath — the compiler's inferred dep
        // is a broader scope, which means it may invalidate more often than the
        // manual memo specifies. This is a preservation failure.
        let has_current = source_path.iter().any(|e| e.property == "current");
        if has_current {
            CompareDepsResult::RefAccessDifference
        } else {
            CompareDepsResult::Subpath
        }
    } else {
        // Inferred is more specific than source — subpath issue
        let has_current = inferred_path.iter().any(|e| e.property == "current");
        if has_current {
            CompareDepsResult::RefAccessDifference
        } else {
            CompareDepsResult::Subpath
        }
    }
}

/// Walk all blocks within a reactive terminal.
fn walk_terminal_blocks(terminal: &crate::hir::types::ReactiveTerminal, state: &mut WalkerState) {
    use crate::hir::types::ReactiveTerminal;

    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            walk_reactive_block(consequent, state);
            walk_reactive_block(alternate, state);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, case_block) in cases {
                walk_reactive_block(case_block, state);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            walk_reactive_block(init, state);
            walk_reactive_block(test, state);
            if let Some(upd) = update {
                walk_reactive_block(upd, state);
            }
            walk_reactive_block(body, state);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            walk_reactive_block(init, state);
            walk_reactive_block(test, state);
            walk_reactive_block(body, state);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            walk_reactive_block(test, state);
            walk_reactive_block(body, state);
        }
        ReactiveTerminal::Label { block, .. } => {
            walk_reactive_block(block, state);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            walk_reactive_block(block, state);
            walk_reactive_block(handler, state);
        }
        ReactiveTerminal::Logical { right, .. } => {
            walk_reactive_block(right, state);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
    }
}
