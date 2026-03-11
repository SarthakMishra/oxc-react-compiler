# Memoization Codegen -- Cache Allocation & Slot Reads/Writes

> The core value proposition of React Compiler: memoizing values in cache slots.
> The codegen infrastructure for `_c(N)` and `$[N]` already exists in `codegen.rs`,
> but the memoization pipeline must produce `ReactiveScopeBlock` nodes for it to activate.

---

## Gap 1: Verify `_c(N)` cache allocation is emitted

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Current state:** `codegen_function()` at lines 41-44 of `codegen.rs` already emits `const $ = _c({total_slots});` when `total_slots > 0`. The `count_cache_slots()` function counts slots from `ReactiveScopeBlock` nodes. The `generate_import_statement()` function emits `import { c as _c } from "react/compiler-runtime";`.

The issue is NOT in codegen itself -- it's in the memoization pipeline. If no `ReactiveScopeBlock` nodes are produced, `count_cache_slots` returns 0 and no `_c()` call is emitted.

**What's needed:**

- Debug the full memoization pipeline to determine why `ReactiveScopeBlock` nodes are not appearing in the ReactiveFunction tree for real components
- The pipeline chain is: `infer_reactive_scope_variables` (Pass 33) assigns `ReactiveScope` to identifiers -> `build_reactive_scope_terminals_hir` (Pass 43) creates `Terminal::Scope` nodes in the CFG -> `build_reactive_function` (Pass 47) converts `Terminal::Scope` to `ReactiveScopeBlock` in the tree
- Add diagnostic/debug logging to trace: (a) how many scopes `infer_reactive_scope_variables` creates, (b) how many `Terminal::Scope` nodes `build_reactive_scope_terminals_hir` inserts, (c) how many `ReactiveScopeBlock` nodes `build_reactive_function` emits
- Likely root cause candidates:
  1. `infer_reactive_scope_variables` returns scopes but doesn't assign them to identifier `.scope` fields
  2. `build_reactive_scope_terminals_hir` doesn't find the scopes because the range/instruction mapping is off
  3. `find_scope_in_block` in `build_reactive_function.rs` fails to match the scope ID and falls back to emitting instructions without scope wrapping (line 197-202)
  4. Pruning passes (49-53) are too aggressive and remove all scopes

**Depends on:** Gap 1 of codegen-correctness.md (fixing variable names so output is parseable for debugging)

**Files to investigate:**
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs` (build_reactive_scope_terminals_hir)
- `crates/oxc_react_compiler/src/reactive_scopes/build_reactive_function.rs` (find_scope_in_block)

---

## Gap 2: Verify `$[N]` memoization slot reads/writes are emitted

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Current state:** `codegen_scope()` at lines 370-463 of `codegen.rs` already emits the full memoization pattern:

```js
if ($[0] !== dep1 || $[1] !== dep2) {
  // scope body
  $[2] = computedValue;
  $[0] = dep1;
  $[1] = dep2;
} else {
  computedValue = $[2];
}
```

This code runs when `ReactiveScopeBlock` nodes exist with populated `scope.dependencies` and `scope.declarations`.

**What's needed:**

- Same root cause as Gap 1 -- if no `ReactiveScopeBlock` nodes exist, no `$[N]` code is emitted
- Additionally verify that `propagate_scope_dependencies_hir` (Pass 46) correctly populates `ReactiveScope.dependencies` -- without dependencies, the scope guard would use sentinel checks only
- Verify that `ReactiveScope.declarations` is populated -- without declarations, no values are stored in or loaded from cache slots

**Depends on:** Gap 1 (same investigation)

**Files to investigate:**
- `crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` (lines 370-463)

---

## Gap 3: End-to-end memoization test

**Upstream:** Fixture tests in `src/__tests__/fixtures/compiler/`
**Current state:** No test that verifies the full path from React component input to memoized JavaScript output with `_c(N)` and `$[N]` patterns.

**What's needed:**

- Add a snapshot test that compiles a simple component like:
  ```jsx
  function Counter() {
    const [count, setCount] = useState(0);
    return <div>{count}</div>;
  }
  ```
  and verifies the output contains:
  - `const $ = _c(N)` for some N > 0
  - At least one `$[N] !== ...` dependency check
  - At least one `$[N] = ...` cache store
  - At least one `... = $[N]` cache load
- This test will initially fail and serve as the acceptance criterion for Gaps 1-2

**Depends on:** None (can be written as a failing test immediately)
