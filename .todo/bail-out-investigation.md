# Stage 2a: Bail-out Investigation Results

> Completed: 2026-03-25
> Starting pool: 108 "we bail, they compile" fixtures
> After fixes: 89 remaining (pre-Stage 4d), ~93 after Stage 4d (+4 IIFE false positives shifted in), **~60 remaining as of 2026-04-03** (per latest conformance breakdown: 26 frozen mutation, 8 ref access, 7 silent, rest other; +3 new false-positive bails from validateInferredDep scope dep resolution mismatch; -6 from Stage 2g error fixture sweep; -4 from Stage 2j Infer mode body heuristic — 4 infer-mode fixtures now correctly skipped)
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

## Remaining Bail-out Breakdown (115 fixtures as of 2026-04-03, was 119 — Stage 2j removed 4 infer-mode false positives)

| Error Category | Count | Fixable? | Notes |
|---------------|-------|----------|-------|
| Existing memo preservation (preserve-memo) | **55** (was 4; +51 from validateInferredDep regression) | **HIGH PRIORITY** | `validateInferredDep` scope dep resolution failures — scope deps resolve to SSA temps, not named variables. Fixing dep resolution would eliminate ~51 false positives. **SINGLE LARGEST BAIL-OUT CATEGORY.** |
| Frozen mutation (false positive) | 15 | MEDIUM | `InferMutableRanges` over-reports mutations on frozen values. Includes IIFE false positives from name-based freeze tracking. |
| Cannot reassign outside component | 11 | BLOCKED | `validateLocalsNotReassignedAfterRender` false positives. Stage 2f attempted and FAILED (-4 net). Requires DeclareContext/StoreContext HIR lowering. |
| (no error / silent) | 8 | MIXED | Various |
| Local variable reassignment | 6+1 | MEDIUM | Overlaps with "cannot reassign" |
| Cannot access refs during render | 6 | MEDIUM | `validateNoRefAccessInRender` false positives |
| Cannot call setState during render | 4 | MEDIUM | `validateNoSetStateInRender` false positives |
| Hooks as normal values | 3 | MEDIUM | Hooks validation false positives. `locally_declared_names` fix (2026-03-26) addresses LoadLocal vector; PropertyLoad callee-name vector remains. |
| setState in useEffect (synchronous) | 2 | HARD | |
| BuildHIR unsupported | 2 | MEDIUM | DefaultParam nonreorderable |
| Other (1 each) | 6 | VARIES | Various edge cases |

## Recommended Next Steps

### Stage 2j: Infer Mode Body Heuristic -- COMPLETE (+4, 2026-04-03)

**Completed 2026-04-03.** Added `body_has_hooks_or_jsx` function to `program.rs` to skip compiling functions in `CompilationMode::Infer` that don't contain hooks or JSX at the top level (matching upstream's `hasHooksOrJsx`). The function performs a shallow AST walk that descends into control flow but NOT into nested function expressions/arrows.

**Fixtures gained (4):** `dont-memoize-primitive-function-call-non-escaping.js`, `infer-skip-components-without-hooks-or-jsx.js`, `infer-no-component-nested-jsx.js`, `infer-no-component-obj-return.js`.

**Fixtures NOT gained (3):** `dont-memoize-primitive-function-call-non-escaping-useMemo.js` (has useMemo, correctly compiles), `should-bailout-without-compilation-infer-mode.js` (needs gating directive support), `valid-setState-in-useEffect-controlled-by-ref-value.js` (needs `@enableAllowSetStateFromRefsInEffects` directive support).

**Files:** `crates/oxc_react_compiler/src/entrypoint/program.rs` (`body_has_hooks_or_jsx`, `stmt_has_hooks_or_jsx`, `expr_has_hooks_or_jsx`, `call_is_hook`).

### Stage 1e: Dynamic Gating Parsing Fix -- COMPLETE (+3, harness fix)

**Completed 2026-03-26.** The 3 gating-mode "silent bail-out" fixtures were actually a conformance test harness parsing issue, not a compiler bug. The `@gating` directive was not being parsed correctly for dynamic import patterns. Fixing the harness parsing logic gained +3 fixtures.

**Fixtures fixed:** 3 gating fixtures (exact names TBD — these moved from "silent bail-out" to passing).

### ObjectExpression Computed Key Bail-out Removed -- COMPLETE (+2)

**Completed 2026-03-26.** Removed an overly aggressive bail-out in HIR lowering that rejected `ObjectExpression` nodes with computed keys. Upstream compiles these patterns successfully. Our bail-out was a false positive from an early conservatism guard.

### Empty Catch Handler Codegen Fix -- COMPLETE (+1)

**Completed 2026-03-26.** Fixed an additional catch handler codegen issue that, combined with the Stage 1d Phase 1 declaration placement fix, unblocked 1 more fixture. The catch handler now correctly emits `catch {}` in contexts where both ordering and empty-handler requirements are met.

### const vs let Keyword in StoreLocal Codegen -- COMPLETE (+0)

**Completed 2026-03-26.** Fixed codegen to emit `const` instead of `let` for `StoreLocal` instructions where the variable is never reassigned. No conformance gain because affected fixtures also differ in other ways (scope inference, naming), but this is a correctness improvement that will contribute to matches once those other differences are resolved.

### Gating Directive Comment Stripping -- COMPLETE (+2, 505->507)

**Completed 2026-03-26.** `codegen.rs` `apply_compilation` now filters `// @gating` and `// @dynamicGating` comment lines from compiled output when gating mode is active. Upstream's Babel plugin removes these annotations during compilation; our source-edit-based approach was preserving them. The fix iterates over output lines and suppresses those whose trimmed content starts with `@gating` or `@dynamicGating`. Fixtures gained: `gating/multi-arrow-expr-export-gating-test.js`, `gating/multi-arrow-expr-gating-test.js`.

### Hooks-as-Value False Positive Fix (locally_declared_names) -- COMPLETE (+0, correctness)

**Completed 2026-03-26.** `validate_hooks_usage.rs` Rule 3 (hooks-as-values check) now skips `LoadLocal` of names present in a `locally_declared_names` set. The set is populated by walking `DeclareLocal` and `Destructure` instructions (with recursive `collect_destructure_names` helper for nested destructuring). This prevents false bails on patterns like `let useFeature = makeObject()` where a locally-declared variable has a hook-like name but is not actually a hook import. No net conformance change because the 3 affected fixtures (listed in the "Hooks as normal values" row above) are also caught by the `PropertyLoad` callee-name check, which is a separate false positive. This fix is a correctness guard for future work.

**Note:** The 3 "Hooks as normal values" bail-outs from the table above are NOT fully resolved. The `locally_declared_names` fix addresses one vector (LoadLocal of locally-declared hook-like names). The remaining false positives come from PropertyLoad instructions where the property name looks like a hook (e.g., `obj.useHook`). Fixing the PropertyLoad check was attempted in this session but was net-zero. These 3 fixtures remain in the bail-out pool.

### Stage 2c (was 2b): Fix `_exp` Directive Handling -- COMPLETE

**Completed 2026-03-25.** Fixed handling of `@validateNoDerivedComputationsInEffects_exp` directive fixtures.
- 20 fixtures now compile instead of bailing
- Net conformance: +0 (all land in slots-DIFFER/MATCH pools)
- These fixtures are unblocked for future scope/codegen improvements
- **Key learning:** Bail-out fixes move fixtures between pools but don't directly increase conformance when output still differs

### Stage 2d: Fix frozen-mutation false positives (11 original + 4 new IIFE = ~15 fixtures) -- BLOCKED
- `InferMutableRanges` incorrectly reports mutations on frozen values
- Requires mutable range analysis refinements
- **NEW (post Stage 4d):** 4 additional IIFE-pattern false positives introduced by name-based freeze tracking:
  - `capturing-func-alias-captured-mutate-iife.js`
  - `capturing-func-alias-computed-iife.js`
  - `capturing-func-alias-mutate-iife.js`
  - `capturing-func-alias-property-iife.js`
  - These shifted from slots-MATCH/DIFFER to bail category. The name-based tracker sees mutations inside IIFEs as post-freeze mutations because it doesn't track scope boundaries.
  - **Fix approach:** Implement scoped name tracking that resets or excludes names within IIFE boundaries from freeze-after-mutation checks.
- **Risk:** HIGH — blocked, see blocker report below

### Blocker Report — Stage 2d Frozen-Mutation False-Positive Bails (2026-03-26)

**Approach attempted:** Three strategies were tried to eliminate the 26 false-positive MutateFrozen bails:

1. **IIFE detection improvement (two-pass per-block approach):** Improved IIFE detection to identify call instructions that target function expressions defined in the same block. This was kept as a safe code quality improvement, but it is a no-op for the 26 false positives — the root cause lies elsewhere.

2. **IIFE skip in Check 1 for MutateFrozen:** Attempted to skip MutateFrozen effects that originate from IIFE call instructions in Check 1 of `validate_no_mutation_after_freeze`. This had no effect because the false positives do not come from the IIFE call instruction itself.

3. **Cross-checking MutateFrozen against frozen_ids:** Attempted to filter MutateFrozen effects by verifying the mutated identifier is actually in the `frozen_ids` set. Result: **-2 regression** (lost 2 true positives that were correctly detected via transitive freeze status but whose identifiers were not directly in `frozen_ids`).

**Assumption that was wrong:** Assumed the false positives came from the IIFE call instruction itself. Actually, they come from `mutate(y)` where `y` is transitively MaybeFrozen via capture chains in the aliasing pass. The IIFE call is not the source of the bad effect — the aliasing pass's transitive freeze propagation is.

**What was discovered:** All 26 false positives come from Check 1 (MutateFrozen effects emitted by `infer_mutation_aliasing_effects`). Root cause: when a mutable container `y` captures frozen data through `y.x = x` (a PropertyStore or Capture effect), `y` becomes MaybeFrozen in the abstract state of the aliasing pass. Then any subsequent Mutate effect on `y` gets upgraded to MutateFrozen by the aliasing pass. The fix needs to happen in one of two places:
- The aliasing pass's `mutate()` method (around line 225 of `infer_mutation_aliasing_effects.rs`), where Mutate effects are upgraded to MutateFrozen based on transitive freeze status
- The PropertyStore/Capture effect handling that propagates frozen status from captured values to their containers

**Regression details:** Cross-checking MutateFrozen against `frozen_ids` caused -2 regression — lost 2 true positives where the identifier was correctly flagged via transitive freeze status but was not directly present in the `frozen_ids` set.

**Prerequisites for a successful attempt:**

- Either fix the aliasing pass's transitive freeze propagation to distinguish "container holds frozen data" from "container itself is frozen" (complex, high regression risk — the aliasing pass is a core pass that affects many downstream analyses)
- Or build a more sophisticated validator-level filter in `validate_no_mutation_after_freeze` that can distinguish direct freezes (parameter is frozen) from transitive freezes (container captured a frozen value) without losing the true positives that rely on transitive detection

**Useful findings to carry forward:**

- All 26 false positives are Check 1 (MutateFrozen from `infer_mutation_aliasing_effects`), not Check 2/3/4
- The IIFE detection improvement (two-pass per-block approach) was kept — it is safe and improves code quality even though it does not fix these specific false positives
- `infer_mutation_aliasing_effects.rs` line ~225 is where `mutate()` upgrades Mutate to MutateFrozen based on abstract state
- The `frozen_ids` set in `validate_no_mutation_after_freeze.rs` tracks directly frozen identifiers (params, context) but NOT transitively frozen containers — this is why the cross-check approach loses true positives
- The upstream aliasing pass likely has a more nuanced freeze propagation model that does not over-promote containers to MaybeFrozen

**Do NOT attempt again until:** Either (a) the aliasing pass's freeze propagation semantics are better understood by detailed comparison with the upstream TypeScript `InferMutationAliasingEffects` pass, or (b) a mechanism exists to distinguish direct vs transitive freeze status in the validator without regressing true positives.

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

## validateInferredDep False Positive Bails (+3 new, 2026-03-26)

The `validateInferredDep` implementation in `validate_preserved_manual_memoization.rs` introduced 3 new false-positive bail-outs. These occur because scope dependency IdentifierIds resolve to SSA temporaries instead of the original named variables. When the dep name (e.g., `t1`) doesn't match any manual memo dep name (e.g., `props.x`), the validation incorrectly fires `CannotPreserveMemoization`.

**Root cause:** Scope dep IdentifierIds after SSA don't map back to original variable names. See Stage 4b blocker report in `index.md` for full details.

**Impact:** 3 fixtures moved from "both compile" to "we bail, they compile" category. These are a known regression from the validateInferredDep implementation and will be resolved when scope dep resolution is fixed.

**These false positives are expected to disappear when:** The scope dep resolution blocker is addressed (mapping SSA temp IdentifierIds back to original named variable paths).

### Blocker Report — Stage 2i (Preserve-Memo False Bails) (2026-03-26)

**Approach attempted:** Three strategies to reduce the 55 false-positive "Existing memoization could not be preserved" bails:

1. **Build temp resolution map before `inline_load_local_temps`:** Hypothesis was that building the pre-inline temporaries map earlier would capture more LoadLocal/PropertyLoad chains before they were inlined away. Result: no effect — the map contained the same entries regardless of timing, because the relevant instruction chains were never present in the first place (the operands are CallExpression/MethodCall/BinaryExpression/Destructure results, not LoadLocal targets).

2. **Skip unnamed deps in `propagate_scope_dependencies_hir`:** Hypothesis was that filtering out deps with unnamed/temp identifiers at the source (propagation) would prevent them from reaching the validation pass. Result: **-15 conformance regression**. Many legitimate scope dependencies are unnamed at propagation time and get named later by `promote_used_temporaries`. Filtering them out at propagation time removes real dependencies that downstream passes need.

3. **Skip compiler temp names ("tN") in `resolve_scope_dep` validation only:** Hypothesis was that deps whose resolved name matches the pattern `t\d+` (compiler temporaries) could be safely excluded from the validateInferredDep mismatch check. Result: reduced false bails from 55 to 7, but caused **-31 conformance regression** from error.* fixtures. Root cause: many error.* fixtures (that should bail with upstream errors) currently bail "by accident" because their scope deps resolve to "tN" temps, triggering the validateInferredDep mismatch. This is the "right behavior for the wrong reason" — skipping "tN" deps in validation causes those fixtures to stop bailing, and they then produce compiled output that doesn't match the expected error output.

**Root cause:** 4000+ unnamed deps are created in `propagate_scope_dependencies_hir` because operands reference unnamed instruction results (CallExpression, MethodCall, StoreLocal, BinaryExpression, Destructure results). These are NOT LoadLocal/PropertyLoad targets, so `temp_map` (the pre-inline temporaries resolution map) cannot resolve them back to named variables. The `promote_used_temporaries` pass then names these deps with compiler-generated names like "t0", "t1", etc. Even when the 55 false bails are eliminated (approach 3), the 48 de-bailed fixtures have codegen differences (slot mismatches) and do not pass conformance — they were hidden behind the false bail.

**Assumption that was wrong:** Assumed that fixing false bails would yield net conformance improvement. In reality: (a) the de-bailed fixtures have codegen differences that prevent them from passing, and (b) ~31 error.* fixtures rely on the "tN" dep mismatch to bail correctly (right outcome, wrong mechanism).

**Regression details:**
- Approach 2 (skip unnamed in propagation): -15 net conformance
- Approach 3 (skip "tN" in validation): 55→7 false bails eliminated, but -31 net conformance from error.* fixtures that stop bailing

**Prerequisites for a successful attempt:**
- Codegen quality must improve for preserve-memo fixtures so that de-bailed fixtures actually pass conformance (slot mismatches must be resolved first)
- OR: `propagate_scope_dependencies_hir` must be enhanced to resolve more operand types through `temp_map` — handle CallExpression results, phi outputs, Destructure outputs, BinaryExpression results, etc. — so that deps resolve to real variable names instead of compiler temps
- OR: error.* fixtures that currently bail "by accident" via "tN" dep mismatch must be made to bail via their correct validation path (e.g., frozen-mutation, ref-access, reassignment validations), so that fixing the false bails does not regress them

**Useful findings to carry forward:**
- `propagate_scope_dependencies_hir` in `propagate_dependencies.rs` creates 4000+ unnamed deps per fixture — this is the upstream divergence point
- `temp_map` (built by `build_temporaries_map_from_hir`) only covers LoadLocal → PropertyLoad chains; it does NOT cover CallExpression, MethodCall, StoreLocal, BinaryExpression, or Destructure instruction results
- `promote_used_temporaries` (Pass 29) names unnamed identifiers with "tN" pattern — these are the deps that cause false bails
- The 48 de-bailed fixtures (from approach 3) all land in slots-DIFFER, not slots-MATCH — they need scope inference fixes before they can pass
- The 31 error.* regressions from approach 3 are a mix of preserve-memo, frozen-mutation, ref-access, and reassignment errors that upstream detects via their respective validators, but our compiler only catches via the accidental "tN" dep mismatch in validateInferredDep

**Do NOT attempt again until:** Either (a) codegen quality improves for preserve-memo fixtures (slot mismatches resolved), OR (b) `propagate_scope_dependencies_hir` is enhanced to resolve more operand types through temp_map, OR (c) the error.* fixtures that bail "by accident" are made to bail via their correct validation paths. Without one of these prerequisites, any attempt to reduce the 55 false bails will cause a net conformance regression.

### Blocker Report — Stage 2i Attempt 4: Temp-name-skipping in validateInferredDep (2026-04-03)

**Approach attempted:** In `validate_preserved_manual_memoization.rs`, skip validation of inferred deps whose resolved name matches a temp pattern (`t0` through `t99`). Hypothesis: temp-named deps are always false positives and can be safely excluded from the comparison.

**Result:** **-56 regression** (540->484). Worst result of all 4 approaches. Zero new passes. All regression from error.* fixtures losing their bail path.

**Why it's worse than attempt 3 (-31):** Attempt 3 skipped all "tN" names in `resolve_scope_dep` validation. Attempt 4 used the same approach but at a different level (the `validateInferredDep` comparison itself). The broader filtering caught more temp-named deps, which means more error.* fixtures lost their bail path. The increased regression demonstrates that the problem scales: the more aggressively you filter temp names, the worse the regression.

**Conclusion:** ALL skip/filter approaches to preserve-memo false-positive bails are fundamentally flawed. The false-positive and true-positive bails share the same mechanism (temp-named dep mismatch). You cannot distinguish them without fixing the underlying scope dep resolution problem (SSA temp -> named variable mapping).

**Do NOT attempt ANY further skip/filter approaches.** The only viable path forward is fixing scope dep resolution so that validateInferredDep can correctly compare deps by their original source-level names.

---

## Stage 2g: Error Fixture Bail-out Sweep (2026-03-26, +6 fixtures, 499->505)

Four new bail-out validations targeting error.* fixtures in known-failures.

### Fix 5: Duplicate fbt/fbs sub-tag detection (+2)

**Fixtures:** `fbt/error.todo-fbt-unknown-enum-value.js`, `fbt/error.todo-multiple-fbt-plural.tsx`
**File:** `validate_no_unsupported_nodes.rs` — `check_fbt_duplicate_tags`
**Upstream:** `Todo: Support duplicate fbt tags`

Two-pass analysis: Pass 1 collects identifiers named `fbt` or `fbs` via `LoadLocal`/`LoadContext`/`LoadGlobal`. Pass 2 counts `_enum`/`_plural`/`_pronoun` MethodCall sub-tags on those identifiers. If any sub-tag type appears 2+ times, bails with a Todo error.

**Important note:** `import fbt from 'fbt'` creates a `LoadLocal` instruction, not `LoadGlobal`, because `fbt` is not in the built-in globals list (`GlobalCollector`). The implementation handles both paths, but future fbt work must be aware of this distinction.

### Fix 6: Ref-to-function detection (+1)

**Fixture:** `error.invalid-pass-ref-to-function.js`
**File:** `validate_no_ref_access_in_render.rs`
**Upstream:** `Cannot access refs during render. Passing a ref to a function may read its value during render.`

Added a check in Pass 2 for `CallExpression` instructions: if any argument is a tracked ref identifier (by ID, by `Type::Ref`, or by name matching `is_ref_name`/`ref_names`) AND the callee is not a hook (determined by `is_hook_name`), the function bails with a ref-access-in-render error. MethodCall is excluded (method calls on objects are a different pattern).

### Fix 7: Self-referencing const declarations (+1)

**Fixture:** `error.dont-hoist-inline-reference.js`
**File:** `validate_no_unsupported_nodes.rs` — `check_self_referencing_declarations`
**Upstream:** `Todo: [hoisting] EnterSSA: Expected identifier to be defined before being used`

Detects `const x = identity(x)` pattern: for each `DeclareLocal` with `InstructionKind::Const`, scans forward until the matching `StoreLocal`, checking if any `LoadLocal` references the same `IdentifierId`. Fires a Todo error if found. Only checks non-temp identifiers (skips `t0`, `t1`, ...). Stops at `DeclareLocal`/`Destructure` boundaries to avoid scanning too far.

**Limitation:** Only handles `Const` kind, not `Let`. JavaScript `let` also has TDZ semantics, but no current conformance fixtures test this. See Deferred section in index.md.

**Regression avoided:** An initial broader version that also checked function params and destructured bindings caused -11 regression. The final version is scoped to `DeclareLocal Const` with exact `IdentifierId` matching.

### Fix 8: Dynamic gating invalid identifier validation (+2)

**Fixtures:** `gating/dynamic-gating-invalid-identifier-nopanic.js`, `gating/error.dynamic-gating-invalid-identifier.js`
**File:** `program.rs` — `is_valid_js_identifier`

Validates the condition in `'use memo if(cond)'` / `'use forget if(cond)'` directives against JavaScript identifier rules: must start with ASCII letter/`_`/`$`, subsequent chars must be ASCII alphanumeric/`_`/`$`, must not be a JS reserved keyword or literal (`true`, `false`, `null`, `undefined`, etc.). When the condition fails validation, compilation bails with an error instead of attempting to lower the invalid condition.

### Attempted but REJECTED: 0-slot codegen

**Result: -52 regression (505->453)**. Attempted emitting passthrough code (no `_c()` wrapper) for functions producing 0 cache slots. Failed because many 0-slot expected outputs contain structural transformations (extracted arrow functions, renamed variables) that differ from simple passthrough. Documented in index.md Deferred section.
