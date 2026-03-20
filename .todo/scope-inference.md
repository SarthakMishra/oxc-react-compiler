# Scope Inference Gaps

Scope inference issues in reactive scope creation, merging, and pruning passes.

Completed: Gaps 8, 9, 10 (plus isMutable fix, PropagateScopeMembershipHIR pass, separated last_use from mutable_range, Steps 1-6 aliasing effect pipeline). Remaining: Gap 7 (over-memoization, ~175 fixtures) and Gap 11 (under-memoization, ~404 fixtures — foundational blocker). Four attempts to narrow mutable ranges all reverted (96%/88% → 36% render regression each time).

---

## Gap 7: Over-memoization / Slot Count Divergence

**Priority:** P1 (part of 622-fixture scope divergence)

**Current state:** ~175 conformance fixtures produce MORE cache slots than upstream. Slot excess ranges from +1 to +42. This means we create scopes that upstream doesn't, or fail to merge scopes that upstream combines.

**What's needed:**
- Compare scope merging logic against upstream `MergeReactiveScopesThatInvalidateTogether`
- Check if we fail to merge scopes that should be combined
- Check if we track dependencies that upstream prunes (e.g., stable `useState` setters)
- Verify `PruneNonEscapingScopes` matches upstream behavior

**Upstream:**
- `src/ReactiveScopes/MergeReactiveScopesThatInvalidateTogether.ts`
- `src/ReactiveScopes/PruneNonEscapingScopes.ts`
- `src/ReactiveScopes/PropagateScopeDependencies.ts`

**Depends on:** None

---

## Gap 11: Under-memoization / Missing Scopes

**Priority:** P1 (~404 fixtures have FEWER slots than upstream) — FOUNDATIONAL BLOCKER

**Root cause:** Our BFS mutation propagation produces narrower mutation ranges than upstream's full abstract interpreter. We compensate with `effective_range = max(mutable_range.end, last_use + 1)`, but this is an approximation that still leaves ~404 fixtures under-memoized.

**Slot deficit distribution:**
- -1: ~136 fixtures
- -2: ~118 fixtures
- -3 to -5: ~96 fixtures
- -6 to -23: ~54 fixtures

**What's needed (ordered by dependency):**
1. Port upstream's full abstract interpreter state machine from `src/Inference/InferMutationAliasingEffects.ts` (~2000 lines)
2. Once BFS produces sufficiently wide ranges, switch from `effective_range` to `mutable_range`
3. This also unblocks validation relaxation (Gap 5a: +58 fixtures)

**Why this unblocks everything:**
- Fixes the ~404 under-memoization fixtures directly
- Unblocks validation relaxation (Gap 5a)
- May also reduce over-memoization (Gap 7) as a side effect

**Upstream:**
- `src/ReactiveScopes/InferReactiveScopeVariables.ts`
- `src/Inference/InferMutationAliasingEffects.ts`

**Depends on:** None, but extremely high risk — DO NOT attempt without ability to A/B test with render comparison
