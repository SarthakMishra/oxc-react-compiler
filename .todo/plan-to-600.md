# Plan: 422 → 600+ Conformance

Current: 422/1717 (24.6%) | Target: 600/1717 (34.9%) | Need: +178

## Failure Breakdown (1295 divergences)

| Category | Count | Description |
|---|---|---|
| We bail, they compile | 215 | False-positive validations |
| We compile, they don't | 123 | Over-memoization / missing validations |
| Both compile, slots MATCH | 270 | Correct slots, codegen structure differs |
| Both compile, slots DIFFER | 584 | Scope analysis produces wrong boundaries |
| Both no memo, format diff | 94 | Neither memoizes, output formatting differs |

---

## Phase 1: Frozen-Mutation Validator — ID-Based Tracking (~13 fixtures)

**Status**: 33 remaining false positives after effect/callback exemption
**Root cause**: `frozen_names: FxHashSet<&str>` conflates all SSA versions of the same variable

### What upstream does
Upstream tracks frozen status on `InstructionValue` (allocation-site) objects, not names.
Each `IdentifierId` has a points-to set of allocation sites. Reassignment (`x = []`) creates
a new allocation site — the old frozen one is unchanged, the new one is mutable.

### Implementation
Replace name-based tracking with ID-based tracking in `validate_no_mutation_after_freeze.rs`:

1. Replace `frozen_names: FxHashSet<&str>` with `frozen_ids: FxHashSet<IdentifierId>`
2. When freezing (JSX usage, hook return, Freeze effect), add the specific `IdentifierId`
3. When checking mutations, check the operand's `IdentifierId` against `frozen_ids`
4. SSA guarantees reassigned variables get new IDs — naturally solves the conflation
5. Keep name-based resolution as fallback for cross-block phi references

**Fixtures fixed**: reassignment-separate-scopes, switch (x4), same-variable-as-dep-and-redeclare (x2),
component.js, reactive-scopes.js — ~10-13 fixtures

**Risk**: Low. ID-based is strictly more precise than name-based. No false negatives.

---

## Phase 2: IIFE Inlining with Captures (~8 fixtures)

**Status**: 8 IIFE fixtures remain as frozen-mutation false positives
**Root cause**: Our `inline_iife.rs` only handles trivial IIFEs (no captures, no args)

### What upstream does
Upstream's `InlineImmediatelyInvokedFunctionExpressions` runs BEFORE mutation analysis.
It handles IIFEs WITH captures by splicing the function body into the parent CFG.
Context variables are already in scope (they're captures), so no special handling needed.
Only excludes functions with parameters, async, or generators.

### Implementation
Extend `inline_iife.rs` to handle IIFEs with captures:

1. Detect pattern: `FunctionExpression` → immediate `CallExpression` with 0 args
2. If the function has `context` (captures) but no `params`, it's inlinable
3. Splice the IIFE body blocks into the parent CFG:
   - Replace `Return` terminals with `StoreLocal` to the call's lvalue + `Goto` to continuation
   - Context variables are already in parent scope — no alpha-renaming needed
4. Remove the original FunctionExpression + CallExpression instructions
5. Run BEFORE `infer_mutation_aliasing_effects` (Phase 5 in pipeline)

**Fixtures fixed**: capturing-func-alias-*-iife.js (x8)

**Risk**: Medium. IIFE inlining affects the CFG structure. Need to handle:
- Multiple `Return` paths (if/else inside IIFE)
- Nested IIFEs (recurse)
- Side effects in IIFE body that reference outer mutable variables

---

## Phase 3: Scope Inference — `isMutable(instr, operand)` Check (~100-150 fixtures)

**Status**: Our scope inference uses range-overlap heuristic instead of per-instruction mutability
**Root cause**: Fundamental architectural gap in `infer_reactive_scope_variables.rs`

### What upstream does
Upstream's `InferReactiveScopeVariables.ts` unions lvalue with operands only when
`isMutable(instr, operand)` returns true — meaning the instruction ID falls within
the operand's `mutableRange`. This is a point-in-time check, not a range-overlap check.

### Our current behavior
We check `op_range.end > op_range.start + 1` (non-trivial range) and union unconditionally.
This over-unions: an operand might have a wide mutable range but NOT be mutable at this
specific instruction's position.

### Implementation
In Phase 2 of `infer_reactive_scope_variables.rs`:

```rust
// BEFORE (wrong):
if op_range.end.0 > op_range.start.0 + 1 {
    dsu.union(lvalue_id, op_id);
}

// AFTER (correct):
let instr_id = instr.lvalue.identifier.mutable_range.start; // instruction position
if op_range.start.0 <= instr_id.0 && instr_id.0 < op_range.end.0 {
    dsu.union(lvalue_id, op_id);
}
```

Also need to check lvalue mutability: only union when the LVALUE itself is mutable at this instruction.

**Fixtures fixed**: Estimated ~100-150 across slot-differ and same-slot categories.
This is the single highest-impact scope analysis fix.

**Risk**: Medium-High. Changes scope boundaries globally. Needs careful testing.
Run conformance before/after and verify no regressions in passing fixtures.

---

## Phase 4: Codegen — Reassignment Cache Slots (~40-60 fixtures)

**Status**: We don't allocate cache slots for `scope.reassignments`
**Root cause**: Missing feature in `codegen.rs`

### What upstream does
For each reactive scope, upstream allocates cache slots for:
1. N slots for dependencies (dep-check values)
2. M slots for declarations (values produced inside scope, used outside)
3. K slots for reassignments (variables declared outside scope, reassigned inside)

Reassignment slots allow the cache to store/restore variables that are modified inside a scope
but whose declaration is outside. Without these, the variable loses its updated value on cache hit.

### Implementation
In `codegen.rs`:

1. In `count_cache_slots`, add `scope.reassignments.len()` to the total
2. In scope codegen, after storing declarations to cache, store reassignment values
3. In the `else` branch (cache hit), restore reassignment values from cache
4. Track reassignment identifiers in the scope during `propagate_dependencies`

**Fixtures fixed**: Estimated ~40-60 fixtures where scope structure matches but
cache slot count or restore logic differs.

**Risk**: Low-Medium. Adding cache slots is additive — it can't break existing behavior.

---

## Phase 5: Codegen — Declaration Sorting (~50-80 fixtures)

**Status**: We iterate declarations in insertion order; upstream sorts deterministically
**Root cause**: Non-deterministic declaration ordering in scope codegen

### What upstream does
`CodegenReactiveFunction.ts` uses `compareScopeDeclaration()` to sort declarations.
This ensures deterministic slot indices: declarations are ordered by their position
in the source code (by `DeclarationId` or source location).

### Implementation
In `codegen.rs`, before emitting scope declarations, sort them by source location:

```rust
scope.declarations.sort_by_key(|(_, decl)| decl.identifier.loc.start);
```

**Fixtures fixed**: Estimated ~50-80 fixtures where the only difference is declaration order.

**Risk**: Very low. Pure ordering change.

---

## Phase 6: Dependency Propagation — Inner Function Crawling (~30-50 fixtures)

**Status**: We don't recurse into FunctionExpression/ObjectMethod bodies for dep collection
**Root cause**: Missing traversal in `propagate_dependencies.rs`

### What upstream does
Upstream's `PropagateScopeDependenciesHIR.ts` uses `enterInnerFn()` to recursively
enter inner function bodies and extract dependencies that close over outer scope variables.
This ensures that if a scope contains a closure that references `props.x`, the scope
gets `props.x` as a dependency.

### Implementation
In Phase 2 of `propagate_dependencies.rs`:

1. When encountering `InstructionValue::FunctionExpression { lowered_func, .. }` inside a scope
2. Recurse into `lowered_func.body` to find operands that reference outer-scope variables
3. Add those as dependencies of the current scope
4. Be careful to exclude variables declared inside the inner function (local to it)

**Fixtures fixed**: Estimated ~30-50 fixtures with closures as scope outputs.

**Risk**: Medium. Needs correct scoping to avoid false dependencies.

---

## Phase 7: Scope Inference — Phi Node Refinement (~30-50 fixtures)

**Status**: We unconditionally union all phi operands; upstream only unions mutated phis
**Root cause**: Over-unioning in `infer_reactive_scope_variables.rs`

### What upstream does
Upstream only unions phi operands when the phi's `mutableRange.end` extends past the
first instruction in the block — meaning the phi value is mutated after creation.
Non-mutated phis (simple value selection) should NOT be unioned because their operands
come from different control flow paths and may need separate scopes.

### Implementation
In Phase 2 of `infer_reactive_scope_variables.rs`, change phi handling:

```rust
for phi in &block.phis {
    let phi_id = phi.place.identifier.id;
    let phi_range = phi.place.identifier.mutable_range;
    // Only union if the phi is mutated after creation
    let first_instr_id = block.instructions.first()
        .map(|i| i.lvalue.identifier.mutable_range.start)
        .unwrap_or(InstructionId(u32::MAX));
    if phi_range.end.0 > first_instr_id.0 {
        for (_, operand) in &phi.operands {
            dsu.make_set(operand.identifier.id);
            dsu.union(phi_id, operand.identifier.id);
        }
    }
}
```

**Fixtures fixed**: Estimated ~30-50 fixtures with conditional value selection.

**Risk**: Medium. Affects scope boundaries for if/else patterns.

---

## Phase 8: Dependency Propagation — Control Dependency Analysis (~15-20 fixtures)

**Status**: Phi outputs from reactive conditions are not marked reactive
**Root cause**: No control dependency tracking

### What upstream does
Upstream's `InferReactivePlaces` explicitly tracks control dependencies: if a phi
node's value depends on a reactive branch condition (e.g., `props.cond`), the phi
output is marked reactive even if both assigned values are constants.

### Implementation
Add a pass after SSA that marks phi outputs as reactive when their controlling
branch condition is reactive:

1. For each phi node, find the predecessor blocks' terminal conditions
2. If any condition is reactive (uses a param/hook-derived value), mark the phi reactive
3. This feeds into `infer_reactive_scope_variables` which creates dep-checked scopes

**Fixtures fixed**: reactive-control-dependency-*.js (11 fixtures) + related patterns.

**Risk**: Medium. Requires understanding the CFG structure to trace control flow.

---

## Phase 9: Over-Memoization Control (~40-50 fixtures)

**Status**: We compile/memoize functions that upstream returns unchanged
**Root cause**: Missing directive handling and over-eager scope creation

### Implementation
1. **`@expectNothingCompiled` support**: Parse test annotation, skip compilation entirely (~20)
2. **Non-reactive-only scope pruning**: If a scope has no reactive deps AND all its values
   are primitives, prune it (upstream's `PruneNonEscapingScopes` handles some of this) (~15)
3. **`compilationMode: 'annotation'`/`'syntax'` support**: Only compile functions with
   `'use memo'` directive when these modes are specified (~10)

**Risk**: Low. These are additive checks that reduce false compilation.

---

## Estimated Impact Summary

| Phase | Description | Est. Fixtures | Cumulative |
|---|---|---|---|
| 1 | ID-based freeze tracking | +10-13 | 432-435 |
| 2 | IIFE inlining with captures | +8 | 440-443 |
| 3 | `isMutable` scope inference | +40-80 | 480-523 |
| 4 | Reassignment cache slots | +20-40 | 500-563 |
| 5 | Declaration sorting | +20-40 | 520-603 |
| 6 | Inner function dep crawling | +15-30 | 535-633 |
| 7 | Phi node refinement | +15-30 | 550-663 |
| 8 | Control dependency analysis | +10-15 | 560-678 |
| 9 | Over-memoization control | +20-30 | 580-708 |

**Conservative estimate**: 580 | **Optimistic estimate**: 708

Note: Fixture counts overlap — many fixtures are affected by multiple gaps.
The conservative estimate accounts for this overlap.

---

## Implementation Order (Recommended)

**Easy wins first** (phases 1, 5, 9a) — low risk, clear implementation:
- Phase 1: ID-based freeze tracking (1-2 sessions)
- Phase 5: Declaration sorting (< 1 session)
- Phase 9a: @expectNothingCompiled (< 1 session)

**Core architecture** (phases 3, 7) — highest impact, needs careful testing:
- Phase 3: isMutable scope inference (2-3 sessions)
- Phase 7: Phi node refinement (1-2 sessions)

**Feature additions** (phases 4, 6, 8) — additive, moderate risk:
- Phase 4: Reassignment cache slots (1-2 sessions)
- Phase 6: Inner function dep crawling (1-2 sessions)
- Phase 8: Control dependency analysis (2-3 sessions)

**Structural changes** (phase 2) — affects CFG, needs most care:
- Phase 2: IIFE inlining (2-3 sessions)

---

## Beyond 600: What Gets Hard

To go from 600 to 1000+, the remaining gaps are:
- **Abstract interpreter for mutable ranges** — upstream uses a full points-to analysis
  with allocation-site abstraction. Our BFS-based approach fundamentally differs.
- **Full type system** — upstream has built-in type signatures for all React hooks,
  DOM methods, and common libraries. We use name-based heuristics.
- **Optional chaining scope semantics** — upstream has dedicated passes for `?.` chains.
- **Lambda lifting / function outlining** — upstream outlines inner functions differently.
- **Flow fixtures** (38) — permanently blocked by OXC parser limitation.

These require major architectural work and are unlikely to be cost-effective for a POC.
