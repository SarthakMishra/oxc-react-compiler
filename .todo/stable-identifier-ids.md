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

## Status: REVERTED (2026-03-16)

The initial implementation (commit `0b8b792`) was reverted in `f161523` because
the flat `binding_ids: FxHashMap<String, IdentifierId>` registry does not handle
JavaScript scoping correctly:

- **Shadowing**: `let x = 1; { let x = 2; }` — inner `x` gets the same ID as
  outer `x`, conflating two distinct bindings
- **Block scope**: Variables declared in different blocks but with the same name
  share IDs when they shouldn't
- **Closures**: Nested function builders create fresh `HIRBuilder` instances with
  empty registries, but `setup_context_variables()` is never called, so captured
  variables from outer scope get fresh IDs anyway

### What's needed to retry

The `binding_ids` registry must be **scope-aware** instead of a flat name map.
Options:

1. **Scope stack approach**: Push/pop scope frames when entering/leaving blocks.
   Each frame has its own name→ID map. Lookup walks the stack for the innermost
   binding. This matches JavaScript's lexical scoping.

2. **OXC semantic info approach**: Use the binding information already computed by
   `oxc_semantic::SemanticBuilder` (which resolves all scopes and bindings) and
   thread it through to the HIR builder. Each OXC `SymbolId` maps to exactly one
   binding — use it as the stable ID.

3. **DeclarationId-based approach**: The HIR already has `DeclarationId` on
   declaration sites. Extend the builder to record `name → DeclarationId` per
   scope, and when creating a reference, look up the binding's DeclarationId to
   find the corresponding IdentifierId.

Option 2 is the most robust (OXC already solved scoping) but requires threading
semantic info through the builder. Option 1 is self-contained but must handle
all JS scoping rules correctly.

### Lessons from the first attempt

- The SSA rewrite (enter_ssa.rs, eliminate_redundant_phi.rs) was correct and
  could be reused — the ssa_version field approach works
- Downstream passes (inference, reactive scopes, validation, codegen) were
  genuinely compatible — no changes needed
- The ONLY issue was in Phase 1: the flat binding_ids registry
- The conformance test's temp normalization (`tN` → sequential renaming) already
  handles renumbered temps correctly

## Files Checklist (40+ files)

### CRITICAL (must redesign):
- [ ] hir/types.rs — add `ssa_version: u32` to Identifier (was done, reverted)
- [ ] hir/build.rs — add **scope-aware** binding registry (flat map was wrong)
- [ ] ssa/enter_ssa.rs — rewrite to use ssa_version (was done, reverted)
- [ ] ssa/eliminate_redundant_phi.rs — use (IdentifierId, ssa_version) tuples (was done, reverted)
- [x] inference/infer_mutation_aliasing_effects.rs — verified compatible (no changes needed)

### IMPORTANT (algorithm updates):
- [x] inference/infer_mutation_aliasing_ranges.rs — verified compatible
- [x] reactive_scopes/propagate_dependencies.rs — verified compatible
- [x] reactive_scopes/merge_scopes.rs — verified compatible
- [x] reactive_scopes/infer_reactive_scope_variables.rs — verified compatible
- [ ] reactive_scopes/prune_scopes.rs — needs ssa_version field update
- [x] reactive_scopes/codegen.rs — verified compatible (temps have unique IDs)
- [x] validation/validate_no_mutation_after_freeze.rs — verified compatible
- [x] inference/analyse_functions.rs — verified compatible

### MODERATE (map key refactoring):
- [x] inference/infer_types.rs — verified compatible
- [x] inference/infer_reactive_places.rs — verified compatible
- [ ] inference/collect_optional_chain_dependencies.rs — needs ssa_version field update
- [x] optimization/dead_code_elimination.rs — verified compatible
- [x] optimization/constant_propagation.rs — verified compatible
- [x] optimization/prune_temporary_lvalues.rs — verified compatible

### LOW (mechanical simplification):
- [x] All 11 validation passes — verified compatible
- [ ] All snapshot/test files — need updating after re-implementation
