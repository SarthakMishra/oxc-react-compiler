# Memoization Structure Differences

> **Priority**: P1 (largest category -- 904 fixtures, 64% of all failures)
> **Impact**: 904 divergences where both compilers produce `_c()` but structure differs
> **Tractability**: LOW per-item, HIGH aggregate -- items are interdependent; no single fix moves the needle alone

## Problem Statement

When both our compiler and Babel memoize a function, our output differs structurally. The 904 fixtures break down into:

| Sub-category | Count | Root cause |
|-------------|-------|------------|
| Over-scoped (too many cache slots) | ~400 | Globals/stable values treated as reactive deps |
| Sentinel pattern never emitted | ~280 | Non-reactive allocations need sentinel check scopes |
| Under-scoped (too few cache slots) | ~90 | Missing scopes for some expressions |
| Same slots, wrong deps | ~40 | Dependency tracking diverges |
| Other structural | ~94 | Temp variable naming, code ordering |

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

### Gap 5: Sentinel Scope Emission

**Upstream:** Babel creates reactive scopes for allocating expressions (JSX elements, object/array literals) even when they have no reactive dependencies. These scopes use the sentinel pattern (`Symbol.for("react.memo_cache_sentinel")`) instead of dependency checking.
**Current state:** `infer_reactive_scope_variables.rs` only creates scopes for reactive identifiers. An attempt to add `is_allocating` tracking was made but reverted because it gained 0 fixtures while losing 10 (the structural output still didn't match even with correct scope creation).
**What's needed:**
- Revisit allocating scope creation now that Gap 2 (JSX preservation) is done -- the previous attempt failed because structural differences masked the fix. Gap 1 (temp inlining) is also in place.
- Add sentinel pattern emission to codegen: instead of `if ($[0] !== dep)`, emit `if ($[0] === Symbol.for("react.memo_cache_sentinel"))` for allocating-only scopes
- This is the root cause of ~280 divergences
**Fixture gain estimate:** ~100-200 (but only after Gaps 1+2 are done)
**Depends on:** Gap 1 ✅, Gap 2 ✅ (both now complete -- this gap is unblocked)

### Gap 6: Over-Scoped Dependencies

**Upstream:** Babel correctly identifies global values (e.g., `Math.max`, `console.log`), stable hook returns (e.g., `setState` from `useState`), and other non-reactive values, and excludes them from dependency tracking.
**Current state:** We treat some globals and stable values as reactive, causing them to appear as dependencies in scopes. This results in more cache slots than needed (~400 fixtures).
**What's needed:**
- Audit `infer_reactive_places.rs` against upstream `InferReactivePlaces.ts` -- verify which identifiers are marked as reactive
- Verify that globals are never marked reactive
- Verify that stable hook returns (setState, dispatch, ref objects) are not marked reactive
- May also involve `propagate_dependencies.rs` -- some dependencies may be added during propagation that upstream excludes
**Fixture gain estimate:** ~100-200 (reducing false reactive deps fixes slot counts)
**Depends on:** None

## Measurement Strategy

After each gap, run conformance and measure:
```bash
cargo test conformance -- --nocapture 2>&1 | tail -5
```

Expected progression (gaps are interdependent, so gains compound):
- After Gap 1 (temp inlining) + Gap 2 (JSX): ~200-300 new passes
- After Gap 4 (scope heuristics) + Gap 6 (over-scoped deps): ~100-200 additional
- After Gap 5 (sentinel scopes): ~100-200 additional
- After Gap 3 (slot count alignment): remaining residual
- Total potential from this category: ~400-600 new passes

## Risks and Notes

- **Interdependency is the key risk**: Previous experience shows that fixing one structural issue in isolation gains zero fixtures because the remaining issues still cause mismatches. Temp inlining (Gap 1) and JSX preservation (Gap 2) are now both complete -- sentinel scope emission (Gap 5) is the next unblocked high-impact item.
- **Temp inlining correctness**: Must verify that inlined expressions maintain the same evaluation order. Only inline pure expressions or expressions where order doesn't matter.
- **JSX edge cases**: Self-closing elements, boolean attributes (`<div disabled />`), computed property names in JSX, namespace attributes (`xml:lang`).
- **Scope merging audit scope**: The merge/prune passes are among the most complex in the compiler. A full audit requires careful line-by-line comparison with upstream TypeScript.
