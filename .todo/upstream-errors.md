# Upstream Errors -- Validation Gaps

> **Priority**: P2 (~7 actionable remaining fixtures)
> **Impact**: Nearly all upstream error fixtures resolved. Gaps 2-8 complete. Gap 1 has 1 remaining. Gap 9 has ~6 remaining.
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

~~**Count:** ~4 remaining (moved from Gap 9 overlap; alias + phi tracking now done)~~
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
- ~~`error.invalid-mutate-props-in-effect-fixpoint.js`~~ Resolved (2026-03-14)
- ~~`error.invalid-mutation-of-possible-props-phi-indirect.js`~~ Resolved (2026-03-14)
- `error.mutate-function-property.js` -- mutation of function object property (still in known-failures)

**Completed (2026-03-14, hook alias session):** 2 of 3 remaining frozen mutation fixtures resolved: `error.invalid-mutate-props-in-effect-fixpoint.js` and `error.invalid-mutation-of-possible-props-phi-indirect.js` now passing. 1 remains: `error.mutate-function-property.js` (mutation of function object property). Rust module: `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`.

### Gap 2: Validate Preserve Existing Memoization ✅

~~**Count:** 13 fixtures~~
~~**Upstream error:** "Compilation Skipped" (preserve-memo mode)~~
~~**Upstream:** `ValidatePreserveExistingMemoizationGuarantees.ts`~~

**Completed (2026-03-13):** Pipeline gate fixes for Pass 5 and Pass 61. Pass 5 (`drop_manual_memoization`) now preserves memo markers when `validate_preserve_existing_memoization_guarantees` is set (not just `enable_preserve`). Pass 61 now runs on both config flags. Error messages aligned with upstream ("Existing memoization could not be preserved..."). Pruned memoizations silently skipped. All 20 preserve-memo-validation error fixtures now passing, plus 11 bonus error fixtures from other categories. Rust modules: `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`, `crates/oxc_react_compiler/src/validation/validate_preserved_manual_memoization.rs`. +31 fixtures total (278 -> 309/1717).

### Gap 3: Exhaustive Deps Remaining ✅

~~**Count:** 2 remaining (6 fixed by extra-dep detection + mode gating, 2026-03-13)~~
~~**Upstream error:** "Missing/extra deps"~~

**Completed (2026-03-14):** Both remaining exhaustive deps fixtures resolved: `error.invalid-exhaustive-effect-deps-missing-only.js` and `error.sketchy-code-exhaustive-deps.js`. All 8 exhaustive deps validation-error fixtures now passing. Note: 5 additional exhaustive-deps fixtures remain in known-failures but are compilation divergences (P1 territory), not validation-error fixtures. Rust module: `crates/oxc_react_compiler/src/validation/validate_exhaustive_dependencies.rs`.

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

### Gap 6: Dynamic Hook Identity ✅

~~**Count:** 2 remaining (was 4; 2 resolved by SSA + conditional hook detection improvements)~~
~~**Upstream error:** "Hooks must be same function"~~

**Completed (2026-03-14):** Both remaining fixtures resolved via hook alias detection infrastructure: `error.invalid-conditional-call-aliased-hook-import.js` and `error.invalid-conditional-call-aliased-react-hook.js`. Implementation: `collect_hook_aliases()` in `program.rs` scans imports for aliased hooks (e.g., `import { useState as useMyState }`), stored in `EnvironmentConfig.hook_aliases`. The `is_hook()` closure in `validate_hooks_usage.rs` checks both the standard `use[A-Z]` naming convention and the alias set. Rule 5 (dynamic hook identity) detects unstable hook-named callees resolved through StoreLocal chains. All 4 original Gap 6 fixtures now passing. Rust modules: `crates/oxc_react_compiler/src/entrypoint/program.rs`, `crates/oxc_react_compiler/src/hir/environment.rs`, `crates/oxc_react_compiler/src/validation/validate_hooks_usage.rs`.

### Gap 7: setState During Render ✅

~~**Count:** 1 remaining (2 of 3 resolved)~~
~~**Upstream error:** "Cannot call setState during render"~~

**Completed (2026-03-14):** Final fixture `error.invalid-hoisting-setstate.js` resolved as part of the hook alias / validation sweep session. All 3 setState-during-render fixtures now passing. Rust module: `crates/oxc_react_compiler/src/validation/validate_no_set_state_in_render.rs`.

### Gap 8: Hoisting/TDZ ✅

~~**Count:** 1 actionable fixture (2 `todo-*` are upstream bugs, skippable)~~
~~**Upstream error:** "Cannot access variable before declared"~~

**Completed (2026-03-14):** The one actionable fixture (`error.invalid-hoisting-setstate.js`) was resolved as part of Gap 7 completion. The 2 `todo-functiondecl-hoisting` fixtures are upstream TODOs and should be skipped.

### Gap 9: Other Validation Errors

**Count:** ~7 remaining error fixtures in known-failures.txt
**What's needed:** These cover several sub-categories:
- **Frozen mutation** (1): `error.mutate-function-property.js` (Gap 1 remainder)
- **Type provider** (2): `error.invalid-type-provider-hooklike-module-default-not-hook.js`, `error.invalid-type-provider-nonhook-name-typed-as-hook.js`
- **Preserve-memo edge case** (1): `error.repro-preserve-memoization-inner-destructured-value-mistaken-as-dependency-later-mutation.js`
- **Mutable function as prop** (1): `error.invalid-pass-mutable-function-as-prop.js`
- **Other** (2): `error.call-args-destructuring-asignment-complex.js`, `error.dont-hoist-inline-reference.js`
- ~~**Mutation tracking** (~3)~~ Mostly resolved (moved to Gap 1; `mutate-function-property` remains)
- ~~**Ref naming heuristic** (2)~~ Resolved (`ref-like-name-not-Ref`, `ref-like-name-not-a-ref` both passing, 2026-03-14)
- ~~**Preserve-memo** `repro-preserve-memoization-inner-destructured-value-mistaken-as-dependency-mutated-dep`~~ Resolved (2026-03-14)
- ~~`invalid-mutate-global-*`~~ Resolved (outer-scope property mutation + render helper detection)
- ~~`not-useEffect-external-mutate`~~ Resolved
- ~~`invalid-return-mutable-function-from-hook`~~ Resolved (2026-03-14)
- ~~`hook-call-freezes-captured-identifier.tsx`~~ Resolved (Phase 65: hook-call capture freeze)
- ~~`hook-call-freezes-captured-memberexpr.jsx`~~ Resolved (Phase 65: hook-call capture freeze)
- ~~`assign-ref-in-effect-hint`~~ Resolved (Phase 65: assign-ref diagnostic)
- ~~`invalid-hook-function-argument-mutates-local-variable`~~ Resolved (Phase 65: moved to Gap 1 completed)
**Fixture gain estimate:** ~2-5 (type provider requires custom type system integration; others need individual analysis)
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
- **Hook alias detection + dynamic hook identity (2026-03-14):** Broad validation sweep resolving 33 fixtures:
  - `collect_hook_aliases()` in `program.rs` for import alias detection (e.g., `import { useState as useMyState }`)
  - `hook_aliases` field in `EnvironmentConfig`
  - `is_hook()` closure in `validate_hooks_usage.rs` checking both hook naming and aliases
  - Dynamic hook identity check (Rule 5) -- detects unstable hook-named callees via StoreLocal chain resolution
  - Hook alias support wired into all 4 existing hook rules (conditional calls, nested functions, hooks-as-values, dynamic identity)
  - Collateral gains: `rules-of-hooks/error.invalid-dynamic-hook-via-hooklike-local.js`, `rules-of-hooks/error.invalid-hook-as-prop.js`, `rules-of-hooks/error.invalid-hook-for.js`, `rules-of-hooks/error.invalid-hook-from-hook-return.js`, `rules-of-hooks/rules-of-hooks-0e2214abc294.js`, and 28 additional fixtures across frozen mutation, exhaustive deps, ref naming, hoisting, and compilation structure categories
  - Completes Gaps 1, 3, 6, 7, 8
  - 33 fixtures removed from known-failures.txt. +25 net (391 -> 416/1717).
  - Rust modules: `crates/oxc_react_compiler/src/entrypoint/program.rs`, `crates/oxc_react_compiler/src/hir/environment.rs`, `crates/oxc_react_compiler/src/validation/validate_hooks_usage.rs`, `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`.

## Total Fixture Gain Estimate

Achieved so far: ~130 upstream error fixtures resolved across all gaps. Gaps 2-8 are
fully complete. Gap 1 has 1 remaining fixture (`error.mutate-function-property.js`).
Gap 9 has ~6 remaining actionable fixtures (type provider x2, preserve-memo edge case,
pass-mutable-function-as-prop, call-args-destructuring, dont-hoist-inline-reference).
The remaining Invariant/Todo fixtures are upstream bugs and should be registered as
known skips.

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
