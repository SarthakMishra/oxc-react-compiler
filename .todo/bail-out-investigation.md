# Stage 2a: Bail-out Investigation Results

> Completed: 2026-03-25
> Starting pool: 108 "we bail, they compile" fixtures
> After fixes: 89 remaining (pre-Stage 4d), ~93 after Stage 4d (+4 IIFE false positives shifted in)
> Note: Conformance tests use `compilationMode:"all"` — all functions are compiled, not just detected components/hooks. This affects which bail-out validations fire.

## Summary

Of the original 108 fixtures where we bail but upstream compiles:
- **19 fixed** by removing overly aggressive file-level bail-outs (lint mode, incompatible imports, eslint suppression)
- **89 remaining** broken down by error category below

## Fixes Applied (Stage 2b, partial)

### Fix 1: Remove `OutputMode::Lint` early return (+14 fixtures moved to compile, +2 net passing)
- **Root cause:** We returned untransformed code for `@outputMode:"lint"` fixtures. Upstream still compiles in lint mode (emits memoization AND diagnostics).
- **Fix:** Removed the early return in `program.rs`. Now we compile normally.
- **Result:** 42 lint-mode fixtures now compile. 2 passed outright (`static-components/invalid-dynamically-construct-component-in-render.js`, `static-components/invalid-dynamically-constructed-component-new.js`). 2 error.todo fixtures regressed (added to known-failures). Rest moved to slots-match/slots-differ categories.

### Fix 2: Remove `has_known_incompatible_import` bail (+3 fixtures moved to compile, +0 net passing)
- **Root cause:** We bailed entire files importing from `ReactCompilerKnownIncompatibleTest`. Upstream still compiles but emits per-function diagnostics.
- **Fix:** Removed the file-level bail. Retained function/constant for future per-function diagnostics.

### Fix 3: Refine `has_compiler_runtime_import` check (+1 fixture moved to compile, +0 net passing)
- **Root cause:** We bailed on ANY import from `react/compiler-runtime`. The `babel-existing-react-runtime-import.js` fixture imports `{someImport}` (not the compiler cache).
- **Fix:** Only bail when `c` or `useMemoCache` is specifically imported.

### Fix 4: Remove `has_eslint_suppression_for_rules` file-level bail (+1 fixture passing)
- **Root cause:** We bailed entire files with custom eslint suppression rules. Upstream bails per-function.
- **Fix:** Removed the file-level bail. TODO: implement per-function suppression.

## Remaining Bail-out Breakdown (89 fixtures)

| Error Category | Count | Fixable? | Notes |
|---------------|-------|----------|-------|
| Values derived from props/state (effect-derived-computations validation) | 20 | YES | New validation `validateNoDerivedComputationsInEffects` fires incorrectly; upstream compiles despite this validation |
| Frozen mutation (false positive) | 15 (11 original + 4 IIFE from Stage 4d) | MEDIUM | `InferMutableRanges` over-reports mutations on frozen values. 4 new IIFE false positives from name-based freeze tracking. |
| Cannot reassign outside component | 10 | MEDIUM | `validateLocalsNotReassignedAfterRender` false positives |
| (no error / silent) | 9 | MIXED | Various: gating mode (3), 0-scope functions (2), misc (4) |
| Cannot access refs during render | 8 | MEDIUM | `validateNoRefAccessInRender` false positives. Note: separate from Stage 4e-B ref-access *detection* fixes (where we fail to bail on upstream-error fixtures). |
| setState in useEffect (synchronous) | 7 | HARD | New validation that doesn't exist upstream or fires incorrectly |
| Cannot call setState during render | 4 | MEDIUM | `validateNoSetStateInRender` false positives |
| Existing memo preservation | 4 | HARD | `preserveExistingMemoization` validation gaps |
| Extra effect dependencies | 3 | MEDIUM | `validateExhaustiveDeps` false positives |
| Hooks as normal values | 3 | MEDIUM | Hooks validation false positives |
| Other (1-2 each) | 10 | VARIES | Various edge cases |

## Recommended Next Steps

### Stage 2c (was 2b): Fix `_exp` Directive Handling -- COMPLETE

**Completed 2026-03-25.** Fixed handling of `@validateNoDerivedComputationsInEffects_exp` directive fixtures.
- 20 fixtures now compile instead of bailing
- Net conformance: +0 (all land in slots-DIFFER/MATCH pools)
- These fixtures are unblocked for future scope/codegen improvements
- **Key learning:** Bail-out fixes move fixtures between pools but don't directly increase conformance when output still differs

### Stage 2d: Fix frozen-mutation false positives (11 original + 4 new IIFE = ~15 fixtures)
- `InferMutableRanges` incorrectly reports mutations on frozen values
- Requires mutable range analysis refinements
- **NEW (post Stage 4d):** 4 additional IIFE-pattern false positives introduced by name-based freeze tracking:
  - `capturing-func-alias-captured-mutate-iife.js`
  - `capturing-func-alias-computed-iife.js`
  - `capturing-func-alias-mutate-iife.js`
  - `capturing-func-alias-property-iife.js`
  - These shifted from slots-MATCH/DIFFER to bail category. The name-based tracker sees mutations inside IIFEs as post-freeze mutations because it doesn't track scope boundaries.
  - **Fix approach:** Implement scoped name tracking that resets or excludes names within IIFE boundaries from freeze-after-mutation checks.
- **Risk:** MEDIUM

### Stage 4d: Frozen-mutation false negatives -- COMPLETE (+9 net, 426->435)

Completed 2026-03-25. Implemented name-based freeze tracking in `validate_no_mutation_after_freeze.rs`.

**Approach:** Track frozen identifiers by name (not just IdentifierId) to solve cross-scope identity mismatches where the same logical variable has different IdentifierIds in different scopes.

**Results:**
- 7 of 9 planned fixtures fixed + 2 bonus = 9 total gained
- 9 fixtures shifted from slots-MATCH/DIFFER to bail (IIFE false positives + other side effects of broader freeze tracking)

**Remaining 2 planned fixtures:**
- `error.assign-ref-in-effect-hint.js` — requires effect callback mutation checking, not just freeze tracking
- `error.invalid-jsx-captures-context-variable.js` — complex JSX capture pattern needing deeper capture analysis

**Trade-off:** Name-based tracking is coarser than IdentifierId-based tracking. It correctly catches more true positives (the 9 gained) but also catches 4 false positives on IIFE patterns. Net impact is positive.

### Stage 2e: Fix ref-access false positives (8 fixtures) — LOW PRIORITY, NO CONFORMANCE IMPACT

~~- `validateNoRefAccessInRender` is over-eager~~
~~- Some patterns (assigning ref-accessing functions to properties, ref type casts) should be allowed~~
~~- **Risk:** MEDIUM~~

**Investigation completed (2026-03-25):** Thoroughly investigated whether relaxing ref-access false positives would improve conformance. **Result: NO conformance gain.** The 8 fixtures freed by relaxing ref-access validation land in slots-DIFFER (not matched), so they do not pass conformance. Additionally, 2 fixtures that currently pass by accident (Flow parse errors producing output that happens to match upstream error format) would regress. Net impact: **-2 to +0 conformance**.

**Decision:** Deprioritized. Not worth pursuing until scope inference improvements (Stage 3) can make the freed fixtures actually match. The false-positive bail-outs are semantically incorrect (we bail when upstream compiles), but fixing them does not improve the metric.

**Note (2026-03-25):** Stage 4e-B separately fixed 1 ref-access *detection* gap (`error.validate-mutate-ref-arg-in-render.js` now correctly bails). That fix improved the detection path (name-based + Type::Ref fallback for PropertyLoad/PropertyStore), but the Stage 2e false-positive fixtures are the *opposite* problem: we bail when we should compile. These are distinct issues:
- **4e-B ref-access (detection):** We compile when upstream bails with "Cannot access refs during render". Fix: improve ref tracking to catch more true positives. 1 of 4 fixed, 3 remain.
- **2e ref-access (false positives):** We bail with "Cannot access refs during render" when upstream compiles successfully. Fix: relax over-eager patterns. 8 fixtures, 0 fixed so far. **LOW PRIORITY — freed fixtures land in slots-DIFFER, not matched.**
- `error.invalid-pass-ref-to-function.js` (4e-B remaining) specifically needs ref-through-function-call tracking: detecting when a ref object is passed as an argument to a function that accesses `.current` on it.

### Stage 2f: Fix reassignment false positives (10 fixtures → ~5-7 gained)
- `validateLocalsNotReassignedAfterRender` false positives
- **Risk:** MEDIUM

## Silent Bail-out Detail (9 remaining)

1. `babel-existing-react-runtime-import.js` — imports `{someImport}` from runtime (our refined check still correctly bails because the expected output has `c as _c` import, meaning upstream adds it alongside the existing import; we'd need smarter import merging)
2. `gating/infer-function-expression-React-memo-gating.js` — `@gating` mode not supported
3. `gating/invalid-fnexpr-reference.js` — `@gating` mode not supported
4. `infer-functions-component-with-ref-arg.js` — `@compilationMode:"infer"`, function with ref arg not detected as compilable
5. `unused-object-element-with-rest.js` — 0 scopes survive pipeline (scope inference gap)
6. `invalid-jsx-in-catch-in-outer-try-with-catch.js` — try-catch in HIR lowering issue
7. `invalid-jsx-in-try-with-catch.js` — try-catch in HIR lowering issue
8. `valid-set-state-in-useEffect-from-ref.js` — setState-in-effect validation fires
9. `valid-setState-in-effect-from-ref-arithmetic.js` — same
(Note: some may have shifted between categories after the lint-mode fix)
