# Upstream Errors -- Validation Gaps

> **Priority**: P2 (~50 actionable remaining fixtures, high tractability -- each fix is "emit error + bail")
> **Impact**: ~50 remaining actionable fixtures where we compile but Babel bails with a validation error (63 total error fixtures - 13 invariant/todo skips = 50 actionable)
> **Tractability**: HIGH -- each sub-category is a focused validation improvement

## Problem Statement

For 96 fixtures, Babel's validation passes detect a problem and reject the
function (returning source unchanged), but our compiler proceeds and emits
memoized output. Since both sides now return source unchanged when we bail,
fixing each validation gap directly converts fixtures to passes.

Note: 15 additional fixtures fail due to Babel internal errors (Invariant/Todo)
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

**What remains (8 fixtures):**
- ~~Track "frozen" status on values~~ Done
- ~~Detect mutations to frozen values: property writes, array push~~ Done
- ~~Context variable mutations~~ Done (hook-return pre-freeze + function-capture freeze)
- ~~Mutations inside nested functions~~ Done (nested function scanning)
- ~~Indirect mutations through captured closures~~ Done (function-capture freeze)
- Alias tracking: if `a = b` and `b` is frozen, mutating `a` should also error (e.g., `invalid-mutate-after-aliased-freeze`)
- Phi-node frozen tracking: values that *could* be frozen through phi nodes (e.g., `invalid-mutate-phi-which-could-be-frozen`)
- Delete operations on frozen values (e.g., `invalid-delete-computed-property-of-frozen-value`, `invalid-delete-property-of-frozen-value`)
- Indirect mutation through function calls passed as props (e.g., `invalid-pass-mutable-function-as-prop`, `invalid-pass-ref-to-function`)
- Props mutation in effects via indirect references (e.g., `invalid-props-mutation-in-effect-indirect`)
- State mutation variant (e.g., `modify-state-2.js`)
- **Known limitation:** SSA pass assigns unique IDs per Place even for the same variable, making alias/identity tracking harder across instructions
**Fixture gain estimate:** ~3-8 more (remaining cases require deep alias propagation)
**Depends on:** None

### Gap 2: Validate Preserve Existing Memoization ✅

~~**Count:** 13 fixtures~~
~~**Upstream error:** "Compilation Skipped" (preserve-memo mode)~~
~~**Upstream:** `ValidatePreserveExistingMemoizationGuarantees.ts`~~

**Completed (2026-03-13):** Pipeline gate fixes for Pass 5 and Pass 61. Pass 5 (`drop_manual_memoization`) now preserves memo markers when `validate_preserve_existing_memoization_guarantees` is set (not just `enable_preserve`). Pass 61 now runs on both config flags. Error messages aligned with upstream ("Existing memoization could not be preserved..."). Pruned memoizations silently skipped. All 20 preserve-memo-validation error fixtures now passing, plus 11 bonus error fixtures from other categories. Rust modules: `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`, `crates/oxc_react_compiler/src/validation/validate_preserved_manual_memoization.rs`. +31 fixtures total (278 -> 309/1717).

### Gap 3: Exhaustive Deps Remaining

**Count:** 2 remaining (6 fixed by extra-dep detection + mode gating, 2026-03-13)
**Upstream error:** "Missing/extra deps"
**Upstream:** `ValidateExhaustiveDeps.ts`
**Current state:** `validate_exhaustive_dependencies.rs` enhanced with extra dependency detection, per-mode validation (All/MissingOnly/ExtraOnly), and config-driven gating. 6 fixtures fixed (309→315/1717). 2 remain: `error.invalid-exhaustive-effect-deps-missing-only.js`, `error.sketchy-code-exhaustive-deps.js`.
**What's needed:**
- Analyze the 2 remaining fixtures -- likely edge cases in dependency collection or sketchy-code detection patterns
- Note: 5 additional exhaustive-deps fixtures remain in known-failures but are compilation divergences (P1 territory), not validation-error fixtures
**Fixture gain estimate:** ~1-2
**Depends on:** None

### Gap 4: Reassign Outside Component (partially complete)

**Count:** ~2 remaining (6 of 8 now passing)
**Upstream error:** "Cannot reassign variables outside component"
**Upstream:** `ValidateLocalsNotReassignedAfterRender.ts`, `ValidateNoGlobalReassignment.ts` (split across two passes)

**Completed (2026-03-13):** Two-pronged fix:
1. `validate_no_global_reassignment.rs` rewritten with nested function scope analysis -- properly tracks function declarations, arrow functions, and function expressions as scope boundaries, distinguishing global vs local reassignment. Handles increment/decrement operators, compound assignments, and plain assignments.
2. `validate_locals_not_reassigned_after_render.rs` enhanced with async function/arrow detection -- reassignments inside async callbacks now correctly flagged as post-render mutations.
3. `build.rs` fixed function declaration lowering -- StoreLocal instruction now connects function value to its binding identifier, enabling proper scope tracking.

Rust modules: `crates/oxc_react_compiler/src/validation/validate_no_global_reassignment.rs`, `crates/oxc_react_compiler/src/validation/validate_locals_not_reassigned_after_render.rs`, `crates/oxc_react_compiler/src/hir/build.rs`. +8 fixtures (331 -> 339/1717).

Newly passing fixtures: `error.assign-global-in-component-tag-function`, `error.assign-global-in-jsx-children`, `error.reassign-global-fn-arg`, `error.mutate-global-increment-op-invalid-react`, `error.invalid-reassign-local-variable-in-async-callback`, `error.declare-reassign-variable-in-function-declaration`, `error.todo-repro-named-function-with-shadowed-local-same-name` (x2).

**What remains (~2 fixtures):**
- Edge cases likely involving indirect reassignment patterns (reassignment through destructuring, or module-scope variable mutation via object property aliasing)
- May require deeper SSA identity tracking (see Cross-Cutting Issue above)
**Fixture gain estimate:** ~1-2
**Depends on:** None

### Gap 5: Ref Access During Render ✅

~~**Count:** 6 fixtures~~
~~**Upstream error:** "Cannot access refs during render"~~
~~**Upstream:** `ValidateNoRefAccessInRender.ts`~~

**Completed (2026-03-14):** All 6 remaining ref-access-during-render fixtures resolved via improved nested function ref tracking in `validate_no_ref_access_in_render.rs`. The validation now detects ref access patterns inside nested function expressions and lambda callbacks, covering ref values passed through function calls, stored in data structures, and accessed through indirect patterns. Rust module: `crates/oxc_react_compiler/src/validation/validate_no_ref_access_in_render.rs`. +6 fixtures (part of 354 -> 362 batch).

### Gap 6: Dynamic Hook Identity

**Count:** 2 remaining (was 4; 2 resolved by SSA + conditional hook detection improvements)
**Upstream error:** "Hooks must be same function"
**Upstream:** `ValidateHooksUsage.ts`
**Current state:** `validate_hooks_usage.rs` exists with SSA resolution and conditional hook method call detection. 2 remain: `error.invalid-conditional-call-aliased-hook-import.js`, `error.invalid-conditional-call-aliased-react-hook.js` -- both involve aliased hook imports called conditionally.
**What's needed:**
- Detect patterns where a hook import is aliased to a local variable and then called conditionally
- The aliasing breaks SSA-based hook identity tracking
**Fixture gain estimate:** ~1-2
**Depends on:** None

### Gap 7: setState During Render (mostly complete)

**Count:** 1 remaining (2 of 3 resolved)
**Upstream error:** "Cannot call setState during render"
**Upstream:** `ValidateNoSetStateInRender.ts`
**Current state:** `validate_no_set_state_in_render.rs` enhanced with transitive setState detection through helper functions and lambdas. Fixpoint loop resolves arbitrarily deep call chains (foo → bar → baz → setState). Function-to-name mapping propagated through StoreLocal chains. 2 fixtures now passing: `error.unconditional-set-state-lambda.js`, `error.unconditional-set-state-nested-function-expressions.js`. 1 remaining: `error.invalid-hoisting-setstate.js` (requires hoisted context declaration tracking, overlaps with Gap 8).
**Fixture gain estimate:** ~0-1 (remaining fixture requires deeper hoisting analysis)
**Depends on:** None

### Gap 8: Hoisting/TDZ

**Count:** 1 actionable fixture (2 `todo-*` are upstream bugs, skippable)
**Upstream error:** "Cannot access variable before declared"
**Upstream:** Various validation logic in `HIRBuilder.ts`
**Current state:** No TDZ analysis exists. The actionable fixture is `error.invalid-hoisting-setstate.js` (overlaps with Gap 7). The 2 `todo-functiondecl-hoisting` fixtures are upstream TODOs.
**What's needed:**
- Detect references to `let`/`const` variables before their declaration point
- This may be caught during HIR building or as a separate validation pass
**Fixture gain estimate:** ~1
**Depends on:** None

### Gap 9: Other Validation Errors

**Count:** ~29 remaining uncategorized fixtures
**What's needed:** These cover several sub-categories not yet tracked individually:
- **Mutation tracking** (~11): `invalid-mutate-global-*`, `invalid-mutate-props-*`, `invalid-mutation-*`, `mutate-function-property`, `not-useEffect-external-mutate`, `invalid-return-mutable-function-from-hook`, `invalid-hook-function-argument-mutates-local-variable`
- **Hook-call capture freeze** (2): `hook-call-freezes-captured-identifier.tsx`, `hook-call-freezes-captured-memberexpr.jsx`
- **Type provider / incompatible module** (5): `invalid-known-incompatible-*`, `invalid-type-provider-*`
- **Ref naming heuristic** (2): `ref-like-name-not-Ref`, `ref-like-name-not-a-ref`
- **Preserve-memo edge cases** (2): `repro-preserve-memoization-inner-destructured-value-*`
- **Other** (~7): `assign-ref-in-effect-hint`, `capitalized-function-call-aliased`, `call-args-destructuring-asignment-complex`, `dont-hoist-inline-reference`, `invalid-unclosed-eslint-suppression`, `useMemo-non-literal-depslist`, `_todo.computed-lval-in-destructure`, `todo.try-catch-with-throw`
**Fixture gain estimate:** ~10-20 (many require focused per-fixture analysis)
**Depends on:** Analysis of individual fixtures

**Partially completed:**
- `validate_no_eval` pass added (Pass 14.6): detects `eval()` calls and bails out with `EvalUnsupported` diagnostic. Upstream: `ValidateNoJSXInTryStatements.ts` (eval check). Rust module: `crates/oxc_react_compiler/src/validation/validate_no_eval.rs`. Also added `"eval"` to `is_global_name`.
- Hooks-in-nested-functions (Rule 4) added to `validate_hooks_usage.rs` (2026-03-13): `check_hooks_in_nested_functions` detects hook calls inside FunctionExpression and ObjectMethod bodies. Emits bail diagnostic. +4 fixtures: `error.bail.rules-of-hooks-3d692676194b`, `error.bail.rules-of-hooks-8503ca76d6f8`, `error.invalid-hook-in-nested-object-method`, `error.invalid.invalid-rules-of-hooks-d952b82c2597`. Rust module: `crates/oxc_react_compiler/src/validation/validate_hooks_usage.rs`. Conformance: 339 -> 343/1717.

## Total Fixture Gain Estimate

Achieved so far: 76 (19 from Gap 1 frozen mutation [6 initial + 13 enhancement], 31 from Gap 2 preserve-memo pipeline gate fixes, 6 from exhaustive deps improvements, 8 from Gap 4 global reassignment + async callback, 4 from Gap 9 hooks-in-nested-functions, 6 from Gap 5 ref access during render, 2 from Gap 7 setState in nested functions).
Remaining achievable: ~12-32 of the remaining ~42 actionable fixtures. The
categorized gaps (1,3,4,6,7,8) account for ~16 fixtures; Gap 9 "Other" covers
~29 uncategorized fixtures requiring individual triage. The 15 Invariant/Todo
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
