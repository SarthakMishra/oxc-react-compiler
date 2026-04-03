# Stage 2a: Bail-out Investigation Results

> Completed: 2026-03-25
> Starting pool: 108 "we bail, they compile" fixtures
> After fixes: 89 remaining (pre-Stage 4d), ~93 after Stage 4d (+4 IIFE false positives shifted in), **~60 remaining as of 2026-04-03** (per latest conformance breakdown: 26 frozen mutation, 8 ref access, 7 silent, rest other; +3 new false-positive bails from validateInferredDep scope dep resolution mismatch; -6 from Stage 2g error fixture sweep; -4 from Stage 2j Infer mode body heuristic — 4 infer-mode fixtures now correctly skipped)
> Note: Conformance tests use `compilationMode:"all"` — all functions are compiled, not just detected components/hooks. This affects which bail-out validations fire.

## Summary

Of the original 108 fixtures where we bail but upstream compiles:
- **19 fixed** by removing overly aggressive file-level bail-outs (lint mode, incompatible imports, eslint suppression)
- **89 remaining** broken down by error category below

## Fixes Applied (Stage 2b, partial) -- ALL COMPLETE

Four file-level bail-outs removed or converted to per-function bails. See `index.md` Stage 2b for details.
- Fix 1: Lint mode early return removed (+2 net). File: `program.rs`.
- Fix 2: Known-incompatible import converted to per-function bail (+3 net). File: `program.rs`.
- Fix 3: Compiler runtime import check refined (+0 net). File: `program.rs`.
- Fix 4: ESLint suppression converted to per-function bail (+1 net). File: `program.rs`.

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

## Completed Fixes (condensed)

All completed fixes are summarized here. See `index.md` for full details of each stage.

- **Stage 2j: Infer mode body heuristic** -- COMPLETE (+4). `body_has_hooks_or_jsx` in `program.rs`. 3 remain (need directive support).
- **Stage 1e: Dynamic gating parsing** -- COMPLETE (+3, harness fix).
- **ObjectExpression computed key bail-out removed** -- COMPLETE (+2).
- **Empty catch handler codegen** -- COMPLETE (+1).
- **const vs let in StoreLocal codegen** -- COMPLETE (+0, correctness).
- **Gating directive comment stripping** -- COMPLETE (+2). `codegen.rs` `apply_compilation` filters `@gating`/`@dynamicGating` lines.
- **Hooks-as-value locally_declared_names** -- COMPLETE (+0, correctness guard). 3 "hooks as normal values" fixtures remain (PropertyLoad callee-name check).
- **Stage 2c: `_exp` directive handling** -- COMPLETE (+0 net, 20 fixtures moved from bail to compile pools).

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

### Stage 4d: Frozen-mutation false negatives -- COMPLETE (+10 net)

Completed. Name-based freeze tracking in `validate_no_mutation_after_freeze.rs` + destructure freeze propagation + Check 4b effect callback analysis. 1 remaining: `error.invalid-jsx-captures-context-variable.js` (JSX capture analysis). See `index.md` Stage 4d for full details.

### Stage 2e: Ref-access false positives (8 fixtures) — DEPRIORITIZED, NO CONFORMANCE IMPACT

Investigated 2026-03-25. Freed fixtures land in slots-DIFFER, net impact -2 to +0. Deprioritized until Stage 3 scope inference improvements. See `index.md` Stage 2e.

### Stage 2f: Fix reassignment false positives (10 fixtures) — BLOCKED

BLOCKED: Requires DeclareContext/StoreContext HIR lowering. Attempt caused -4 net. See `index.md` Stage 2f blocker report.

### Stage 4e-D: Todo-Bail Fixtures — PARTIALLY COMPLETE (+3)

3 of 10 fixed (for-in-try detection, file-level bail propagation). Remaining tracked in `index.md` Stage 4e-D/E.

## Silent Bail-out Detail (7 remaining, was 9)

1. `babel-existing-react-runtime-import.js` — needs smarter import merging
2. `infer-functions-component-with-ref-arg.js` — Infer mode, function with ref arg not detected as compilable
3. `unused-object-element-with-rest.js` — 0 scopes survive pipeline (scope inference gap)
4. `invalid-jsx-in-catch-in-outer-try-with-catch.js` — try-catch in HIR lowering issue
5. `invalid-jsx-in-try-with-catch.js` — try-catch in HIR lowering issue
6. `valid-set-state-in-useEffect-from-ref.js` — setState-in-effect validation fires
7. `valid-setState-in-effect-from-ref-arithmetic.js` — same

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

## Combined: Check 1 Scope Completion + tN Dep Fix

> **Status:** Part A COMPLETE (+1). Part B DEFINITIVELY BLOCKED.
> **Upstream:** `ValidatePreservedManualMemoization.ts` (Check 1 `isUnmemoized` / `completedScopes`), `PropagateScopeDependencies.ts` (dep resolution via `collectTemporaries`)
> **Files modified:** `validate_preserved_manual_memoization.rs`

### Part A: Check 1 — Scope completion tracking -- COMPLETE

**Completed (2026-04-03).** Implemented `completed_scopes: FxHashSet<ScopeId>` tracking in `WalkerState`. After visiting a `ReactiveInstruction::Scope` block, inserts `scope.id` into `completed_scopes`. `FinishMemoize` now checks whether `decl.identifier.scope` is in `completed_scopes` (per-operand scope membership) instead of the coarse `in_scope` boolean.

**Result:** +1 conformance (549->550). The gain is small because Check 2 (validateInferredDep with tN deps) fires first on most fixtures, preventing Check 1 from being the determining factor. The implementation is correct and matches upstream behavior — it just doesn't unlock gains without Part B.

**File:** `crates/oxc_react_compiler/src/validation/validate_preserved_manual_memoization.rs`

### Part B: tN dep resolution -- DEFINITIVELY BLOCKED

**Original hypothesis:** Check 1 would provide an alternative bail path for error fixtures, allowing Part B (removing tN false deps) to proceed without regression.

**What actually happened:** Check 1 does NOT provide the alternative bail path. Error fixtures still need Check 2's tN dep mismatch to bail correctly. This means ALL approaches to removing/filtering tN deps remain net-negative.

### Blocker Report — Part B: tN Dep Resolution (2026-04-03)

**4 additional approaches attempted in this session (on top of 4 prior attempts):**

| # | Approach | Location | Result | Why it failed |
|---|----------|----------|--------|---------------|
| 5 | Skip tN deps in propagate_dependencies.rs | `propagate_dependencies.rs` Phase 2 | -55 | Changes codegen slot assignments for all fixtures, not just preserve-memo |
| 6 | Skip synthetic tN names in resolve_scope_dep | `validate_preserved_manual_memoization.rs` | -55 | Removes load-bearing bails from error fixtures (same mechanism as attempt 3) |
| 7 | Remove MethodCall check entirely | `validate_preserved_manual_memoization.rs` | -4 | Freed fixtures still fail on slot mismatch; lost 4 true-positive bails |
| 8 | Receiver-only MethodCall check | `validate_preserved_manual_memoization.rs` | -4 | Catches wrong patterns — receiver is not the relevant operand |

**Key finding:** ALL 76 preserve-memo false bails are Check 2 (validateInferredDep), caused by synthetic tN-named deps. 55 of those are load-bearing (error fixtures bail "by accident" via tN mismatch). The root cause is computation-result temps (CallExpression, MethodCall, BinaryExpression, Destructure outputs) that cannot be traced to named variables through the temp map.

**Why Check 1 didn't help:** The error fixtures that bail via tN mismatch do so because their memo values DO have completed scopes (scope inference creates scopes for them). Check 1 passes for these fixtures — they are "memoized" from Check 1's perspective. They only bail because Check 2 sees tN deps that don't match manual deps. Removing the tN deps removes their bail path with no replacement.

**Assumption that was wrong:** Assumed error fixtures would fail Check 1 (scope not completed) and could be caught there instead of Check 2. In reality, our scope inference creates scopes for most memo values, so Check 1 passes even when upstream would prune/not-create those scopes.

**8 total approaches tried across all sessions, ALL net-negative.** The skip/filter strategy is fundamentally exhausted.

**Only viable path forward:** Port upstream's richer `ReactiveScopeDependency` type which includes the full property access path. This would let validateInferredDep compare deps by their source-level names (e.g., `props.x`) instead of SSA temp names (e.g., `t1`). This is a significant refactor of:
- `propagate_dependencies.rs` (dep collection and resolution)
- `validate_preserved_manual_memoization.rs` (dep comparison)
- Possibly `prune_scopes.rs` and `codegen.rs` (dep consumers)

**Do NOT attempt any further skip/filter approaches.** The only viable path is structural: porting `ReactiveScopeDependency` with full access paths.

---

## Stage 2g: Error Fixture Bail-out Sweep -- COMPLETE (+6, 499->505)

Four new bail-out validations. See `index.md` Stage 2g for full details.
- Fix 5: Duplicate fbt/fbs sub-tag detection (+2). File: `validate_no_unsupported_nodes.rs`.
- Fix 6: Ref-to-function detection (+1). File: `validate_no_ref_access_in_render.rs`.
- Fix 7: Self-referencing const declarations (+1). File: `validate_no_unsupported_nodes.rs`.
- Fix 8: Dynamic gating invalid identifier validation (+2). File: `program.rs`.
- **REJECTED: 0-slot codegen** (-52 regression). See `index.md` Deferred section.
