# Memoization Codegen -- Cache Allocation & Slot Reads/Writes

> The core value proposition of React Compiler: memoizing values in cache slots.
> The codegen infrastructure for `_c(N)` and `$[N]` already exists in `codegen.rs`,
> but the memoization pipeline must produce `ReactiveScopeBlock` nodes for it to activate.

---

## Gap 1: Debug memoization pipeline -- ReactiveScopeBlock generation [~]

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Current state (after partial fix):** Multiple root causes were identified and fixed:

- `infer_reactive_scope_variables` now correctly assigns scopes to identifiers (was discarding them). File: `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`
- `infer_reactive_places` now marks function params as reactive. File: `crates/oxc_react_compiler/src/inference/infer_reactive_places.rs`
- `infer_mutation_aliasing_ranges` now extends ranges to last use (not just last mutation). File: `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs`
- `propagate_scope_dependencies_hir` now populates both dependencies and declarations. File: `crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs`
- `build_reactive_function` now prevents instruction duplication in scope blocks. File: `crates/oxc_react_compiler/src/reactive_scopes/build_reactive_function.rs`

**Remaining work -- scope boundary alignment:**

- Scopes are being created but capture discriminant/marker instructions instead of the actual computation instructions they should wrap
- The scope range (start_id..end_id) from `infer_reactive_scope_variables` needs to correctly bracket the computation, not the control-flow markers
- This likely requires fixes in `build_reactive_scope_terminals_hir` or in how `infer_mutation_aliasing_ranges` computes the range boundaries
- Until this is fixed, memoization output will have scopes in wrong positions (wrapping wrong instructions)

**Depends on:** codegen-correctness.md gaps (all completed)

**Files to investigate:**
- `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs` (build_reactive_scope_terminals_hir)
- `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs` (range computation)
- `crates/oxc_react_compiler/src/reactive_scopes/build_reactive_function.rs` (scope placement)

---

## Gap 2: Verify `$[N]` memoization slot reads/writes are emitted [~]

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Current state (after partial fix):** `propagate_scope_dependencies_hir` now populates both `scope.dependencies` and `scope.declarations`. The codegen path in `codegen_scope()` is confirmed working -- it correctly emits `$[N]` read/write patterns when `ReactiveScopeBlock` nodes have populated deps/decls.

**Remaining:** Same scope boundary alignment issue as Gap 1. Once scopes wrap the correct instructions, the `$[N]` patterns will emit correctly. No separate work needed beyond Gap 1.

**Depends on:** Gap 1 (scope boundary alignment)

---

## Gap 3: End-to-end memoization test ✅

~~**Previous:** No test verifying the full path from React component input to memoized JavaScript output.~~

**Completed**: Added `test_e2e_memoization` snapshot test in `crates/oxc_react_compiler/tests/snapshot_tests.rs`. The test documents the current memoization pipeline state -- scopes are created but boundary alignment is not yet correct. This serves as the regression/progress test for Gaps 1-2. Snapshot: `crates/oxc_react_compiler/tests/snapshots/snapshot_tests__e2e_memoization.snap`.
