# Scope Inference

---

## Gap 7: Over-memoization / Slot Count Divergence

**Priority:** P3 -- over/under-memoization

**Current state:** In 9 out of 16 benchmark fixtures, we create more cache slots than the upstream compiler. This indicates that our reactive scope inference is too aggressive -- we are creating more scopes or tracking more dependencies than necessary, leading to over-memoization.

**What's needed:**

- Compare our scope merging logic against upstream `MergeReactiveScopesThatInvalidateTogether`
- Check if we are failing to merge scopes that should be combined (producing two scopes where upstream produces one)
- Check if we are tracking dependencies that upstream prunes (e.g., stable values like `useState` setters should not be dependencies)
- Verify that our `PruneNonEscapingScopes` pass matches upstream behavior -- scopes for values that don't escape the function should be eliminated
- Audit the scope declaration tracking: are we creating cache slots for values that don't need them?

**Upstream files:**
- `src/ReactiveScopes/MergeReactiveScopesThatInvalidateTogether.ts`
- `src/ReactiveScopes/PruneNonEscapingScopes.ts`
- `src/ReactiveScopes/PropagateScopeDependencies.ts`

**Evidence:** 9/16 benchmark fixtures show slot count > upstream. The excess slots don't necessarily cause wrong output (over-memoization is correct but wasteful), but they indicate scope inference divergence that may also manifest as correctness issues in edge cases.

**Depends on:** P0 codegen fixes should be done first (no point optimizing scope inference if the emission is broken)
