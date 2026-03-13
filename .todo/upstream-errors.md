# Upstream Errors -- Validation Gaps

> **Priority**: P2 (~46 remaining fixtures, high tractability -- each fix is "emit error + bail")
> **Impact**: ~46 remaining fixtures where we compile but Babel bails with a validation error
> **Tractability**: HIGH -- each sub-category is a focused validation improvement

## Problem Statement

For 96 fixtures, Babel's validation passes detect a problem and reject the
function (returning source unchanged), but our compiler proceeds and emits
memoized output. Since both sides now return source unchanged when we bail,
fixing each validation gap directly converts fixtures to passes.

Note: 21 additional fixtures fail due to Babel internal errors (Invariant/Todo)
-- these are upstream bugs, not validation gaps. We should skip them.

## Sub-categories

### Gap 1: Frozen Mutation Detection (mostly complete)

**Count:** 5 remaining (21 of 26 now passing)
**Upstream error:** "This value cannot be modified"
**Upstream:** `ValidateLocalsNotReassignedAfterRender.ts`, `InferMutableRanges.ts`

**Completed (2026-03-13, initial):** `validate_no_mutation_after_freeze` pass added as Pass 16.5, running after `infer_mutation_aliasing_effects`. Detects property stores, computed stores, and array push on frozen values. Also detects for-in/for-of loops over context variables. +6 fixtures passing.

**Completed (2026-03-13, enhancement):** Three major improvements to freeze tracking:
1. Hook-return pre-freeze: Values returned by hook calls (useContext, useState, etc.) and all their destructured targets are frozen at definition site. Uses `collect_frozen_from_destructure` for nested array/object patterns. DIVERGENCE: Over-freezes setters (e.g., setState from useState), but setters are never mutated via property stores in practice.
2. Function-capture freeze: When a function argument is passed to a hook call, all variables captured by that function are frozen after the call. Tracks captures via `func_captures` and `name_to_func_captures` maps.
3. Nested function mutation scanning: `check_nested_function_mutation` recursively scans FunctionExpression bodies for mutations to outer frozen variables, including checking aliasing effects.

Rust module: `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`. +13 fixtures (318 -> 331/1717).

Newly passing fixtures include: `capture-ref-for-mutation`, `invalid-disallow-mutating-refs-in-render-transitive`, `invalid-function-expression-mutates-immutable-value`, `invalid-jsx-captures-context-variable`, `invalid-mutate-context`, `invalid-mutate-context-in-callback`, `invalid-non-imported-reanimated-shared-value-writes`, `modify-state`, `modify-useReducer-state`, `todo-allow-assigning-to-inferred-ref-prop-in-callback`, `todo-for-loop-with-context-variable-iterator`, `invalid-hook-from-property-of-other-hook`, `skip-useMemoCache`.

**What remains (5 fixtures):**
- ~~Track "frozen" status on values~~ Done
- ~~Detect mutations to frozen values: property writes, array push~~ Done
- ~~Context variable mutations~~ Done (hook-return pre-freeze + function-capture freeze)
- ~~Mutations inside nested functions~~ Done (nested function scanning)
- ~~Indirect mutations through captured closures~~ Done (function-capture freeze)
- Alias tracking: if `a = b` and `b` is frozen, mutating `a` should also error (e.g., `invalid-mutate-after-aliased-freeze`)
- Delete operations on frozen values (e.g., `invalid-delete-computed-property-of-frozen-value`, `invalid-delete-property-of-frozen-value`)
- Indirect mutation through function calls passed as props (e.g., `invalid-pass-mutable-function-as-prop`, `invalid-pass-ref-to-function`)
- Props mutation in effects via indirect references (e.g., `invalid-props-mutation-in-effect-indirect`)
- **Known limitation:** SSA pass assigns unique IDs per Place even for the same variable, making alias/identity tracking harder across instructions
**Fixture gain estimate:** ~2-5 more (remaining cases require deep alias propagation)
**Depends on:** None

### Gap 2: Validate Preserve Existing Memoization ✅

~~**Count:** 13 fixtures~~
~~**Upstream error:** "Compilation Skipped" (preserve-memo mode)~~
~~**Upstream:** `ValidatePreserveExistingMemoizationGuarantees.ts`~~

**Completed (2026-03-13):** Pipeline gate fixes for Pass 5 and Pass 61. Pass 5 (`drop_manual_memoization`) now preserves memo markers when `validate_preserve_existing_memoization_guarantees` is set (not just `enable_preserve`). Pass 61 now runs on both config flags. Error messages aligned with upstream ("Existing memoization could not be preserved..."). Pruned memoizations silently skipped. All 20 preserve-memo-validation error fixtures now passing, plus 11 bonus error fixtures from other categories. Rust modules: `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`, `crates/oxc_react_compiler/src/validation/validate_preserved_manual_memoization.rs`. +31 fixtures total (278 -> 309/1717).

### Gap 3: Exhaustive Deps Remaining

**Count:** 8 fixtures
**Upstream error:** "Missing/extra deps"
**Upstream:** `ValidateExhaustiveDeps.ts`
**Current state:** `validate_exhaustive_dependencies.rs` exists with recent SSA name resolution improvements. 8 fixtures still diverge -- likely edge cases in dependency analysis.
**What's needed:**
- Analyze the 8 failing fixtures to understand which dependency patterns are missed
- May involve: conditional dependencies, optional chaining in deps, computed property access as deps
- SSA resolution was recently added -- remaining gaps are likely deeper semantic issues
**Fixture gain estimate:** ~5-8
**Depends on:** None

### Gap 4: Reassign Outside Component

**Count:** 6 fixtures
**Upstream error:** "Cannot reassign variables outside component"
**Upstream:** `ValidateLocalsNotReassignedAfterRender.ts`
**Current state:** `validate_locals_not_reassigned_after_render.rs` exists. May not detect reassignment of module-level variables from within component functions.
**What's needed:**
- Detect when a component/hook function assigns to a variable declared in an outer (module) scope
- Emit the appropriate error
- This is related to Gap 1 (both involve mutation tracking) but focuses on scope-crossing assignments
**Fixture gain estimate:** ~4-6
**Depends on:** None

### Gap 5: Ref Access During Render

**Count:** 6 fixtures
**Upstream error:** "Cannot access refs during render"
**Upstream:** `ValidateNoRefAccessInRender.ts`
**Current state:** `validate_no_ref_access_in_render.rs` exists with recent SSA resolution and PropertyStore/PropertyLoad improvements (+6 and +15 fixtures in recent sessions). 6 remaining failures indicate deeper ref aliasing patterns not yet handled.
**What's needed:**
- Analyze the 6 remaining fixtures -- likely involve:
  - Ref values passed through function calls and returned
  - Ref values stored in data structures (arrays, objects) and accessed later
  - Indirect ref access through destructuring
- Extend ref identity tracking to follow these patterns
**Fixture gain estimate:** ~3-6
**Depends on:** None

### Gap 6: Dynamic Hook Identity

**Count:** 4 fixtures
**Upstream error:** "Hooks must be same function"
**Upstream:** `ValidateHooksUsage.ts`
**Current state:** `validate_hooks_usage.rs` exists with SSA resolution. These 4 fixtures likely involve dynamic hook identity -- calling different hook implementations conditionally.
**What's needed:**
- Detect patterns where a variable holding a hook function is assigned different values in different branches
- Emit error when hook identity is not stable across renders
**Fixture gain estimate:** ~2-4
**Depends on:** None

### Gap 7: setState During Render

**Count:** 2 fixtures
**Upstream error:** "Cannot call setState during render"
**Upstream:** `ValidateNoSetStateInRender.ts`
**Current state:** `validate_no_set_state_in_render.rs` exists with SSA resolution (+9 fixtures recently). 2 remaining fixtures need transitive setState tracking.
**What's needed:**
- Track setState calls through helper functions (transitive detection)
- If `helper()` calls `setState`, and the component calls `helper()` during render, that's an error
**Fixture gain estimate:** ~1-2
**Depends on:** None

### Gap 8: Hoisting/TDZ

**Count:** 2 fixtures
**Upstream error:** "Cannot access variable before declared"
**Upstream:** Various validation logic in `HIRBuilder.ts`
**Current state:** No TDZ analysis exists.
**What's needed:**
- Detect references to `let`/`const` variables before their declaration point
- This may be caught during HIR building or as a separate validation pass
**Fixture gain estimate:** ~1-2
**Depends on:** None

### Gap 9: Other

**Count:** ~7 remaining fixtures (was 8)
**What's needed:** Triage individually -- these are miscellaneous validation errors that don't fit the above categories. Some may be one-off edge cases in existing validation passes.
**Fixture gain estimate:** ~3-7
**Depends on:** Analysis of individual fixtures

**Partially completed:**
- `validate_no_eval` pass added (Pass 14.6): detects `eval()` calls and bails out with `EvalUnsupported` diagnostic. Upstream: `ValidateNoJSXInTryStatements.ts` (eval check). Rust module: `crates/oxc_react_compiler/src/validation/validate_no_eval.rs`. Also added `"eval"` to `is_global_name`.

## Total Fixture Gain Estimate

Achieved so far: 56 (19 from Gap 1 frozen mutation [6 initial + 13 enhancement], 31 from Gap 2 preserve-memo pipeline gate fixes, 6 from exhaustive deps improvements).
Remaining achievable: ~17-37 of the remaining ~46 fixtures (some require deep
alias tracking that may not be worth the complexity). The 21 Invariant/Todo
fixtures should be registered as known skips.

## Cross-Cutting Issue: SSA Place Identity

**Discovered during Gap 1 implementation.** The SSA pass assigns a unique
`IdentifierId` per `Place` even when multiple Places refer to the same source
variable. This means alias tracking across instructions is harder than it should
be -- you cannot simply compare `IdentifierId` values to determine if two Places
refer to the same variable. This affects Gap 1 (alias-based frozen mutation
detection), Gap 4 (reassign outside component), and Gap 5 (ref aliasing). A
future SSA improvement to unify variable references would simplify all three.

## Measurement Strategy

```bash
cargo test conformance -- --nocapture 2>&1 | tail -5
```

Each gap can be measured independently since they target different error categories.
