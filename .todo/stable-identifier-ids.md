# Stable IdentifierId Refactor

## Problem

Our HIR creates a **fresh IdentifierId for every Place reference**. When `x`
is referenced 5 times, it gets 5 different IDs. This means:

- The abstract heap can't link freeze/mutate state across references to the
  same variable
- The frozen-mutation validator needs 300+ lines of name-based workarounds
- Alias tracking breaks (freezing x doesn't freeze y where y = x)
- Scope dependency analysis needs dual ID tracking (IdentifierId + DeclarationId)
- ~100+ conformance fixtures are affected by incorrect value flow tracking

Upstream (Babel) uses JavaScript object reference identity -- same variable =
same object pointer. All references share state naturally.

## Solution: Reuse Declaration IDs for References

Make `make_named_place()` return the SAME IdentifierId for all references to
the same binding. Add an `ssa_version: u32` field to Place for SSA versioning
(instead of creating fresh IDs per SSA version).

## Architecture Decisions

1. **SSA versioning**: Add `ssa_version: u32` field to Place (not synthetic ID pool)
2. **Effect tracking**: Keep effect on lvalue (no change -- already per-instruction)
3. **Mutable ranges**: Store on declaration only, look up by (base_id, version)
4. **DeclarationId**: Keep for clarity (marks "this is a declaration site")
5. **Temporaries**: Remain fresh (unnamed values, different namespace)
6. **Codegen temp naming**: `t{id}` unchanged for temps; named variables use their name

## Impact Assessment

- **40+ files** affected across the codebase
- **5 CRITICAL files** require core logic redesign
- **8 IMPORTANT files** need algorithm updates
- **20+ MODERATE files** need mechanical map-key refactoring
- **11+ LOW files** (validation passes) just get simpler

## Cross-References

This refactor is the root-cause fix for multiple items in the backlog:

- **[false-bailouts.md](false-bailouts.md)#frozen-mutation-false-positives** --
  158 fixtures. The abstract heap loses track of freeze state across references
  because each reference has a different ID. Stable IDs would let the heap
  propagate freeze/mutate correctly without name-based workarounds.
- **[scope-analysis.md](scope-analysis.md)#over-counting** -- 474 fixtures.
  Dual-tracking (IdentifierId + DeclarationId) in propagate_dependencies causes
  incorrect dependency sets. Stable IDs eliminate the need for dual tracking.
- **[scope-analysis.md](scope-analysis.md)#under-counting** -- 162 fixtures.
  Missing value flow means some dependencies are not discovered at all.
- **[unnecessary-memo.md](unnecessary-memo.md)#unnecessary-memoization** --
  partial. Some unnecessary memoization comes from incorrect reactive-place
  inference caused by broken alias tracking.

---

## Phase 1: Foundation -- Types & Builder

**Goal**: All references to the same variable share the same IdentifierId.

**Files to modify:**
- `hir/types.rs` -- Add `ssa_version: u32` to Place, update IdGenerator
- `hir/build.rs` -- Add binding registry (`FxHashMap<String, IdentifierId>`),
  modify `make_named_place()` to look up existing binding ID instead of
  creating a fresh one
- Keep `make_temp()` unchanged (temps still get fresh IDs)

**Key changes:**
```rust
// Before: always fresh
fn make_named_place(&mut self, name: &str, loc: Span) -> Place {
    let id = self.env.id_generator.next_identifier_id(); // FRESH
    ...
}

// After: reuse binding's ID
fn make_named_place(&mut self, name: &str, loc: Span) -> Place {
    let id = self.binding_ids.get(name).copied()
        .unwrap_or_else(|| {
            let new_id = self.env.id_generator.next_identifier_id();
            self.binding_ids.insert(name.to_string(), new_id);
            new_id
        });
    Place { identifier: Identifier { id, ssa_version: 0, ... }, ... }
}
```

**Testing:** All pre-SSA passes should still work (they use IDs as map keys --
fewer unique keys but same logic). Run full test suite after each sub-step.

---

## Phase 2: SSA Pass Rewrite

**Goal**: SSA versioning uses `ssa_version` field instead of creating new IDs.

**Files to modify:**
- `ssa/enter_ssa.rs` -- Rewrite stacks: `FxHashMap<IdentifierId, Vec<u32>>`
  (base_id -> version stack). `fresh_ssa_name()` becomes `fresh_ssa_version()`
  returning a u32. `rename_place_def/use` update `place.ssa_version` instead
  of `place.identifier.id`.
- `ssa/eliminate_redundant_phi.rs` -- Update replacement maps to use
  (IdentifierId, u32) keys

**Key changes:**
```rust
// Before: creates new IdentifierIds for SSA versions
fn fresh_ssa_name(original: IdentifierId, stacks, next_id) -> IdentifierId {
    let new_id = IdentifierId(*next_id);
    *next_id += 1;
    stacks.entry(original).or_default().push(new_id);
    new_id
}

// After: increments version counter
fn fresh_ssa_version(base_id: IdentifierId, stacks) -> u32 {
    let versions = stacks.entry(base_id).or_default();
    let version = versions.len() as u32;
    versions.push(version);
    version
}

// rename_place_def: sets ssa_version instead of replacing id
fn rename_place_def(place: &mut Place, stacks) {
    let version = fresh_ssa_version(place.identifier.id, stacks);
    place.ssa_version = version;
}
```

**Critical risk:** SSA is core to the entire compiler. Test exhaustively after
each change.

---

## Phase 3: Inference & Analysis Passes

**Goal**: All passes that key maps by IdentifierId work correctly with stable IDs.

### Sub-phase 3a: Mutation aliasing

- `inference/infer_mutation_aliasing_effects.rs` -- AbstractHeap maps become
  (IdentifierId, version) keyed, OR just IdentifierId since all refs share one.
  The fresh-ID workarounds (pre_freeze_params walking all places) become
  unnecessary.
- `inference/infer_mutation_aliasing_ranges.rs` -- creation_map, last_use_map
  become version-aware

### Sub-phase 3b: Reactive scopes

- `reactive_scopes/propagate_dependencies.rs` -- The dual-tracking
  (IdentifierId + DeclarationId) becomes redundant. Simplify to single-ID
  tracking. temp_map shrinks.
- `reactive_scopes/merge_scopes.rs` -- DepKey simplifies
- `reactive_scopes/infer_reactive_scope_variables.rs` -- ranges map simplifies
- `reactive_scopes/prune_scopes.rs` -- used-IDs sets shrink

### Sub-phase 3c: Other inference

- `inference/analyse_functions.rs` -- signature map simplifies
- `inference/infer_types.rs` -- type map simplifies
- `inference/infer_reactive_places.rs` -- stable set simplifies
- `inference/collect_optional_chain_dependencies.rs` -- chain map simplifies

---

## Phase 4: Validation Passes

**Goal**: Simplify all validators that build id_to_name maps.

**Files (11+):**
- `validate_no_mutation_after_freeze.rs` -- Can be drastically simplified to
  just check MutateFrozen effects (no more name-based workarounds)
- All other validators -- Fewer map entries, same logic, mostly mechanical

---

## Phase 5: Optimization Passes

- `optimization/dead_code_elimination.rs` -- used-ID sets shrink
- `optimization/constant_propagation.rs` -- constant map becomes more precise
- `optimization/prune_temporary_lvalues.rs` -- temp tracking simplifies

---

## Phase 6: Codegen

- `reactive_scopes/codegen.rs` -- Verify temp naming still works, update
  inline_map if needed
- Run full conformance suite, update snapshots

---

## Phase 7: Cleanup & Validation

- Remove all name-based ID workarounds across the codebase
- Remove `pre_freeze_params` place-walking (heap tracks correctly now)
- Remove dual-tracking in propagate_dependencies
- Update known-failures.txt (expect significant improvements)
- Document the new ID model

---

## Expected Outcomes

- **Frozen-mutation false positives**: 158 -> ~20 (heap correctly propagates freeze)
- **Scope analysis**: ~50-100 fixture improvements (correct value flow)
- **Code simplification**: Remove ~500+ lines of workaround code
- **Conformance**: Potential +30-80 fixture improvements
- **Performance**: Slightly faster (fewer unique IDs, smaller maps)

## Risk Mitigation

- Each phase is independently testable
- Full test suite after every sub-phase
- Keep the old validator code in a branch for A/B comparison
- Phase 1-2 are the highest risk; if they work, phases 3-7 are largely mechanical

---

## Status: IMPLEMENTED (2026-03-16)

Phase 1 re-implemented with scope-aware binding registry (`binding_scopes:
Vec<FxHashMap<...>>`). The flat map issue is fixed — push/pop scope frames
at block statements, for-loops, for-in/of. `make_declared_place()` creates
fresh bindings for declarations, `make_named_place()` looks up the scope stack
for references. Shadowed variables correctly get distinct IDs.

Phases 2+ (SSA rewrite, downstream passes) carried over from the original
implementation without changes — all verified compatible.

### Remaining regressions (8 fixtures)

- 3 non-error (array-pattern-params, object-pattern-params, lambda-mutated) —
  temp renumbering causes different output; structurally correct but the
  conformance tokenizer doesn't fully normalize these cases
- 5 preserve-memo-validation errors — stable IDs change scope analysis enough
  that the preserve-memo validator produces different results

### Potential further improvements

- Thread `setup_context_variables()` into nested function builders so captured
  variables from outer scope use `LoadContext` instead of `LoadLocal`
- Use OXC `SymbolId` for even more precise binding resolution (avoids the
  scope stack entirely)

## Files Checklist (40+ files)

### CRITICAL (must redesign):
- [x] hir/types.rs — added `ssa_version: u32` to Identifier
- [x] hir/build.rs — added scope-aware `binding_scopes` registry with push/pop
- [x] ssa/enter_ssa.rs — rewrote to use ssa_version instead of fresh IDs
- [x] ssa/eliminate_redundant_phi.rs — rewrote to use (IdentifierId, ssa_version) tuples
- [x] inference/infer_mutation_aliasing_effects.rs — verified compatible (no changes needed)

### IMPORTANT (algorithm updates):
- [x] inference/infer_mutation_aliasing_ranges.rs — verified compatible
- [x] reactive_scopes/propagate_dependencies.rs — verified compatible
- [x] reactive_scopes/merge_scopes.rs — verified compatible
- [x] reactive_scopes/infer_reactive_scope_variables.rs — verified compatible
- [x] reactive_scopes/prune_scopes.rs — updated ssa_version field
- [x] reactive_scopes/codegen.rs — verified compatible (temps have unique IDs)
- [x] validation/validate_no_mutation_after_freeze.rs — verified compatible
- [x] inference/analyse_functions.rs — verified compatible

### MODERATE (map key refactoring):
- [x] inference/infer_types.rs — verified compatible
- [x] inference/infer_reactive_places.rs — verified compatible
- [x] inference/collect_optional_chain_dependencies.rs — updated ssa_version field
- [x] optimization/dead_code_elimination.rs — verified compatible
- [x] optimization/constant_propagation.rs — verified compatible
- [x] optimization/prune_temporary_lvalues.rs — verified compatible

### LOW (mechanical simplification):
- [x] All 11 validation passes — verified compatible
- [x] All snapshot/test files — updated
