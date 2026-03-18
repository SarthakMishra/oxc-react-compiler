# Scope Inference

Scope inference issues. Gaps 8, 9, and 10 are completed. Remaining work is over-memoization (Gap 7).

---

## Gap 7: Over-memoization / Slot Count Divergence

**Priority:** P3 -- over-memoization (correct but wasteful, 8 fixtures affected)

**Current state:** In 8 benchmark fixtures, we create more cache slots than the upstream compiler. This indicates our reactive scope inference is too aggressive -- creating more scopes or tracking more dependencies than necessary.

**What's needed:**
- Compare scope merging logic against upstream `MergeReactiveScopesThatInvalidateTogether`
- Check if we fail to merge scopes that should be combined
- Check if we track dependencies that upstream prunes (e.g., stable `useState` setters)
- Verify `PruneNonEscapingScopes` matches upstream behavior
- Audit scope declaration tracking for unnecessary cache slots

**Upstream:**
- `src/ReactiveScopes/MergeReactiveScopesThatInvalidateTogether.ts`
- `src/ReactiveScopes/PruneNonEscapingScopes.ts`
- `src/ReactiveScopes/PropagateScopeDependencies.ts`

**Evidence:** 8 fixtures show slot count > upstream. Excess slots don't cause wrong output but indicate scope inference divergence.

**Depends on:** None

---

## Gap 8: Scope Output Variables Not Produced Inside Scope Body ✅

**Completed**: Phase 3b in propagate_dependencies.rs, destructure-in-scope hoisting, phantom scope declaration filter.

---

## Gap 9: JSX Tag Names Using Temporary Identifiers ✅

**Completed**: Built global `TagConstantMap` in codegen. Render equivalence improved from 28% to 32%.

---

## Gap 10: Temporal Dead Zone / Initialization Order ✅

**Completed**: Root causes were Gap 5 (logical expression flattening) and Gap 8 (scope output variables).
