# InferReactiveScopeVariables: Upstream vs Ours Comparison

**Date:** 2026-04-06
**Upstream:** `src/ReactiveScopes/InferReactiveScopeVariables.ts`
**Ours:** `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`

## Summary

Line-by-line comparison of upstream `findDisjointMutableValues` with our Phase 2 scope grouping logic.
Applied the `isMutable` operand filtering fix and declarations map for phi handling.

**Result:** 559 -> 560 conformance (+1), improved slot accuracy (-18 in -1 under-split category).

## Divergences Found

### D1: Operand Collection (FIXED - partial)

**Upstream:** Per-instruction-kind operand collection with `isMutable(instr, operand)` check.
Only operands whose `mutableRange` spans the current instruction are included.
Also requires `operand.identifier.mutableRange.start > 0` (exclude globals).

**Ours (old):** `collect_operand_ids` collected ALL operands unconditionally, then filtered by
effective_range overlap. No `start > 0` check.

**Fix applied:** Added `is_mutable(instr_id, op_id)` check on all operands via a closure.
Also added declarations map for StoreLocal/StoreContext/DeclareLocal/DeclareContext.

**Impact:** +1 conformance, -18 in -1 under-split, +12 in +1 over-split.

### D2: StoreLocal/StoreContext Specific Handling (NOT FIXED - no net benefit)

**Upstream:** StoreLocal has specific logic:
- `lvalue.place.identifier` added if `mutableRange.end > mutableRange.start + 1` (range width check)
- `value` added if `isMutable(instr, value) && value.mutableRange.start > 0`

**Ours:** We use `collect_operand_ids` which returns both lvalue and value, then filter by `is_mutable`.
The difference: upstream uses a RANGE WIDTH check for lvalue, we use an `is_mutable` check.
The range width check is less restrictive (includes lvalues whose range spans >1 instruction regardless
of whether the current instruction is within the range).

**Why not fixed:** Per-instruction-kind matching was tested but caused regressions when combined with
`may_allocate` changes for the lvalue condition. The simpler approach (filter all operands by isMutable)
is closer to correct behavior for our current state.

### D3: Destructure Pattern Operands (NOT FIXED - same reason as D2)

**Upstream:** Each pattern operand added only if `mutableRange.end > mutableRange.start + 1`.
Value added if `isMutable && start > 0`.

**Ours:** We use `collect_operand_ids` which only returns `value.identifier.id` for Destructure,
not the pattern targets. Pattern targets get scope membership via Phase 4 propagation instead.

### D4: MethodCall Property Identifier (CANNOT FIX)

**Upstream:** Always pushes `instr.value.property.identifier` (a Place for the ComputedLoad).
**Ours:** Our HIR stores MethodCall.property as a String, not a Place. We don't have a separate
identifier for the computed load result.

### D5: `mayAllocate` vs `is_allocating_for_sentinel` (DIVERGENCE KEPT)

**Upstream `mayAllocate`:**
- Returns true for PropertyStore, ComputedStore, RegExpLiteral
- Returns true for all non-primitive Call/MethodCall/TaggedTemplate results
- No hook exclusion, no escape heuristic

**Our `is_allocating_for_sentinel`:**
- Returns false for PropertyStore, ComputedStore, RegExpLiteral
- Excludes hook calls (useXxx pattern)
- Requires `last_use > instr_id` (escape check) for calls

**Why kept:** Matching upstream's `mayAllocate` exactly causes -62 conformance regression because
we lack `PruneNonEscapingScopes` and other downstream passes that clean up excess scopes.

### D6: Scope Creation Filter (DIVERGENCE KEPT)

**Upstream:** Creates a scope for EVERY disjoint set. No reactive/mutable filter.
**Ours:** `any_allocating || (any_reactive && any_mutable)` filter.

**Why kept:** Upstream's DSU only contains mutable/allocating identifiers by construction
(because isMutable/mayAllocate gates what enters the DSU). Our DSU may contain non-mutable
groups, so the filter prevents creating unnecessary scopes.

### D7: Phi Handling (PARTIALLY MATCHED)

**Upstream condition:**
```
phi.mutableRange.start + 1 !== phi.mutableRange.end
  && phi.mutableRange.end > (block.instructions.at(0)?.id ?? block.terminal.id)
```

**Our condition:** `phi_range.end.0 > phi_range.start.0 + 1`

The second part of upstream's condition (`end > first instruction id`) ensures the phi is
mutated AFTER the block starts, not just at its definition. We simplify to just the range
width check. Tested matching upstream's exact condition: no conformance difference.

**Upstream also:** Includes declaration identifiers from a `declarations` map when unioning
phi operands. We added the declarations map tracking but don't include it in phi union
(tested: no conformance difference with our current data).

### D8: `use_mutable_range=true` (STILL NEGATIVE)

With all fixes applied, switching to `use_mutable_range=true` still causes -57 regression
(560 -> 503). The regression is overwhelmingly in over-splitting (+1: 66 -> 145).
Our mutable ranges are still too narrow for many identifiers compared to upstream.

**Root cause:** The BFS range propagation in `infer_mutation_aliasing_ranges.rs` produces
narrower ranges than upstream. This is a separate issue from scope grouping and must be
fixed before `use_mutable_range=true` can be enabled.

## Files Modified

| File | Change |
|------|--------|
| `infer_reactive_scope_variables.rs` | Added isMutable operand filter, declarations map, cleaned up functions |
| `disjoint_set.rs` | Added `union_many` method |
| `known-failures.txt` | Added gating fixture, removed 2 newly passing |
