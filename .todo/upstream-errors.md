# Upstream Errors -- Validation Gaps

> **Priority**: P2 (~20 actionable remaining fixtures, high tractability -- each fix is "emit error + bail")
> **Impact**: ~13 remaining actionable fixtures where we compile but Babel bails with a validation error (63 total error fixtures - 13 invariant/todo skips - 37 resolved = 13 actionable)
> **Tractability**: HIGH -- each sub-category is a focused validation improvement

## Problem Statement

For 96 fixtures, Babel's validation passes detect a problem and reject the
function (returning source unchanged), but our compiler proceeds and emits
memoized output. Since both sides now return source unchanged when we bail,
fixing each validation gap directly converts fixtures to passes.

Note: 15 additional fixtures fail due to Babel internal errors (Invariant/Todo)
-- these are upstream bugs, not validation gaps. We should skip them.

## Sub-categories

### Gap 1: Frozen Mutation Detection (nearly complete)

**Count:** ~4 remaining (moved from Gap 9 overlap; alias + phi tracking now done)
**Upstream error:** "This value cannot be modified"
**Upstream:** `ValidateLocalsNotReassignedAfterRender.ts`, `InferMutableRanges.ts`

**Completed (2026-03-13, initial):** `validate_no_mutation_after_freeze` pass added as Pass 16.5, running after `infer_mutation_aliasing_effects`. Detects property stores, computed stores, and array push on frozen values. Also detects for-in/for-of loops over context variables. +6 fixtures passing.

**Completed (2026-03-13, enhancement):** Three major improvements to freeze tracking:
1. Hook-return pre-freeze: Values returned by hook calls (useContext, useState, etc.) and all their destructured targets are frozen at definition site. Uses `collect_frozen_from_destructure` for nested array/object patterns. DIVERGENCE: Over-freezes setters (e.g., setState from useState), but setters are never mutated via property stores in practice.
2. Function-capture freeze: When a function argument is passed to a hook call, all variables captured by that function are frozen after the call. Tracks captures via `func_captures` and `name_to_func_captures` maps.
3. Nested function mutation scanning: `check_nested_function_mutation` recursively scans FunctionExpression bodies for mutations to outer frozen variables, including checking aliasing effects.

Rust module: `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`. +13 fixtures (318 -> 331/1717).

Newly passing fixtures include: `capture-ref-for-mutation`, `invalid-disallow-mutating-refs-in-render-transitive`, `invalid-function-expression-mutates-immutable-value`, `invalid-jsx-captures-context-variable`, `invalid-mutate-context`, `invalid-mutate-context-in-callback`, `invalid-non-imported-reanimated-shared-value-writes`, `modify-state`, `modify-useReducer-state`, `todo-allow-assigning-to-inferred-ref-prop-in-callback`, `todo-for-loop-with-context-variable-iterator`, `invalid-hook-from-property-of-other-hook`, `skip-useMemoCache`.

**Completed (2026-03-14, param pre-freeze):** Function parameters are now pre-frozen using `param_names` from the pipeline. This enables detection of mutations to props and other frozen parameters inside closures and through indirect references. +6 fixtures (362 -> 368/1717).

Newly passing fixtures: `error.invalid-mutation-in-closure.js`, `error.invalid-prop-mutation-indirect.js`, `error.invalid-props-mutation-in-effect-indirect.js`, `fault-tolerance/error.try-finally-and-mutation-of-props.js`, `fault-tolerance/error.var-declaration-and-mutation-of-props.js`, `repro-retain-source-when-bailout.js`.

**Completed (2026-03-14, mutation tracking deep session):** Three more sub-categories resolved:
1. Alias freeze tracking: if `a = b` and `b` is frozen, mutating `a` now correctly errors.
2. Phi-node freeze propagation: values that could be frozen through phi nodes now tracked.
3. Derivation chain tracking: derived values from frozen sources tracked through assignment chains.

Rust modules: `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`, `crates/oxc_react_compiler/src/hir/build.rs` (delete expression lowering fix).

**Completed (2026-03-14, Phase 65):** Hook-arg local mutation -- `error.invalid-hook-function-argument-mutates-local-variable.js` now detects mutation of local variables passed as hook function arguments. +1 fixture (part of 384 -> 388 batch).

**What remains (~3 fixtures):**
- ~~Track "frozen" status on values~~ Done
- ~~Detect mutations to frozen values: property writes, array push~~ Done
- ~~Context variable mutations~~ Done (hook-return pre-freeze + function-capture freeze)
- ~~Mutations inside nested functions~~ Done (nested function scanning)
- ~~Indirect mutations through captured closures~~ Done (function-capture freeze)
- ~~Props mutation in effects via indirect references~~ Done (param pre-freeze)
- ~~Indirect mutation through function calls passed as props~~ Partially addressed (param pre-freeze covers direct prop mutation patterns)
- ~~Alias tracking~~ Done (alias freeze tracking)
- ~~Phi-node frozen tracking~~ Done (phi-node freeze propagation)
- ~~Hook-arg local mutation~~ Done (Phase 65)
- `error.invalid-mutate-props-in-effect-fixpoint.js` -- props mutation in effect with fixpoint iteration
- `error.invalid-mutation-of-possible-props-phi-indirect.js` -- indirect phi-based possible-props mutation
- `error.mutate-function-property.js` -- mutation of function object property
**Fixture gain estimate:** ~1-3 (remaining cases require deeper analysis of specific mutation patterns)
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

### Gap 4: Reassign Outside Component ✅

~~**Count:** ~2 remaining (6 of 8 now passing)~~ All 8 resolved.
**Upstream error:** "Cannot reassign variables outside component"
**Upstream:** `ValidateLocalsNotReassignedAfterRender.ts`, `ValidateNoGlobalReassignment.ts` (split across two passes)

**Completed (2026-03-13):** Two-pronged fix:
1. `validate_no_global_reassignment.rs` rewritten with nested function scope analysis -- properly tracks function declarations, arrow functions, and function expressions as scope boundaries, distinguishing global vs local reassignment. Handles increment/decrement operators, compound assignments, and plain assignments.
2. `validate_locals_not_reassigned_after_render.rs` enhanced with async function/arrow detection -- reassignments inside async callbacks now correctly flagged as post-render mutations.
3. `build.rs` fixed function declaration lowering -- StoreLocal instruction now connects function value to its binding identifier, enabling proper scope tracking.

Rust modules: `crates/oxc_react_compiler/src/validation/validate_no_global_reassignment.rs`, `crates/oxc_react_compiler/src/validation/validate_locals_not_reassigned_after_render.rs`, `crates/oxc_react_compiler/src/hir/build.rs`. +8 fixtures (331 -> 339/1717).

Newly passing fixtures: `error.assign-global-in-component-tag-function`, `error.assign-global-in-jsx-children`, `error.reassign-global-fn-arg`, `error.mutate-global-increment-op-invalid-react`, `error.invalid-reassign-local-variable-in-async-callback`, `error.declare-reassign-variable-in-function-declaration`, `error.todo-repro-named-function-with-shadowed-local-same-name` (x2).

**Completed (2026-03-14, destructure-to-global):** Destructure assignment patterns that reassign global/outer-scope variables now correctly detected in `validate_no_global_reassignment.rs`. Both remaining fixtures resolved: `error.invalid-destructure-assignment-to-global.js` (destructuring assignment to a global variable) and `error.invalid-destructure-to-local-global-variables.js` (mixed destructuring where some targets are global). Gap 4 is now fully complete (all 8 fixtures resolved). +2 fixtures (part of 388 -> 391 batch).

~~**What remains (~2 fixtures):**~~
~~- Edge cases likely involving indirect reassignment patterns (reassignment through destructuring, or module-scope variable mutation via object property aliasing)~~
~~- May require deeper SSA identity tracking (see Cross-Cutting Issue above)~~
**Fixture gain estimate:** All resolved.
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

**Count:** ~10 remaining uncategorized fixtures
**What's needed:** These cover several sub-categories not yet tracked individually:
- **Mutation tracking** (~3): `invalid-mutate-props-in-effect-fixpoint`, `invalid-mutation-of-possible-props-phi-indirect`, `mutate-function-property` (also tracked under Gap 1 remaining)
- **Type provider** (2): `invalid-type-provider-*`
- **Ref naming heuristic** (2): `ref-like-name-not-Ref`, `ref-like-name-not-a-ref`
- **Preserve-memo edge cases** (2): `repro-preserve-memoization-inner-destructured-value-*`
- **Other** (~2): `call-args-destructuring-asignment-complex`, `dont-hoist-inline-reference`
- ~~`invalid-mutate-global-*`~~ Resolved (outer-scope property mutation + render helper detection)
- ~~`not-useEffect-external-mutate`~~ Resolved
- ~~`invalid-return-mutable-function-from-hook`~~ Resolved
- ~~`hook-call-freezes-captured-identifier.tsx`~~ Resolved (Phase 65: hook-call capture freeze)
- ~~`hook-call-freezes-captured-memberexpr.jsx`~~ Resolved (Phase 65: hook-call capture freeze)
- ~~`assign-ref-in-effect-hint`~~ Resolved (Phase 65: assign-ref diagnostic)
- ~~`invalid-hook-function-argument-mutates-local-variable`~~ Resolved (Phase 65: moved to Gap 1 completed)
**Fixture gain estimate:** ~3-7 (remaining require focused per-fixture analysis)
**Depends on:** Analysis of individual fixtures

**Partially completed:**
- `validate_no_eval` pass added (Pass 14.6): detects `eval()` calls and bails out with `EvalUnsupported` diagnostic. Upstream: `ValidateNoJSXInTryStatements.ts` (eval check). Rust module: `crates/oxc_react_compiler/src/validation/validate_no_eval.rs`. Also added `"eval"` to `is_global_name`.
- Hooks-in-nested-functions (Rule 4) added to `validate_hooks_usage.rs` (2026-03-13): `check_hooks_in_nested_functions` detects hook calls inside FunctionExpression and ObjectMethod bodies. Emits bail diagnostic. +4 fixtures: `error.bail.rules-of-hooks-3d692676194b`, `error.bail.rules-of-hooks-8503ca76d6f8`, `error.invalid-hook-in-nested-object-method`, `error.invalid.invalid-rules-of-hooks-d952b82c2597`. Rust module: `crates/oxc_react_compiler/src/validation/validate_hooks_usage.rs`. Conformance: 339 -> 343/1717.
- Known-incompatible module detection added to `program.rs` (2026-03-14): Scans import sources for incompatible libraries (`react-native-reanimated`, `react-native-gesture-handler`, `@shopify/react-native-skia`) and rejects the entire file with a bail diagnostic. +3 fixtures: `error.invalid-known-incompatible-hook.js`, `error.invalid-known-incompatible-hook-return-property.js`, `error.invalid-known-incompatible-function.js`. Rust module: `crates/oxc_react_compiler/src/entrypoint/program.rs`.
- ESLint suppression detection added to `program.rs` (2026-03-14): Scans source text for unclosed `eslint-disable-next-line react-hooks/exhaustive-deps` comments (suppression without matching re-enable). +2 fixtures: `error.invalid-unclosed-eslint-suppression.js`, `unclosed-eslint-suppression-skips-all-components.js`. Rust module: `crates/oxc_react_compiler/src/entrypoint/program.rs`.
- `useMemo` non-literal dependency list detection added to `validate_use_memo.rs` (2026-03-14): Rejects `useMemo(fn, deps)` where deps argument is not an array literal. +1 fixture: `error.useMemo-non-literal-depslist.ts`. Rust module: `crates/oxc_react_compiler/src/validation/validate_use_memo.rs`.
- Capitalized call alias resolution improved in `validate_no_capitalized_calls.rs` (2026-03-14): Better SSA resolution for aliased capitalized function calls. Rust module: `crates/oxc_react_compiler/src/validation/validate_no_capitalized_calls.rs`.
- Total from Gap 9 completions in this batch: +6 fixtures (368 -> 374/1717).
- **Mutation tracking deep session (2026-03-14):** 6 sub-categories addressed:
  - Delete expression lowering fixed in `build.rs` -- delete expressions now correctly lowered to HIR, enabling mutation detection for delete operations.
  - Phi-node freeze propagation in `validate_no_mutation_after_freeze.rs` -- values that could be frozen through phi nodes now tracked across branches.
  - Alias freeze tracking in `validate_no_mutation_after_freeze.rs` -- `a = b` where `b` is frozen causes mutations to `a` to error.
  - Derivation chain tracking in `validate_no_mutation_after_freeze.rs` -- derived values from frozen sources tracked through multi-step assignment chains.
  - Outer-scope property mutation detection in `validate_no_global_reassignment.rs` -- property stores/mutations on variables from outer scopes now detected.
  - Render helper detection in `validate_no_global_reassignment.rs` -- functions invoked as render helpers properly validated for global mutation.
  - 10 fixtures removed from known-failures.txt. +10 fixtures (374 -> 384/1717).
  - Rust modules: `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`, `crates/oxc_react_compiler/src/validation/validate_no_global_reassignment.rs`, `crates/oxc_react_compiler/src/hir/build.rs`.
- **Phase 65 (2026-03-14):** 3 sub-categories addressed:
  - Hook-call capture freeze: `hook-call-freezes-captured-identifier.tsx` and `hook-call-freezes-captured-memberexpr.jsx` -- hook calls that freeze captured identifiers and member expressions now properly detected in `validate_no_mutation_after_freeze.rs`.
  - Hook-arg local mutation: `invalid-hook-function-argument-mutates-local-variable.js` -- mutation of local variables passed as hook function arguments now detected (moved from Gap 1 remaining to completed).
  - Assign-ref hint: `assign-ref-in-effect-hint.js` -- correct diagnostic now emitted for ref assignment in effects.
  - 4 fixtures removed from known-failures.txt. +4 fixtures (384 -> 388/1717).
- **Destructure-to-global + bailout-infer-mode (2026-03-14):** 3 fixtures resolved:
  - Destructure assignment to globals: `error.invalid-destructure-assignment-to-global.js` and `error.invalid-destructure-to-local-global-variables.js` -- destructuring assignments targeting global/outer-scope variables now detected in `validate_no_global_reassignment.rs`. Completes Gap 4.
  - Bailout without compilation in infer mode: `should-bailout-without-compilation-infer-mode.js` -- correctly bails out when compilation is not needed in infer mode.
  - 3 fixtures removed from known-failures.txt. +3 fixtures (388 -> 391/1717).

## Total Fixture Gain Estimate

Achieved so far: 105 (25 from Gap 1 frozen mutation [6 initial + 13 enhancement + 6 param pre-freeze], 31 from Gap 2 preserve-memo pipeline gate fixes, 6 from exhaustive deps improvements, 10 from Gap 4 global reassignment + async callback + destructure-to-global, 4 from Gap 9 hooks-in-nested-functions, 6 from Gap 5 ref access during render, 2 from Gap 7 setState in nested functions, 6 from Gap 9 known-incompatible/ESLint/useMemo/capitalized-call fixes, 10 from Gap 9 mutation tracking [delete ops + phi freeze + alias freeze + derivation chains + outer-scope property mutation + render helper detection], 4 from Phase 65 [hook-call capture freeze + hook-arg mutation + assign-ref hint], 1 from bailout-infer-mode).
Remaining achievable: ~3-10 of the remaining ~13 actionable fixtures. The
categorized gaps (1,3,6,7,8) account for ~7 fixtures; Gap 9 "Other" covers
~9 uncategorized fixtures requiring individual triage. The 15 Invariant/Todo
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
