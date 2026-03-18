# Scope Inference

These issues cause scope output variables to be uninitialized or misaligned at runtime. Many of the "Cannot read properties of undefined" errors are a combination of this bug and the logical expression flattening bug (see codegen-emission.md Gap 5).

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

**Depends on:** Gap 5 (logical expression flattening) should be fixed first -- it affects scope output analysis.

---

## Gap 8: Scope Output Variables Not Produced Inside Scope Body (partially fixed)

**Priority:** P0 -- causes 7 partial_error render failures

**Current state:** Partially addressed by multiple fixes:
- Phase 3b in propagate_dependencies.rs catches unscooped StoreLocal/StoreContext variables
- Destructure-in-scope hoisting (Phase 88) moves destructure instructions out of scope bodies
- Phantom scope declaration filter (Phase 89) removes spurious declarations

**Remaining issues (updated based on deep analysis):**

Many of the "Cannot read properties of undefined" errors are actually caused by **Gap 5 (logical expression flattening)** rather than scope output misplacement. When `??` and `&&` are flattened, the right-branch value unconditionally overwrites the left-branch value, producing wrong types that then crash on method calls like `.filter()`, `.length`, etc.

After Gap 5 is fixed, the remaining scope output issues to investigate:
- Variables produced by `useMemo`/`useCallback` return values that are cached in scopes but never assigned in the computation branch (only in the else/reload branch)
- Example from data-table.oxc.js: `sortedData = t125;` where `t125` is assigned in the `else` branch but not the `if` branch of the scope guard

**What's still needed:**

- Fix Gap 5 first (logical flattening), then re-evaluate which "undefined" errors remain
- For remaining cases: ensure that when a variable is produced by a hook call (useMemo, useCallback) inside a scope, both the computation path and the reload path assign it

**Upstream files:**
- `src/ReactiveScopes/InferReactiveScopeVariables.ts`
- `src/ReactiveScopes/BuildReactiveBlocks.ts`
- `src/ReactiveScopes/PropagateScopeDependencies.ts`

**Depends on:** Gap 5 (logical expression flattening) -- many "undefined" crashes will resolve when short-circuit semantics are restored

---

## Gap 9: JSX Tag Names Using Temporary Identifiers ✅

~~**Priority:** P1 -- causes 2 render failures, visible corruption~~

**Completed:** Built a global `TagConstantMap` in codegen that recursively walks the entire reactive function tree to find temps assigned `Primitive::String` or `LoadGlobal` values. This map is threaded through all codegen functions (`codegen_block`, `codegen_terminal`, `codegen_scope`, etc.) and consulted during JSX tag resolution in both `codegen_instruction` and the inline `expr_string` path. Fixes `<t15>` → `<button>`, `<t40>` → `<div>`, etc. across all fixtures. Render equivalence improved from 28% (7/25) to 32% (8/25). Rust module: `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`.

---

## Gap 10: Temporal Dead Zone / Initialization Order

**Priority:** P1 -- causes 2 render failures

**Current state:** Some compiled output has variables accessed before their initialization within the same scope, triggering TDZ errors. This is different from Gap 3 (which was cross-scope ordering, now fixed) -- these are within-scope ordering issues where the cache reload path reads a variable before the computation path has a chance to initialize it.

**Symptoms:**
- 1x `Cannot access 't38' before initialization`
- 1x `Cannot access 'handleSubmit' before initialization`
- 1x `t60 is not defined`

**What's needed:**

- Check if the scope's cache reload path (`else` branch of the guard) reads a variable that is only declared further down in the function
- Verify that pre-declaration hoisting covers all variables that appear in cache reload paths, not just scope output variables
- May need to hoist additional temporaries that are used in scope reload logic but defined inside scope bodies
- Check if instruction ordering within scope bodies matches the original source order

**Upstream files:**
- `src/ReactiveScopes/CodegenReactiveFunction.ts`

**Depends on:** Gap 5 (some TDZ errors may be caused by logical expression flattening producing unexpected variable references)
