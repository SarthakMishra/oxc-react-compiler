# Scope Inference Gaps

Scope inference issues in reactive scope creation, merging, and pruning passes.

Completed: Gaps 8, 9, 10. Remaining: under-memoization (biggest category) and over-memoization.

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

**Priority:** P1 (404 fixtures have FEWER slots than upstream)

**Current state:** 404 conformance fixtures produce FEWER cache slots than upstream. Deficit ranges from -1 to -23. This is the single largest category of conformance divergence. It means we're either:
1. Not creating scopes that upstream creates
2. Merging scopes too aggressively
3. Pruning scopes that upstream keeps
4. Missing reactive value tracking (so values don't trigger scope creation)

**Slot deficit distribution:**
- -1: 139 fixtures
- -2: 111 fixtures
- -3 to -5: 96 fixtures
- -6 to -10: 41 fixtures
- -11 to -23: 17 fixtures

**What's needed:**
- Pick a few -1 deficit fixtures and diff our output vs upstream to understand root cause
- Most likely issues:
  - Scope dependency tracking misses some reactive values
  - `AlignReactiveScopesToBlockScopes` prunes scopes upstream would keep
  - Missing scope creation for certain expression types (optional chaining, template literals, etc.)

**Upstream:**
- `src/ReactiveScopes/InferReactiveScopeVariables.ts`
- `src/ReactiveScopes/AlignReactiveScopesToBlockScopes.ts`
- `src/ReactiveScopes/PropagateScopeDependencies.ts`

**Depends on:** None

---

## Gap 8: Scope Output Variables Not Produced Inside Scope Body -- COMPLETED

## Gap 9: JSX Tag Names Using Temporary Identifiers -- COMPLETED

## Gap 10: Temporal Dead Zone / Initialization Order -- COMPLETED
