# Scope Inference Gaps

Scope inference issues in reactive scope creation, merging, and pruning passes.

Completed: Gaps 8, 9, 10 (plus isMutable fix in Phase 93). Remaining: Gap 7 (over-memoization, 175 fixtures) and Gap 11 (under-memoization, 404 fixtures -- foundational blocker).

---

## Gap 7: Over-memoization / Slot Count Divergence

**Priority:** P1 (part of 622-fixture scope divergence)

**Current state:** 175 conformance fixtures produce MORE cache slots than upstream. Slot excess ranges from +1 to +13. This means we create scopes that upstream doesn't, or fail to merge scopes that upstream combines.

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

**Priority:** P1 (404 fixtures have FEWER slots than upstream) -- THIS IS THE FOUNDATIONAL BLOCKER

**Root cause identified:** The `last_use_map` in `InferReactiveScopeVariables` tracks uses too broadly, which prevents scope creation for values that should be independently memoized. This was confirmed during the Priority #3 investigation which yielded +1 conformance by fixing `isMutable` checks in operand union (`795e340`).

**Current state:** 404 conformance fixtures produce FEWER cache slots than upstream. Deficit ranges from -1 to -23. This is the single largest category of conformance divergence AND it blocks validation relaxation (proven by the reverted Priority #4 attempt).

**Slot deficit distribution:**
- -1: 139 fixtures
- -2: 111 fixtures
- -3 to -5: 96 fixtures
- -6 to -10: 41 fixtures
- -11 to -23: 17 fixtures

**What's needed (ordered by dependency):**
1. Remove `last_use_map` mechanism and replace with upstream's approach to scope variable inference
2. Implement missing `PropagateScopeDependenciesHIR` pass (upstream has this as a separate pre-pass)
3. Audit `AlignReactiveScopesToBlockScopes` against upstream for over-pruning
4. Re-validate scope merging logic in `MergeReactiveScopesThatInvalidateTogether`

**Why this unblocks everything:**
- Fixes the 404 under-memoization fixtures directly
- Unblocks validation relaxation (Gap 5a) by ensuring correct scopes before we relax checks
- May also reduce over-memoization (Gap 7) as a side effect

**Upstream:**
- `src/ReactiveScopes/InferReactiveScopeVariables.ts`
- `src/ReactiveScopes/AlignReactiveScopesToBlockScopes.ts`
- `src/ReactiveScopes/PropagateScopeDependencies.ts`
- `src/ReactiveScopes/PropagateScopeDependenciesHIR.ts`

**Depends on:** None

---

## Gap 8: Scope Output Variables Not Produced Inside Scope Body -- COMPLETED

## Gap 9: JSX Tag Names Using Temporary Identifiers -- COMPLETED

## Gap 10: Temporal Dead Zone / Initialization Order -- COMPLETED
