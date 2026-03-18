# Scope Inference

These issues cause the remaining 76% render failures (19/25 pairs). The common root cause is scope declarations not matching what is actually computed inside scope bodies -- variables are declared as scope outputs but the producing instruction lives in a different scope (or outside any scope), so the variable is uninitialized or undefined at runtime.

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

**Depends on:** Gap 8 should be investigated first (it causes runtime crashes, not just waste). Gap 9 is resolved.

---

## Gap 8: Scope Output Variables Not Produced Inside Scope Body

**Priority:** P0 -- causes 10+ render failures

**Current state:** The most common class of remaining render failure is a variable being listed as a scope's output (declared) but the instruction that produces it lives in a different scope or outside any scope entirely. At runtime, the variable is never assigned inside the scope's `if ($[N] !== ...)` block, so it remains `undefined` when read after the scope.

**Symptoms (from render test errors):**
- 3x `remaining is not defined` -- variable computed outside its declared scope
- 3x `Cannot read properties of undefined (reading 'length'/'filter')` -- scope output never assigned, downstream code reads `.length` on undefined
- 1x `undefined is not iterable` -- scope output used in destructuring/spread but never assigned
- 1x `editingId is not defined` -- variable missing from scope body
- 1x `Cannot read properties of undefined (reading 'localeCompare')` -- scope output is undefined
- 1x `Cannot read properties of undefined (reading '0')` -- array scope output is undefined

**What's needed:**

- Audit `InferReactiveScopeVariables` -- when a variable is assigned to a scope, verify that the instruction producing it (StoreLocal, Destructure, etc.) is actually inside that scope's mutable range
- Audit `BuildReactiveFunction` -- when constructing the ReactiveFunction tree, verify that scope bodies contain all instructions that write to scope output variables
- Check if the scope's `mutable_range` correctly covers the instruction that produces each declared output
- Compare against upstream `PropagateScopeDependencies` which explicitly validates that scope outputs are produced within the scope
- May need to add a validation pass that checks: for every scope output variable, the defining instruction is inside the scope body

**Upstream files:**
- `src/ReactiveScopes/InferReactiveScopeVariables.ts`
- `src/ReactiveScopes/BuildReactiveBlocks.ts`
- `src/ReactiveScopes/PropagateScopeDependencies.ts`

**Depends on:** None -- this is the highest priority item

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

**Depends on:** Gap 8 (some of these may be caused by the same scope output misplacement issue)
