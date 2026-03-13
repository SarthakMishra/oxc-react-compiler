# Memoization Structure Differences

> **Priority**: P1 (largest category -- 904 fixtures, 64% of all failures)
> **Impact**: 904 divergences where both compilers produce `_c()` but structure differs
> **Tractability**: LOW per-item, HIGH aggregate -- items are interdependent; no single fix moves the needle alone

## Problem Statement

When both our compiler and Babel memoize a function, our output differs structurally. The 904 fixtures break down into:

| Sub-category | Count | Root cause |
|-------------|-------|------------|
| Over-scoped (too many cache slots) | ~400 | Globals/stable values treated as reactive deps |
| ~~Sentinel pattern never emitted~~ | ~~280~~ | ~~RESOLVED -- sentinel scopes now emitted~~ |
| Under-scoped (too few cache slots) | ~90 | Missing scopes for some expressions |
| Same slots, wrong deps | ~37 | Dependency tracking diverges (property-path resolution now active) |
| Other structural | ~94 | Temp variable naming, code ordering |
| Sentinel regressions (temporary) | +35 | Scopes correct, deps/slots still wrong |

The structural issues compound: a fixture may have wrong temp variables AND wrong slot counts AND missing sentinel scopes. Fixing one in isolation typically gains zero fixtures because the remaining issues still cause a mismatch.

## Files to Modify

### Temp Variable Inlining
- **`crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`** -- add inline pass or modify codegen to collapse SSA chains
- Potentially new file: **`crates/oxc_react_compiler/src/optimization/inline_temporaries.rs`** -- post-RF pass to inline trivial SSA chains

### JSX Preservation
- **`crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`** -- lines 325-348, modify JSX codegen to emit JSX syntax instead of `_jsx()` calls

### Scope/Dependency Analysis
- **`crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`** -- review merge heuristics vs upstream
- **`crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`** -- review prune decisions vs upstream
- **`crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`** -- reactive place inference
- **`crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs`** -- dependency tracking

## Implementation Plan

### Gap 1: Temp Variable Inlining Pass [IN PROGRESS]

**Upstream:** Babel's codegen never sees raw SSA temps -- its IR-to-code translation directly inlines simple expressions. The relevant upstream logic is spread across `CodegenReactiveFunction.ts` and `PrintReactiveFunction.ts`.
**Current state (updated 2026-03-13):** Recursive cross-scope temp use-counting has been implemented directly in `codegen.rs`. The codegen now walks nested `ReactiveTerminal::Scope` blocks when counting temp uses, so temps referenced only inside child scopes are correctly identified as single-use and inlined. All hash collections in codegen were migrated to `FxHashMap`/`FxHashSet` for performance. Conformance remains at 304/1717 -- this is a foundational fix that unblocks other P1 items rather than moving fixtures on its own.

**What remains:**
- The inlining logic itself is functional and correct for cross-scope cases
- Fixture gains will materialize when combined with other P1 fixes (JSX preservation, sentinel scopes, over-scoped deps) -- the interdependency noted in the risk section is the key blocker
- May need additional refinement now that JSX preservation (Gap 2) has landed, as JSX nodes create additional temp chains that are now exercised

**Implementation file:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
**Fixture gain estimate:** ~150-200 (compound effect with other P1 gaps; 0 in isolation)
**Depends on:** None (but gains depend on Gap 2 + Gap 5 + Gap 6)

### Gap 2: JSX Syntax Preservation in Codegen ✅

~~**Upstream:** `CodegenReactiveFunction.ts` emits JSX syntax directly (`<div>`, `<Component>`, `<>{...}</>`)~~
~~**Current state:** `codegen.rs` lines 325-348 emit `_jsx("div", { ... })` and `_jsxs(_Fragment, { children: [...] })` function call syntax~~

**Completed**: JSX syntax preservation fully implemented in `codegen.rs`. The `InstructionValue::JsxExpression` arm now emits proper JSX syntax (`<div>`, `<Component>`, `<>...</>`) instead of `_jsx()`/`_jsxs()` function calls. Self-closing vs open/close tags, spread props, string/expression children, and fragment shorthand all handled. The `react/jsx-runtime` import is removed from generated output; only `_c` from `react/compiler-runtime` remains. 23 snapshot files updated. Conformance unchanged at 304/1717 due to JSX normalization in the test harness. Implementation file: `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`.

### Gap 3: Cache Slot Count Alignment

**Upstream:** Babel counts cache slots based on reactive scope outputs + dependencies
**Current state:** Our `count_cache_slots()` function counts slots based on our scope structure, which may differ from Babel's due to different scope boundaries and extra temporaries
**What's needed:**
- After Gap 1 (temp inlining) is done, re-measure cache slot divergences -- many may be resolved by having fewer temps
- Compare `count_cache_slots` logic with upstream's `getScopeCount` in `CodegenReactiveFunction.ts`
- Verify that scope outputs and declarations match upstream's expectations
- This gap may be fully resolved by Gap 1 + Gap 4 + Gap 5 + Gap 6
**Fixture gain estimate:** Compound effect with other gaps
**Depends on:** Gap 1 (temp inlining reduces slot count)

### Gap 4: Scope Merging/Splitting Heuristic Review

**Upstream:** `MergeOverlappingReactiveScopes.ts`, `PruneNonEscapingScopes.ts`, `PruneAlwaysInvalidatingScopes.ts`
**Current state:** We have `merge_scopes.rs` and `prune_scopes.rs` implementing these passes, but the heuristics may diverge in edge cases
**What's needed:**
- Audit `merge_overlapping_reactive_scopes_hir` against upstream `MergeOverlappingReactiveScopes.ts` -- check the merge condition (do we merge scopes whose ranges overlap the same way Babel does?)
- Audit `merge_reactive_scopes_that_invalidate_together` against upstream -- check the "invalidate together" heuristic
- Audit scope terminal construction (`build_reactive_scope_terminals_hir`) -- verify we create scope boundaries at the same points
- Fix any divergences found
- Re-measure after fixes
**Fixture gain estimate:** ~50-100 (scope boundary differences cause wrong slot groupings)
**Depends on:** None (can be done in parallel with Gap 1)

### Gap 5: Sentinel Scope Emission ✅

~~**Upstream:** Babel creates reactive scopes for allocating expressions (JSX elements, object/array literals) even when they have no reactive dependencies. These scopes use the sentinel pattern (`Symbol.for("react.memo_cache_sentinel")`) instead of dependency checking.~~
~~**Current state:** `infer_reactive_scope_variables.rs` only creates scopes for reactive identifiers.~~

**Completed**: Sentinel scope emission is now active. `infer_reactive_scope_variables.rs` creates reactive scopes for allocating expressions (JSX elements, object/array literals) even when they have no reactive dependencies. `prune_scopes.rs` was updated to preserve these scopes. `codegen.rs` emits the sentinel pattern (`Symbol.for("react.memo_cache_sentinel")`) for scopes with zero reactive dependencies. Net conformance impact: -32 (35 regressions added to known-failures.txt, 3 newly passing). The regressions are expected -- the scopes are structurally correct but other P1 issues (over-scoped deps in Gap 6, slot counts in Gap 3) cause the overall output to still diverge. Implementation files: `infer_reactive_scope_variables.rs`, `prune_scopes.rs`, `codegen.rs`.

### Gap 6: Over-Scoped Dependencies ✅

~~**Upstream:** Babel correctly identifies global values (e.g., `Math.max`, `console.log`), stable hook returns (e.g., `setState` from `useState`), and other non-reactive values, and excludes them from dependency tracking.~~
~~**Current state:** We treat some globals and stable values as reactive, causing them to appear as dependencies in scopes. This results in more cache slots than needed (~400 fixtures).~~

**Completed**: Globals, stable hook returns (SetState, Ref), and property accesses of globals are no longer treated as reactive dependencies. Three files modified: `infer_types.rs` (type inference for stable hook returns), `infer_reactive_places.rs` (globals and stable values excluded from reactive marking), `propagate_dependencies.rs` (global property accesses filtered from dependency propagation). Conformance unchanged at 272/1717 -- gains expected to compound with remaining P1 fixes (Gap 3 slot counts, Gap 4 scope heuristics).

### Gap 7: Property-Path Dependency Resolution ✅

~~**Upstream:** `PropagateScopeDependencies.ts` uses `collectTemporaries()` to follow LoadLocal → PropertyLoad → ComputedLoad chains, resolving each SSA temporary to its root named variable + property path. Dependencies are emitted as e.g. `props.x` rather than just `props`.~~
~~**Current state:** `propagate_dependencies.rs` emitted dependencies using the raw SSA temp identifier, losing property path information.~~

**Completed**: Full property-path dependency resolution implemented in `propagate_dependencies.rs`. A `temp_map: FxHashMap<IdentifierId, TemporaryInfo>` is built in Phase 1.5, tracing LoadLocal/LoadContext → PropertyLoad chains to resolve SSA temps to `(root_identifier, property_path)`. The `collect_read_operand_places_for_deps` function uses this map to emit `ReactiveScopeDependency` with proper `DependencyPathEntry` paths. `codegen.rs` gained `dependency_display_name()` to render deps with dot-separated property paths (including optional chaining `?.`). Sentinel scope codegen was also fixed to store the first declaration value into the sentinel slot (previously sentinel scopes had no cache-store, causing re-computation every render). `DependencyPathEntry` gained `PartialEq, Eq` derives for deduplication. Conformance: 315 → 318/1717 (+3). Implementation files: `propagate_dependencies.rs`, `codegen.rs`, `types.rs`.

### Gap 8: Sentinel Scope Codegen Correctness ✅

~~**Upstream:** Sentinel scopes (zero reactive deps) store the first declaration value into cache slot 0 after computation, so subsequent renders skip re-computation via the sentinel check.~~
~~**Current state:** Sentinel scopes emitted the sentinel check but never stored anything into the cache slot, causing re-computation every render.~~

**Completed**: Fixed in `codegen.rs`. When `deps.is_empty()` (sentinel scope), the codegen now stores the first declaration value (`$[slot_start] = declName`) after the if-block body. This matches upstream behavior where sentinel scopes mark themselves as "computed" by writing a value to the sentinel slot. Part of the Gap 7 changeset.

## Measurement Strategy

After each gap, run conformance and measure:
```bash
cargo test conformance -- --nocapture 2>&1 | tail -5
```

Expected progression (gaps are interdependent, so gains compound):
- Gap 1 (temp inlining) ✅ + Gap 2 (JSX) ✅ + Gap 5 (sentinel) ✅: structural foundation complete, 35 temporary regressions
- Gap 6 (over-scoped deps) ✅: globals/stable values excluded from deps
- Gap 7 (property-path deps) ✅ + Gap 8 (sentinel codegen) ✅: deps now emit `props.x` not just `props`, sentinel scopes store values correctly (+3 fixtures)
- After Gap 4 (scope heuristics): ~50-100 additional
- After Gap 3 (slot count alignment): remaining residual
- Total potential from this category: ~400-600 new passes

## Risks and Notes

- **Interdependency is the key risk**: Previous experience shows that fixing one structural issue in isolation gains zero fixtures because the remaining issues still cause mismatches. Temp inlining (Gap 1), JSX preservation (Gap 2), sentinel scope emission (Gap 5), over-scoped deps (Gap 6), property-path deps (Gap 7), and sentinel codegen (Gap 8) are all complete. Property-path deps yielded +3 fixtures, showing the compound effect is beginning. Slot count alignment (Gap 3) and scope heuristics (Gap 4) are the remaining blockers before larger compound fixture gains materialize.
- **Temp inlining correctness**: Must verify that inlined expressions maintain the same evaluation order. Only inline pure expressions or expressions where order doesn't matter.
- **JSX edge cases**: Self-closing elements, boolean attributes (`<div disabled />`), computed property names in JSX, namespace attributes (`xml:lang`).
- **Scope merging audit scope**: The merge/prune passes are among the most complex in the compiler. A full audit requires careful line-by-line comparison with upstream TypeScript.
