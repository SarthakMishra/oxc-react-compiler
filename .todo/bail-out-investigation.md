# Stage 2a: Bail-out Investigation Results

> Completed: 2026-03-25
> Starting pool: 108 "we bail, they compile" fixtures
> After fixes: 89 remaining (pre-Stage 4d), ~93 after Stage 4d (+4 IIFE false positives shifted in), **80 remaining as of 2026-03-26** (per latest conformance breakdown: 26 frozen mutation, 8 ref access, 7 silent, rest other)
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

### Fix 2: Remove `has_known_incompatible_import` file-level bail, then re-enable as per-function bail (+3 fixtures moved to compile initially, +3 net passing after re-enable)
- **Root cause:** We bailed entire files importing from `ReactCompilerKnownIncompatibleTest`. Upstream still compiles but emits per-function diagnostics.
- **Initial fix (2026-03-25):** Removed the file-level bail. Retained function/constant for future per-function diagnostics. +0 net at the time.
- **Follow-up (2026-03-26):** Re-enabled as a per-function bail-out matching upstream behavior. Upstream DOES bail per-function on known-incompatible imports; the initial full removal was too aggressive. Re-enabling as per-function bail gained **+3 net passing fixtures** (UPSTREAM ERROR fixtures that need us to bail to pass conformance).

### Fix 3: Refine `has_compiler_runtime_import` check (+1 fixture moved to compile, +0 net passing)
- **Root cause:** We bailed on ANY import from `react/compiler-runtime`. The `babel-existing-react-runtime-import.js` fixture imports `{someImport}` (not the compiler cache).
- **Fix:** Only bail when `c` or `useMemoCache` is specifically imported.

### Fix 4: Remove `has_eslint_suppression_for_rules` file-level bail, then re-enable as per-function bail (+1 fixture passing)
- **Root cause:** We bailed entire files with custom eslint suppression rules. Upstream bails per-function.
- **Initial fix (2026-03-25):** Removed the file-level bail. +1 net at the time.
- **Follow-up (2026-03-26):** Re-enabled as a custom ESLint suppression per-function bail matching upstream behavior. The per-function bail correctly bails individual functions that have ESLint suppression annotations, rather than bailing the entire file. This gained **+1 net passing fixture** (the suppression bail fixture itself now correctly bails).

## Remaining Bail-out Breakdown (80 fixtures as of 2026-03-26, was 89)

| Error Category | Count | Fixable? | Notes |
|---------------|-------|----------|-------|
| Values derived from props/state (effect-derived-computations validation) | 20 | YES | New validation `validateNoDerivedComputationsInEffects` fires incorrectly; upstream compiles despite this validation |
| Frozen mutation (false positive) | 26 (per latest breakdown) | MEDIUM | `InferMutableRanges` over-reports mutations on frozen values. Includes 4 IIFE false positives from name-based freeze tracking. Note: Stage 4d follow-up (destructure freeze propagation + Check 4b) improved true-positive detection but did not address false positives. |
| Cannot reassign outside component | 10 | MEDIUM | `validateLocalsNotReassignedAfterRender` false positives |
| (no error / silent) | 6 (was 9, 3 gating fixed) | MIXED | Various: ~~gating mode (3)~~ fixed in Stage 1e (conformance harness issue), 0-scope functions (2), misc (4) |
| Cannot access refs during render | 8 | MEDIUM | `validateNoRefAccessInRender` false positives. Note: separate from Stage 4e-B ref-access *detection* fixes (where we fail to bail on upstream-error fixtures). |
| setState in useEffect (synchronous) | 7 | HARD | New validation that doesn't exist upstream or fires incorrectly |
| Cannot call setState during render | 4 | MEDIUM | `validateNoSetStateInRender` false positives |
| Existing memo preservation | 4 | HARD | `preserveExistingMemoization` validation gaps |
| Extra effect dependencies | 3 | MEDIUM | `validateExhaustiveDeps` false positives |
| Hooks as normal values | 3 | MEDIUM | Hooks validation false positives |
| Other (1-2 each) | 10 | VARIES | Various edge cases |

## Recommended Next Steps

### Stage 1e: Dynamic Gating Parsing Fix -- COMPLETE (+3, harness fix)

**Completed 2026-03-26.** The 3 gating-mode "silent bail-out" fixtures were actually a conformance test harness parsing issue, not a compiler bug. The `@gating` directive was not being parsed correctly for dynamic import patterns. Fixing the harness parsing logic gained +3 fixtures.

**Fixtures fixed:** 3 gating fixtures (exact names TBD — these moved from "silent bail-out" to passing).

### ObjectExpression Computed Key Bail-out Removed -- COMPLETE (+2)

**Completed 2026-03-26.** Removed an overly aggressive bail-out in HIR lowering that rejected `ObjectExpression` nodes with computed keys. Upstream compiles these patterns successfully. Our bail-out was a false positive from an early conservatism guard.

### Empty Catch Handler Codegen Fix -- COMPLETE (+1)

**Completed 2026-03-26.** Fixed an additional catch handler codegen issue that, combined with the Stage 1d Phase 1 declaration placement fix, unblocked 1 more fixture. The catch handler now correctly emits `catch {}` in contexts where both ordering and empty-handler requirements are met.

### const vs let Keyword in StoreLocal Codegen -- COMPLETE (+0)

**Completed 2026-03-26.** Fixed codegen to emit `const` instead of `let` for `StoreLocal` instructions where the variable is never reassigned. No conformance gain because affected fixtures also differ in other ways (scope inference, naming), but this is a correctness improvement that will contribute to matches once those other differences are resolved.

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

### Stage 4d: Frozen-mutation false negatives -- COMPLETE (+10 net, 426->435 initial, +1 follow-up 452->453)

Completed 2026-03-25 (initial), updated 2026-03-26 (follow-up).

**Initial approach (2026-03-25):** Track frozen identifiers by name (not just IdentifierId) to solve cross-scope identity mismatches where the same logical variable has different IdentifierIds in different scopes.

**Initial results:**
- 7 of 9 planned fixtures fixed + 2 bonus = 9 total gained
- 9 fixtures shifted from slots-MATCH/DIFFER to bail (IIFE false positives + other side effects of broader freeze tracking)

**Follow-up (2026-03-26): Destructure freeze propagation + Check 4b effect callback analysis (+1, 452->453)**

Two improvements in `validate_no_mutation_after_freeze.rs`:

1. **Destructure freeze propagation:** When a frozen value (e.g., props param) is destructured via `Destructure` instruction, the output bindings now inherit frozen status. Previously only top-level parameter names were tracked via `param_names`, missing destructured fields like `{foo}` from `Component({foo})`. The fix iterates over all `Destructure` instructions and propagates frozen status from the destructured value to each output binding.

2. **Check 4b effect callback analysis:** Previously Check 4 (mutation-after-freeze in function expressions) skipped ALL effect callbacks unconditionally. The correct upstream behavior checks effect callbacks for prop/context mutations while excluding ref mutations. The fix re-enables the check for effect callbacks but filters out ref-named identifiers using `is_ref_name` (imported from `validate_no_ref_access_in_render.rs`, made `pub(crate)` for cross-module reuse). This ensures `ref.current = x` in effects does not false-positive, while `props.x = y` in effects correctly errors.

**Fixtures gained in follow-up:**
- `error.assign-ref-in-effect-hint.js` — effect callback now correctly detects mutation of frozen (non-ref) value

**Remaining 1 planned fixture:**
- `error.invalid-jsx-captures-context-variable.js` — complex JSX capture pattern needing deeper capture analysis

**Trade-off:** Name-based tracking is coarser than IdentifierId-based tracking. It correctly catches more true positives (the 10 gained) but also catches 4 false positives on IIFE patterns. Net impact is positive.

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

### Stage 4e-D: Todo-Bail Fixtures — PARTIALLY COMPLETE (+3, 450->453)

Completed 2026-03-26. Fixed 3 of 10 todo-bail fixtures from the "we compile, they don't" category:

**Fixtures fixed:**
- `repro-declaration-for-all-identifiers.js` — for-in-try detection via Terminal::For
- `repro-for-loop-in-try.js` — same Terminal::For detection
- `repro-nested-try-catch-in-usememo.js` — file-level bail propagation (ANY_FUNCTION_BAILED thread-local)

**Cross-cutting fix: file-level bail propagation.** Added `ANY_FUNCTION_BAILED` thread-local flag in `program.rs` that propagates any per-function bail-out to the file level. This matches upstream behavior where a file-level bail from any nested function means the entire file is treated as "not transformed." This mechanism is reusable and will automatically benefit future per-function bail-outs that affect file-level transformation status.

**7 remaining todo-bail fixtures:**
- `optional-call-chain-in-ternary.ts` — optional chaining inside ternary in try block, not detected by current validation
- `todo-optional-call-chain-in-optional.ts` — same pattern
- `propagate-scope-deps-hir-fork/todo-optional-call-chain-in-optional.ts` — same pattern (duplicate in subfolder)
- `error.dont-hoist-inline-reference.js` — hoisting validation gap, not investigated
- ~3 others (need re-enumeration)

**New gaps discovered:**
1. **Optional chain in ternary detection:** Our validation does not detect `?.()` or `?.` inside ternary expressions within try blocks. Upstream bails on this pattern. Requires new detection in `validate_no_unsupported_nodes.rs` or `build.rs`.
2. **Hoisting inline reference:** `error.dont-hoist-inline-reference.js` — upstream error not replicated, needs investigation.

## Silent Bail-out Detail (9 remaining)

1. `babel-existing-react-runtime-import.js` — imports `{someImport}` from runtime (our refined check still correctly bails because the expected output has `c as _c` import, meaning upstream adds it alongside the existing import; we'd need smarter import merging)
2. ~~`gating/infer-function-expression-React-memo-gating.js` — `@gating` mode not supported~~ ✅ Fixed in Stage 1e (conformance harness parsing issue)
3. ~~`gating/invalid-fnexpr-reference.js` — `@gating` mode not supported~~ ✅ Fixed in Stage 1e (conformance harness parsing issue)
4. `infer-functions-component-with-ref-arg.js` — `@compilationMode:"infer"`, function with ref arg not detected as compilable
5. `unused-object-element-with-rest.js` — 0 scopes survive pipeline (scope inference gap)
6. `invalid-jsx-in-catch-in-outer-try-with-catch.js` — try-catch in HIR lowering issue
7. `invalid-jsx-in-try-with-catch.js` — try-catch in HIR lowering issue
8. `valid-set-state-in-useEffect-from-ref.js` — setState-in-effect validation fires
9. `valid-setState-in-effect-from-ref-arithmetic.js` — same
(Note: some may have shifted between categories after the lint-mode fix)
