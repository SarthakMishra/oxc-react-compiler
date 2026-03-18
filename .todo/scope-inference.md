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

## Gap 8: Scope Output Variables Not Produced Inside Scope Body (partially fixed)

**Priority:** P0 -- causes 8+ render failures (down from 10+)

**Current state:** Partially addressed by Phase 3b in propagate_dependencies.rs, which catches StoreLocal/StoreContext variables that are unscooped (no reactive scope assigned during InferReactiveScopeVariables) but enclosed by a scope's block range in the HIR CFG. These variables now get declared as scope outputs, enabling hoisting and caching. This fixed the `remaining is not defined` class of errors for the avatar-group fixture.

**Remaining issues:**
- Destructure instructions (e.g., `[todos, setTodos] = useState(...)`) inside scope bodies: the destructure pattern targets are not properly wired as scope outputs. Adding them causes slot misalignment because the Destructure instruction's temp lvalue is already a declaration. Needs a deeper fix in codegen to replace temp declarations with named pattern targets for Destructure instructions.
- 3x `Cannot read properties of undefined (reading 'length'/'filter')` -- scope output never assigned (likely Destructure-related)
- 1x `undefined is not iterable` -- Destructure-in-scope issue
- 1x `Cannot read properties of undefined (reading 'localeCompare')` -- scope output is undefined
- 1x `Cannot read properties of undefined (reading '0')` -- array scope output is undefined

**What's still needed:**

- Fix Destructure-in-scope handling: when a Destructure instruction has a scoped lvalue, the codegen should store/load the pattern targets (the actual variables) rather than the meaningless temp lvalue
- Ensure the scope's cache slot count reflects the actual named outputs, not internal temps
- Consider adding a pre-codegen pass that rewires Destructure scope declarations

**Upstream files:**
- `src/ReactiveScopes/InferReactiveScopeVariables.ts`
- `src/ReactiveScopes/BuildReactiveBlocks.ts`
- `src/ReactiveScopes/PropagateScopeDependencies.ts`

**Depends on:** None -- this remains high priority

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
