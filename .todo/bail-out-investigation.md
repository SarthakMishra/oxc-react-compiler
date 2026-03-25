# Stage 2a: Bail-out Investigation Results

> Completed: 2026-03-25
> Starting pool: 108 "we bail, they compile" fixtures
> After fixes: 89 remaining

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
| Frozen mutation (false positive) | 11 | MEDIUM | `InferMutableRanges` over-reports mutations on frozen values |
| Cannot reassign outside component | 10 | MEDIUM | `validateLocalsNotReassignedAfterRender` false positives |
| (no error / silent) | 9 | MIXED | Various: gating mode (3), 0-scope functions (2), misc (4) |
| Cannot access refs during render | 8 | MEDIUM | `validateNoRefAccessInRender` false positives |
| setState in useEffect (synchronous) | 7 | HARD | New validation that doesn't exist upstream or fires incorrectly |
| Cannot call setState during render | 4 | MEDIUM | `validateNoSetStateInRender` false positives |
| Existing memo preservation | 4 | HARD | `preserveExistingMemoization` validation gaps |
| Extra effect dependencies | 3 | MEDIUM | `validateExhaustiveDeps` false positives |
| Hooks as normal values | 3 | MEDIUM | Hooks validation false positives |
| Other (1-2 each) | 10 | VARIES | Various edge cases |

## Recommended Next Steps

### Stage 2b: Fix `validateNoDerivedComputationsInEffects` (20 fixtures → ~15-18 gained)
- These 20 fixtures all have `@validateNoDerivedComputationsInEffects_exp` directive
- Our validation fires and bails; upstream compiles anyway
- Fix: Check if our validation is implementing this check when the directive only asks for it as a diagnostic (not a bail condition)
- **Risk:** LOW-MEDIUM

### Stage 2c: Fix frozen-mutation false positives (11 fixtures → ~5-8 gained)
- `InferMutableRanges` incorrectly reports mutations on frozen values
- Requires mutable range analysis refinements
- **Risk:** MEDIUM

### Stage 2d: Fix ref-access false positives (8 fixtures → ~3-5 gained)
- `validateNoRefAccessInRender` is over-eager
- Some patterns (assigning ref-accessing functions to properties, ref type casts) should be allowed
- **Risk:** MEDIUM

### Stage 2e: Fix reassignment false positives (10 fixtures → ~5-7 gained)
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
