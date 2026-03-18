# Scope Inference

Scope inference issues. Gaps 8, 9, and 10 are completed. The logical expression flattening bug (codegen-emission.md Gap 5) that caused many "undefined" errors is also fixed. Remaining work is over-memoization (Gap 7).

---

## Gap 7: Over-memoization / Slot Count Divergence

**Priority:** P2 -- over-memoization (correct but wasteful, 8 fixtures affected)

**Current state:** In 8 benchmark fixtures, we create more cache slots than the upstream compiler. This indicates that our reactive scope inference is too aggressive -- we are creating more scopes or tracking more dependencies than necessary, leading to over-memoization. Gap 5 (logical expression flattening) is now fixed, so this can be investigated cleanly.

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

**Evidence:** 8 benchmark fixtures show slot count > upstream. The excess slots don't cause wrong output (over-memoization is correct but wasteful), but they indicate scope inference divergence.

**Depends on:** None (Gap 5 logical expression flattening is now fixed)

---

## Gap 8: Scope Output Variables Not Produced Inside Scope Body ✅ (mostly)

~~**Priority:** P0 -- causes 7 partial_error render failures~~

**Completed** (3 sub-fixes committed): Phase 3b in propagate_dependencies.rs catches unscooped StoreLocal/StoreContext variables. Destructure-in-scope hoisting (Phase 88) moves destructure instructions out of scope bodies. Phantom scope declaration filter (Phase 89) removes spurious declarations. Combined with Gap 5 (logical expression flattening) fix, most "Cannot read properties of undefined" errors are resolved. Remaining edge cases are tracked as individual fixture issues (availability-schedule, canvas-sidebar, booking-list) in codegen-emission.md.

---

## Gap 9: JSX Tag Names Using Temporary Identifiers ✅

~~**Priority:** P1 -- causes 2 render failures, visible corruption~~

**Completed:** Built a global `TagConstantMap` in codegen that recursively walks the entire reactive function tree to find temps assigned `Primitive::String` or `LoadGlobal` values. This map is threaded through all codegen functions (`codegen_block`, `codegen_terminal`, `codegen_scope`, etc.) and consulted during JSX tag resolution in both `codegen_instruction` and the inline `expr_string` path. Fixes `<t15>` → `<button>`, `<t40>` → `<div>`, etc. across all fixtures. Render equivalence improved from 28% (7/25) to 32% (8/25). Rust module: `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`.

---

## Gap 10: Temporal Dead Zone / Initialization Order ✅

~~**Priority:** P1 -- causes 2 render failures~~

**Completed**: Root causes were Gap 5 (logical expression flattening) and Gap 8 (scope output variables). With both fixed, TDZ errors are resolved. Remaining render failures are tracked as individual fixture issues (availability-schedule, canvas-sidebar, booking-list) in codegen-emission.md.
