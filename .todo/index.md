# oxc-react-compiler Backlog

> Last updated: 2026-03-26
> Conformance: **453/1717 (26.4%)** (known-failures.txt has 1264 non-comment entries). Render: **92% (23/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Note: Conformance dropped from 453 to 441 after rebaseline; the previous 453 figure used a different counting methodology. Then +6 from Stage 1e session (441->447), then +5 from latest session (447->452): known-incompatible bail re-enabled +3, ESLint suppression bail +1, object property key quoting +1. Then +1 from freeze validation hardening (452->453): destructure freeze propagation + Check 4b effect callback analysis.
> Known-failures: 1264. Error.* fixtures remaining in KF: 33 (31 top-level + 2 fbt/).
> Note: Conformance tests use `compilationMode:"all"` which affects how fixtures are tested (all functions compiled, not just components/hooks).

---

## Road to 600+ Conformance (447 → 600+, need +153)

### Failure Category Summary (revised 2026-03-25, fresh data from deep-work session)

| Category | Count | Description |
|----------|-------|-------------|
| Both compile, slots DIFFER | 683 (53%) | Scope inference accuracy — different cache slot counts. **Largest pool, requires scope inference fixes.** Deficit (our < expected): ~400 fixtures. Surplus (our > expected): ~283 fixtures. |
| Both compile, slots MATCH | 224 (was 237, -6 from 1d Phase 1 moved to matched, -3 from latest fixes) | Same slots, codegen structure diffs. **Dominated by scope inference differences (declaration placement tied to scope inference, not just codegen). B2 pattern (temps vs original names): 40 fixtures.** |
| We compile, they don't | 194 (15%) | Missing validations. **Revised breakdown (2026-03-26): 134 have no upstream error header (not actionable without specific validation ports), 32 are preserve-memo, rest mixed.** Previously: 60 preserve-memo, 15 flow-parse, 7 todo-bail, 6 invariant, 4 frozen-value. |
| We bail, they compile | 80 (5%) | False-positive bail-outs (down from 108→89→~69 after Stage 2c, +4 IIFE false positives from Stage 4d name-based freeze tracking). Sub-breakdown: 26 frozen mutation, 8 ref access, 7 silent, rest other. |
| Both no memo (format diff) | 83 (6%) | Neither side memoizes. **Requires DCE + constant propagation passes — NOT quick wins.** |

### Key Investigation Findings (2026-03-25, updated 2026-03-26)

1. **"Both no memo" (79 fixtures) is NOT low-hanging fruit.** These require dead-code elimination and constant propagation compiler passes. Neither pass exists yet. This is significant compiler work, not cosmetic format fixes as originally assumed.

2. **"We compile, they don't" (186 fixtures) revised breakdown (2026-03-26):**
   - **134 have no upstream error header** — these fixtures have no `@expectedError` or similar marker in the upstream expected output. They are NOT actionable without identifying the specific upstream validation that rejects them.
   - **32 are preserve-memo** — `validatePreserveExistingMemoizationGuarantees` gaps (revised down from 60; previous count included fixtures that are actually in other categories).
   - **15 are flow-parse** — Flow type annotation parsing failures (not actionable without Flow support)
   - **Remaining:** mixed validation gaps (todo-bail, invariant, frozen-value, ref-access, reassignment, hooks, etc.)

3. **"Slots MATCH" (227 fixtures) is dominated by scope inference differences, not just codegen.** The B2 pattern (40 fixtures, temps vs original names) remains the largest tractable sub-pattern, but the broader pool is driven by scope inference accuracy. Stage 1d Phase 2 (declaration placement inside control flow) was found to be a scope inference issue, not a codegen issue — declarations can only move inside control flow if scope inference correctly places scope boundaries within those blocks.

4. **Stage 2c (`_exp` directive handling) is COMPLETE** — moved 20 fixtures from "we bail, they compile" to "both compile" categories. Net conformance +0 because the newly-compiling fixtures land in slots-DIFFER/MATCH pools (their output doesn't match yet). But this unblocks those 20 fixtures for future scope/codegen fixes.

5. **Stage 1d Phase 2 is a scope inference issue (2026-03-26).** Moving declarations inside control flow blocks (if/for/try) requires that scope inference itself produce scopes that are scoped to those blocks. The current scope inference merges scopes across control flow boundaries, so there is no control-flow-scoped scope to place declarations into. This is NOT a codegen-only fix — it requires scope inference improvements (Stage 3) as a prerequisite.

6. **B2 (variable name preservation) is scope-inference dependent (2026-03-26).** Many B2 fixtures (temps vs original names) also have scope boundary differences. Pure codegen name changes won't pass them without scope inference fixes. Stage 1d Phase 3 (merge decl+init) also implemented and gained +0 (dormant). The codegen-only ceiling for slots-MATCH has been reached.

7. **Re-enabling removed bail-outs as per-function bails gained +4 (2026-03-26).** Known-incompatible import bail (+3) and ESLint suppression bail (+1) were re-enabled as per-function bails matching upstream behavior. The initial full removal was too aggressive -- upstream bails per-function, not file-level.

### Revised Path to 600+

The path is clearer but requires significant compiler infrastructure work:

| Work Item | Pool Size | Potential Gain | Difficulty |
|-----------|-----------|---------------|------------|
| Scope inference fixes (slots-DIFFER) | 688 | +50-100 | HIGH — cascading regression risk, scope MERGING is bottleneck (see 3b blocker) |
| DCE + constant propagation (both-no-memo) | 79 | +30-50 | HIGH — new compiler passes needed |
| `validatePreserveExistingMemoizationGuarantees` gaps | 32 (revised from 60) | +15-25 | MEDIUM — 3 sub-types: validateInferredDep, value-memoized, dep-mutated |
| Variable name preservation in codegen (B2) | 40 | +10-20 | MEDIUM-HIGH — scope output naming changes + scope inference dependency (see lesson #29) |
| Declaration placement / instruction ordering (A1) | 55+ | +15-30 | HIGH — BLOCKED: Phase 2 requires scope inference (Stage 3). Phase 3 (merge decl+init) DONE (+0 dormant). |
| Remaining bail-out fixes (2d-2g) | ~84 total bail pool | +15-25 | MEDIUM — per-validation fixes |
| Todo error detection (remaining) | 4 | +2-4 | LOW-MED — need optional-chain-in-ternary, hoisting, context var |
| Frozen-mutation validation fixes | 1 remains | +10 done (Stage 4d + follow-up) | MEDIUM | 1 remaining needs JSX capture analysis |

**Conservative estimate:** +147-301 from 453 base = 600-754. Reaching 600 is feasible but requires scope inference work (the largest and highest-risk category).

---

### Stage 1: Codegen Structure — "Slots MATCH" Fixes (target: +40-60 fixtures)

**Pool:** 227 fixtures where slot count matches upstream but codegen differs (was 239, reduced by Stage 1b/1c/1d fixes).
**Root causes:** Variable naming (`t0` vs original names), instruction ordering within scopes, scope output placement, dependency list ordering.
**Risk:** LOW — these are the closest to passing, same semantic structure.

#### Stage 1a: Investigate "Slots MATCH" Patterns -- COMPLETE

Completed 2026-03-25. Full results in [slots-match-investigation.md](slots-match-investigation.md).
Found 4 sub-patterns: variable naming (126, 52.7%), instruction ordering (55, 23.0%), structural (58), other (44).

#### Stage 1b: Temp Variable Renumbering -- COMPLETE (+2 net, not +25-40)

Completed 2026-03-25. Implemented `renumber_temps_in_output` in `codegen.rs` (two-pass atomic rename, t0/t1/t2 sequential).
Also fixed `is_temp_place` pattern matching, Unicode safety in `replace_identifier_in_output`, and `$` word boundary.
**Gained only +2 fixtures** (403→405): `gating/multi-arrow-expr-export-gating-test.js`, `gating/multi-arrow-expr-gating-test.js`.
Estimate of +25-40 was wrong: most "naming" differences also involve instruction ordering or scope output name preservation,
which temp renumbering alone cannot fix. See [slots-match-investigation.md](slots-match-investigation.md) for revised analysis.

#### Stage 1c: Minor Codegen Fixes -- COMPLETE (+5 net, 405→410)

Completed 2026-03-25. C2 (return undefined): +5 fixtures. C5 (catch clause): +0 net (improves output but all catch fixtures also blocked by A1 instruction ordering). B4 (edge case naming): skipped — only 1 fixture and high complexity.
**Fixtures gained:** capturing-func-mutate-nested.js, capturing-function-decl.js, hoisting-recursive-call.ts, mutate-captured-arg-separately.js, reassign-object-in-context.js.

#### Stage 1d: Lazy Scope Declaration Placement (phased)

- [x] **Phase 1 (LOW risk): COMPLETE (+6, 444->450).** Moved scope declarations from function-top to just-before-scope-guard. Gained 6 fixtures (exceeded +5 estimate). Same-slots pool reduced 233->227.
- [ ] **Phase 2 (MEDIUM risk, +10-20) — BLOCKED by scope inference:** Move declarations inside control flow blocks (if/for/try). Fixes A1 pattern (39 fixtures with declaration-before-control-flow as first diff). Same-slots pool now 227. **Finding (2026-03-26):** This is actually a scope inference issue, not codegen. Declarations can only move inside control flow if scope inference produces scopes bounded by those control flow blocks. Currently scope inference merges scopes across control flow boundaries. Requires Stage 3 scope inference improvements as prerequisite.
- [x] **Phase 3 (HIGH risk, +5-10): COMPLETE (+0, dormant).** Merge declaration with initialization (`let t0; t0 = expr;` → `let t0 = expr;`). Implemented but gained +0 fixtures — all affected fixtures also differ in other ways (scope inference, naming). The merged form is emitted correctly when declaration and first assignment are adjacent, but the benefit is dormant until scope inference improvements make those fixtures matchable.
- **Details:** [slots-match-investigation.md](slots-match-investigation.md)#stage-1d-lazy-scope-declaration-placement-a1a2

#### Stage 1e: Miscellaneous Codegen/Harness Fixes -- COMPLETE (+6, 441->447)

Completed 2026-03-26. Four independent fixes:

1. **Dynamic gating parsing (+3):** Fixed a conformance test harness issue where `@gating` directive parsing failed for certain dynamic import patterns in 3 fixtures. These were false negatives in the test harness, not compiler bugs.
2. **Empty catch handler codegen (+1):** Fixed codegen to emit `catch {}` instead of `catch (e)` when the catch binding is unused, matching upstream. Combined with prior A1 ordering fix, this unblocked 1 additional fixture.
3. **ObjectExpression computed key bail-out removed (+2):** Removed an overly aggressive bail-out that rejected ObjectExpression nodes with computed keys. Upstream compiles these successfully. +2 fixtures gained.
4. **const vs let keyword in StoreLocal codegen (+0):** Fixed codegen to emit `const` instead of `let` for StoreLocal instructions where the variable is never reassigned. No conformance gain (affected fixtures also differ in other ways) but improves correctness of output.

#### Stage 1f: Follow-up Codegen/Bail Fixes -- COMPLETE (+5, 447->452)

Completed 2026-03-26. Five fixture gains from mixed fixes:

1. **Known-incompatible import bail re-enabled (+3):** Re-enabled `has_known_incompatible_import` as a per-function bail-out (was fully removed in Stage 2b initial). Upstream still bails per-function on known-incompatible imports; we need to match that behavior to pass UPSTREAM ERROR fixtures. +3 fixtures gained.
2. **Custom ESLint suppression bail added (+1):** Re-enabled ESLint suppression detection as a per-function bail. +1 fixture gained.
3. **Object property key quoting fix (+1):** Fixed codegen to properly quote object property keys that are reserved words or contain special characters, matching upstream output format. +1 fixture gained (`repro-non-identifier-object-keys.ts` removed from known-failures).
4. **Stage 1d Phase 3: Merge decl+init implemented (+0, dormant):** Implemented merging of `let t0; t0 = expr;` into `let t0 = expr;` when declaration and first assignment are adjacent. No conformance gain — all affected fixtures also differ in scope inference or naming. The improvement is dormant until scope inference fixes make those fixtures matchable.
5. **B2 investigation finding (+0):** B2 (variable name preservation, 40 fixtures) was found to be scope-inference dependent, NOT codegen-only. Many B2 fixtures have scope boundary differences that prevent passing even with correct variable names. This downgrades B2 from "largest tractable codegen fix" to "partially tractable, scope-dependent."

---

### Stage 2: False-Positive Bail-outs — "We Bail, They Compile" (target: +50-70 fixtures)

**Pool:** Originally 108 fixtures, now **89 remaining** after Stage 2b partial fixes.
**Risk:** MEDIUM — each bail-out removal must not introduce wrong output.

#### Stage 2a: Investigate Bail-out Categories -- COMPLETE

Completed 2026-03-25. Full results in [bail-out-investigation.md](bail-out-investigation.md).
Categorized all 108 bail-outs by error type. Found 4 overly aggressive file-level bail-outs (lint mode, incompatible imports, eslint suppression, runtime import check).

#### Stage 2b: Remove Overly Aggressive File-Level Bail-outs -- COMPLETE (+5 net, 410→411 initial, then +4 more in follow-up)

Completed 2026-03-25 (initial), updated 2026-03-26 (re-enabled + refined). Removed/refined 4 file-level bail-outs in `program.rs`:
- Removed `OutputMode::Lint` early return (+2 net passing, 42 fixtures now compile)
- Re-enabled `has_known_incompatible_import` as per-function bail (+3 net, was +0 when initially removed). The file-level bail was removed in Stage 2b initial, but the known-incompatible import detection was re-enabled as a per-function bail-out matching upstream behavior. +3 fixtures gained.
- Refined `has_compiler_runtime_import` to only bail on `c`/`useMemoCache` imports (+0 net)
- Re-enabled `has_eslint_suppression_for_rules` as custom ESLint suppression bail (+1 net). Originally removed as file-level bail; now properly bails per-function when custom ESLint suppression rules are present, matching upstream's per-function behavior.
Net result: bail-outs reduced 108→~84, conformance +5 total (410→411 initial, then +4 from re-enabled bails in follow-up session).
Remaining ~84 bail-outs require per-validation fixes (see stages 2c-2f below).

#### Stage 2c: Fix `_exp` Directive Handling -- COMPLETE (+0 net, 20 fixtures moved)

Completed 2026-03-25. Fixed handling of `@validateNoDerivedComputationsInEffects_exp` directive fixtures.
These 20 fixtures now compile instead of bailing, but land in slots-DIFFER/MATCH pools (output doesn't match upstream yet).
Net conformance: +0. But these fixtures are now unblocked for future scope/codegen improvements.

#### Stage 2d: Fix Frozen-Mutation False Positives (26 remaining per latest breakdown)

- [ ] Review `validate_no_mutation_after_freeze` / `InferMutableRanges` for over-reporting mutations on frozen values
- [ ] Compare our validation logic against upstream's to find divergence
- [ ] Implement targeted relaxations without losing true-positive detections
- [ ] **NEW (post Stage 4d):** Fix 4 IIFE-pattern false positives introduced by name-based freeze tracking (`capturing-func-alias-*-iife.js`). These fixtures mutate a captured variable inside an IIFE, but the name-based tracker incorrectly treats this as mutation-after-freeze. Future fix: implement scoped name tracking that distinguishes IIFE-internal mutations from true post-freeze mutations.
- **Note (2026-03-26):** Destructure freeze propagation and Check 4b effect callback analysis were completed as part of Stage 4d follow-up. These were true-positive detection improvements, not false-positive fixes. The 26 remaining false positives in the bail pool are the false-positive problem.
- **Risk:** MEDIUM — requires mutable range analysis refinements + scoped name tracking for IIFE patterns
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)

#### Stage 2e: Fix Ref-Access False Positives (8 fixtures) — LOW PRIORITY, NO CONFORMANCE IMPACT

- **Investigated (2026-03-25):** Thoroughly analyzed whether relaxing ref-access false positives would improve conformance. Result: **no gain**. Freed fixtures land in slots-DIFFER (not matched); 2 accidental Flow parse error matches would be lost. Net: -2 to +0.
- **Decision:** Deprioritized until scope inference improvements (Stage 3) make freed fixtures matchable.
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)

#### Stage 2f: Fix Reassignment False Positives (10 fixtures)

- [ ] Review `validateLocalsNotReassignedAfterRender` for false positives
- [ ] Compare against upstream validation
- **Risk:** MEDIUM
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)

#### Stage 2g: Other Bail-out Fixes (remaining ~40 fixtures)

- [ ] Fix remaining false-positive bail-outs: setState-in-render (4), setState-in-effect (7), hooks (3), preserve-memo (4), exhaustive-deps (3), silent (9), other (10)
- [ ] Each fix: compare upstream validation logic, adjust our thresholds
- [ ] Re-categorize after 2c-2f to identify new patterns

#### Stage 2h: Replan — Bail-out Residual (est: 0 fixtures, planning)

- [ ] Categorize remaining "we bail, they compile" after 2c-2g
- [ ] Update plan with new findings or mark as deferred

---

### Stage 3: Scope Inference — Slot Diffs (target: +30-50 fixtures)

**Pool:** 688 slot-differ fixtures total (402 deficit, 286 surplus). Deficit distribution: -1 (131), -2 (120), -3 (35), -4 (42), -5 (16), -6 (22), -7 (4), -8+ (32).
**Root causes:** Scope merging too aggressive/conservative, dependency over/under-counting, mutable range gaps. **Scope MERGING identified as bottleneck (not scope creation) — see 3b blocker report.**
**Risk:** HIGH — scope inference changes can cause cascading regressions.

#### Stage 3a: Investigate ±1 Slot Diff Patterns -- COMPLETE (investigation only)

Completed 2026-03-25 (Phase 142 in journal). Full categorization of 1274 diverged fixtures into 5 buckets.
Key findings (refreshed 2026-03-25: 84 we-bail, 189 we-compile-they-dont, 227 same-slots, 688 slot-differ, 79 both-no-memo):
Key findings for slot-diff (688 fixtures):
- **Primary root cause:** `last_use_map` in `infer_mutation_aliasing_ranges` over-extends mutable ranges vs upstream. Removing it breaks codegen (unresolved JSXText/intermediate refs). Fix requires: (1) receiver mutation effects for MethodCall Apply, (2) reverse scope propagation pass.
- **Secondary root cause (partially fixed):** scope inference operand check used wrong condition; fixed in Phase 92 (isMutable check, mayAllocate lvalue, phi range gating). Net: +1 fixture.
- **Same-slots-different-structure (227):** 5 recurring patterns — variable naming (PromoteUsedTemporaries missing), declaration hoisting, import line presence, temp vs destructuring, optional-chaining. No single fix > ~30 fixtures.
- **Both-no-memo (79):** Requires DCE + constant propagation (new passes). NOT cosmetic.
- **Slot-diff deficit distribution:** -1 (131), -2 (120), -3 (35), -4 (42), -5 (16), -6 (22), -7 (4), -8+ (32). Total deficit: 402. Total surplus: 286.

#### Stage 3b: Fix Dominant Slot Diff Patterns (est: +15-25 fixtures, HIGH risk)

**This is the critical-path task for reaching 600+.** The 688 slot-diff pool is the only category large enough to bridge the gap.

**Slot diff distribution (deficit = our_slots < expected_slots, 402 fixtures):**
| Diff | Count |
|------|-------|
| -1   | 131   |
| -2   | 120   |
| -3   | 35    |
| -4   | 42    |
| -5   | 16    |
| -6   | 22    |
| -7   | 4     |
| -8+  | 32    |

**Surplus (our_slots > expected_slots): 286 fixtures.**

**Known root causes from Phase 92 + Phase 142 investigations:**
1. `last_use_map` in `infer_mutation_aliasing_ranges.rs` extends ranges to last USE (not just last MUTATION). Upstream does NOT do this. Wider ranges mask scope separation bugs but prevent optimal splitting. **Cannot simply remove** — codegen depends on wide ranges for scope containment. Prerequisites: receiver mutation effects for MethodCall Apply + reverse scope propagation pass.
2. Scope inference operand `isMutable` check (partially fixed Phase 92, +1 fixture).

**Sample analysis of deficit fixtures (diverse root causes):**
- `timers.js`: Over-merged scopes — `Date.now()` + JSX lumped into one scope, should be two separate scopes
- `globals-Boolean.js`: Object creation and array creation should be separate scopes, our scope inference merges them
- `simple-alias.js`: Missing scope output — variable should be cached separately but isn't

**Approach:** Do NOT attempt to remove `last_use_map` directly. Instead:
- [ ] Sample 20 fixtures from the ±1 deficit group (we have fewer slots than upstream = under-memoization)
- [ ] For each: diff reactive scope boundaries to identify which specific scope is missing/merged
- [ ] Categorize: (a) scope should split but doesn't (under-splitting), (b) scope merged when shouldn't, (c) dependency over-counted preventing scope creation
- [ ] Identify safe, incremental fixes that don't require removing `last_use_map`
- [ ] **Regression check:** Run full conformance after each change, verify no currently-passing fixtures break
- [ ] If regressions: revert immediately, document in blocker report

**Upstream files:** `src/ReactiveScopes/InferReactiveScopeVariables.ts`, `src/Inference/InferMutationAliasingRanges.ts`, `src/ReactiveScopes/PropagateScopeDependencies.ts`
**Our files:** `infer_reactive_scope_variables.rs`, `infer_mutation_aliasing_ranges.rs`, `propagate_dependencies.rs`

##### Blocker Report — `is_allocating_instruction` heuristic removal attempt (2026-03-25)

**Approach attempted:** Removed the `last_use > instr_id` heuristic from `is_allocating_instruction` for `CallExpression`, `MethodCall`, and `TaggedTemplateExpression`. The hypothesis was that the heuristic was preventing scope creation for allocating instructions when the result was used later, causing scopes to merge when they should remain separate (deficit fixtures).

**Assumption that was wrong:** Assumed that removing the heuristic would create extra sentinel scopes that would then merge correctly with existing scope infrastructure. In reality, the extra sentinel scopes do NOT merge correctly — they create misaligned scope boundaries that break 5 previously-passing fixtures.

**What was discovered:** The problem is NOT in scope CREATION (is_allocating_instruction) but in scope MERGING. The scope merging logic consumes sentinel scopes and combines adjacent scopes, but it does so in a way that depends on the current heuristic filtering. When more sentinel scopes are created by removing the heuristic, the merger produces different (worse) results. The deficit fixtures need fixes to how scopes are merged/split, not to which instructions are flagged as allocating.

**Regression details:** -5 regressions (450 -> 445/1717), 0 new passes. Net: -5. Reverted immediately.

**Prerequisites for a successful attempt:**
- Understand the scope MERGING algorithm in detail (how sentinel scopes are consumed, what decides whether adjacent scopes merge)
- Fix scope merging to handle additional sentinel scopes without breaking existing fixtures
- Alternatively, find a different approach entirely: fix scope splitting logic rather than scope creation logic

**Useful findings to carry forward:**
- The deficit fixtures have diverse root causes (over-merging, missing outputs, wrong boundaries) — there is no single fix
- The `last_use > instr_id` heuristic in `is_allocating_instruction` is load-bearing for scope merging correctness
- Scope merging is the bottleneck, not scope sentinel creation

**Do NOT attempt again until:** The scope merging algorithm is understood in detail and a targeted fix for merging is designed. Do not naively remove heuristics from `is_allocating_instruction`.

#### Stage 3c: Fix Secondary ±1 Patterns (est: +10-15 fixtures)

- [ ] Fix remaining tractable ±1 patterns identified in 3b
- [ ] Regression check on each
- [ ] Consider: receiver mutation effects for MethodCall Apply (prerequisite for last_use_map removal)

#### Stage 3d: ±2 Slot Diff Quick Wins (est: +5-10 fixtures)

- [ ] Sample ±2 fixtures, check if any share root cause with ±1 fixes
- [ ] Fix only if low regression risk

#### Stage 3e: Replan — Slot Diff Residual (est: 0 fixtures, planning)

- [ ] Analyze remaining slot diff failures
- [ ] Determine if mutable_range switch (Phase 4d) would help or if deeper scope inference changes needed
- [ ] Update plan

---

### Stage 4: Validation Gaps — "We Compile, They Don't" (target: +50-75 fixtures)

**Pool:** 186 fixtures where upstream bails with an error but we compile (incorrectly). **Revised breakdown (2026-03-26):** 134 have no upstream error header (not directly actionable), 32 are preserve-memo, 15 flow-parse, remaining: todo-bail, invariant, frozen-value, mixed.
**Risk:** LOW — adding validations is safe (bail-out = pass-through = correct).

#### Stage 4a: Categorize Missing Validations -- COMPLETE (investigation)

Completed 2026-03-25 (extended investigation), revised 2026-03-26.

**Revised breakdown (2026-03-26):** Of the ~186 "we compile, they don't" fixtures:
- **134 have no upstream error header** — these are NOT actionable without identifying the specific upstream validation that rejects each one. This is the dominant sub-pool and is harder than previously estimated.
- **32 are preserve-memo** — `validatePreserveExistingMemoizationGuarantees` gaps (revised down from 60).
- **Remaining ~20:** todo-bail (4), frozen-value (2), flow-parse (15, not actionable), mixed.

| Sub-category | Count | Action Needed |
|-------------|-------|---------------|
| No upstream error header | 134 | NOT directly actionable — need per-fixture investigation to identify which upstream validation rejects each |
| UPSTREAM ERROR fixtures (expected output IS the error) | 29 remaining in KF (was 75, 22 fixed in Stages 4c+4e-A, others already passing) | Must bail (not transform) to pass — error message matching NOT required |
| `validatePreserveExistingMemoizationGuarantees` gaps | 32 (revised from 60) | Extend existing preserve-memo validation |
| `Todo` error detection (unimplemented features) | 4 remaining (25 done, +3 from 4e-D) | 4 need optional-chain-in-ternary (2), hoisting (1), context var detection (1) |
| Frozen-mutation detection gaps | 1 remains (10 fixed) | 10 fixed in Stage 4d + follow-up; 1 remains (JSX capture) |
| Other validation gaps (ref-access, reassignment, hooks) | ~80 | Various per-validation fixes |

#### Stage 4b: Implement `validatePreserveExistingMemoizationGuarantees` Fixes (32 preserve-memo fixtures in "we compile, they don't")

**Updated breakdown (2026-03-26, revised down from 60 to 32):** The preserve-memo fixtures in the "we compile, they don't" category. Previous count of 60 included fixtures that are actually in other categories. Revised to 32 after re-analysis. These break into 3 distinct sub-types:

| Sub-type | Count | What's needed |
|----------|-------|---------------|
| `validateInferredDep` not implemented | 26 | Port upstream's `validateInferredDep` checks — validates that inferred dependencies match manual memo deps |
| "value was memoized" check improvement | 17 | Improve detection of whether a value was actually memoized by the compiler (our check is too permissive) |
| "dependency may be mutated" tracking | 17 | Track whether dependencies of manual memos may be mutated, triggering preserve-memo bail-out |

- [ ] Audit our `validate_preserved_manual_memoization.rs` against upstream
- [ ] Port `validateInferredDep` checks (26 fixtures — largest sub-type)
- [ ] Fix "value was memoized" detection (17 fixtures)
- [ ] Add "dependency may be mutated" tracking (17 fixtures)
- [ ] **Risk:** MEDIUM — our implementation exists but has known gaps
- [ ] **Potential gain:** +30-45 fixtures (some may also need other fixes)

##### Partial Investigation Notes (2026-03-25)

An investigation was started but not completed. Key finding: our validation exists at Pass 61 but fails to detect errors because `finish_in_scope` is true -- our scope inference wraps `FinishMemoize` in reactive scopes, which causes the validation to skip checks it should be performing. Additionally, upstream has `validateInferredDep` checks that we skip entirely. These are **prerequisites** for Stage 4b to succeed:

1. Understand why `finish_in_scope` is true (scope inference wrapping `FinishMemoize` instructions)
2. Port the `validateInferredDep` checks from upstream (affects 26 fixtures)
3. May require scope inference adjustments to avoid wrapping `FinishMemoize` in reactive scopes

**Status:** Investigation incomplete. Approach is understood but implementation requires deeper scope inference analysis. The 3-sub-type breakdown provides clear attack plan once the `finish_in_scope` prerequisite is resolved.

#### Stage 4c: Add Todo Error Detection -- PARTIALLY COMPLETE (+15 net, 411->426)

Completed 2026-03-25. Implemented bail-outs for 15 of 27 Todo-error fixtures:
- Try-without-catch blocks (2 fixtures) — added in `hir/build.rs`
- Computed object keys (4 fixtures) — added in `hir/build.rs`
- Value blocks in try/catch (7 fixtures) — added in `validation/validate_no_unsupported_nodes.rs`
- Throw in try (1 fixture) — added in `validation/validate_no_unsupported_nodes.rs`
- Fbt local variables (1 fixture) — added in `validation/validate_no_unsupported_nodes.rs`

**Key finding:** The 16 fixtures originally identified as targets were already passing. The actual Todo-error fixtures were in the known-failures list (UPSTREAM ERROR set). Of 27 in that set, 15 fixed, 12 remain.

**Remaining 5 Todo-error fixtures** (require more complex handling — 7 of original 12 fixed in Stage 4e-A):
- Hoisting patterns (2) — `error.todo-functiondecl-hoisting.tsx`, `error.todo-valid-functiondecl-hoisting.tsx` — need function-level hoisting infrastructure
- Optional terminal issues (1) — `error.todo-preserve-memo-deps-mixed-optional-nonoptional-property-chain.js` — need optional chaining terminal handling
- Update expression on context vars (1) — `error.todo-handle-update-context-identifiers.js` — BLOCKED: nested HIR builders don't emit LoadContext (see blocker report in Stage 4e-A)
- For-loop context vars (1) — `error.todo-for-loop-with-context-variable-iterator.js` — need for-loop context variable handling
- **Fixed in 4e-A (moved from this list):** hoisted-function-in-unreachable-code, hoist-function-decls, hook-call-spreads-mutable-iterator, default-param-accesses-local, fbt-as-local, bug-invariant-couldnt-find-binding-for-decl, hoisting-simple-function-declaration

#### Stage 4d: Fix Frozen-Mutation False Negatives -- COMPLETE (+10 net, 426->435 initial, then +1 more in 4d follow-up)

Completed 2026-03-25 (initial), updated 2026-03-26 (follow-up). Implemented name-based freeze tracking in `validate_no_mutation_after_freeze.rs` to detect mutations on frozen values that were previously missed due to IdentifierId mismatches across scopes.
- **7 of 9 planned fixtures fixed** plus **2 bonus fixtures** (9 total gained initially)
- **Side effect:** 9 fixtures shifted from slots-MATCH/DIFFER to bail category (name-based tracking introduced false positives on IIFE patterns where a variable is captured and mutated inside an IIFE, but the freeze tracker sees the mutation as post-freeze)
- **Follow-up (2026-03-26):** Fixed `error.assign-ref-in-effect-hint.js` (+1, 452->453) via two improvements:
  - **Destructure freeze propagation:** When a frozen value (e.g., props param) is destructured, output bindings now inherit frozen status. Previously only top-level parameter names were tracked, missing destructured fields.
  - **Check 4b effect callback analysis:** Effect callbacks are now checked for prop/context mutations (previously skipped entirely). Ref mutations excluded via `is_ref_name` filter from `validate_no_ref_access_in_render.rs` (made `pub(crate)` for cross-module reuse).
- **1 planned fixture remains:**
  - `error.invalid-jsx-captures-context-variable.js` — complex JSX capture pattern, needs deeper analysis
- **New regression:** 4 IIFE-pattern fixtures (`capturing-func-alias-*-iife.js`) now falsely bail. See Stage 2d note below.

#### Stage 4e: UPSTREAM ERROR Fixture Handling (36 error.* remain in KF post 4e-D partial, was 43)

**Critical correction (2026-03-25):** The conformance test does NOT require matching exact error messages. It only checks `!compile_result.transformed` (line 781 of conformance_tests.rs). To pass an UPSTREAM ERROR fixture, we just need to bail (not transform). This is much simpler than originally described.

**Revised breakdown of 36 error.* fixtures remaining in known-failures (post Stage 4e-D partial):**

| Sub-category | Count | What we need to bail |
|-------------|-------|---------------------|
| "Compilation Skipped: preserve-memo" | 11 | `validatePreserveExistingMemoizationGuarantees` must detect and bail — overlaps Stage 4b |
| "Todo: hoisting/optional/context-var/etc" | 4 | Remaining unsupported patterns (was 7, 3 fixed in 4e-D): optional-chain-in-ternary (2), hoisting (1), context var update (1) — need deeper compiler infra |
| "Invariant: ..." (upstream internal errors) | 3 | MethodCall codegen (1), inconsistent destructuring (1), unnamed temporary (1) — 3 of original 6 fixed in 4e-A |
| "Error: This value cannot be modified" | 2 | Frozen-mutation detection — overlaps Stage 4d remaining (1 fixed: effect callback Check 4b) |
| "Error: Cannot modify locals after render" | 2 | `validateLocalsNotReassignedAfterRender` gaps |
| "Error: Cannot access refs during render" | 3 | `validateNoRefAccessInRender` gaps (1 fixed: mutate-ref-arg. Remaining: `error.invalid-pass-ref-to-function.js` needs ref-through-function-call tracking, 2 others need further investigation) |
| "Error: setState from useMemo" | 1 | setState-in-render validation gap (already partially fixed in 4cd3b20) |
| "Error: validate-*" | 3 | validate-blocklisted-imports (1), validate-object-entries/values-mutation (2) |
| Compiled output (NOT UPSTREAM ERROR) | 5 | Slots-DIFFER/MATCH issues, not bail-out issues |

**Tractable sub-tasks (no new infrastructure needed):**

- [x] **4e-A: Mixed bail-outs — COMPLETE (+7, 435->442)** — implemented 7 new bail-outs across 3 files: hoisted function decls in unreachable code (3 fixtures: `error.todo-hoist-function-decls.js`, `error.todo-hoisted-function-in-unreachable-code.js`, `error.hoisting-simple-function-declaration.js`), fbt parameter name detection (1: `fbt/error.todo-fbt-as-local.js`), default-param arrow/function expressions (1: `error.default-param-accesses-local.js`), catch clause destructuring (1: `error.bug-invariant-couldnt-find-binding-for-decl.js`), hook spread arguments (1: `error.todo-hook-call-spreads-mutable-iterator.js`). Files: `validate_no_unsupported_nodes.rs`, `build.rs`, `known-failures.txt`. **Note:** `error.todo-handle-update-context-identifiers.js` (Group 6, UpdateExpression on context vars) was NOT fixed — nested HIR builders don't emit `LoadContext` instructions, so context variables can't be detected by walking the nested HIR. See blocker report below.
- [~] **4e-B: Locals-reassigned + ref-access + setState bail-outs (5 fixtures)** — tighten existing validators (`validate_no_ref_access_in_render`, `validate_locals_not_reassigned_after_render`, setState checks, hooks-in-loop) to catch these specific patterns. **Progress:** +2 fixtures (hooks-in-for-loop via Terminal::Branch handling in `validate_hooks_usage.rs`; ref-access detection for `error.validate-mutate-ref-arg-in-render.js` via name-based + Type::Ref fallback in `validate_no_ref_access_in_render.rs`). Remaining potential gain: +3.
- [~] **4e-C: Frozen-mutation remaining (2 fixtures, was 3)** — overlaps Stage 4d remaining. 1 fixed (effect callback Check 4b, 2026-03-26). Remaining: `error.invalid-jsx-captures-context-variable.js` (JSX capture analysis) + 1 other. Potential gain: +2.
- [~] **4e-D: Todo-bail fixtures (10 fixtures) — PARTIALLY COMPLETE (+3, 450->453).** Fixed 3 of 10: `repro-declaration-for-all-identifiers.js` (for-in-try detection via Terminal::For), `repro-for-loop-in-try.js` (same), `repro-nested-try-catch-in-usememo.js` (file-level bail propagation via ANY_FUNCTION_BAILED thread-local). **7 remaining:** `optional-call-chain-in-ternary.ts`, `todo-optional-call-chain-in-optional.ts`, `propagate-scope-deps-hir-fork/todo-optional-call-chain-in-optional.ts`, `error.dont-hoist-inline-reference.js`, and ~3 others. See new gap notes below.
- [ ] **4e-D2: Preserve-memo gaps (11 fixtures)** — overlaps Stage 4b. BLOCKED by `finish_in_scope` issue (see Stage 4b notes). Potential gain: +11 but requires scope inference fix.
- [ ] **4e-E: Todo remaining (4 fixtures, was 7, 3 fixed in 4e-D)** — overlaps Stage 4c remaining. Need optional-chain-in-ternary (2), hoisting (1), context var update (1). Potential gain: +4 but requires new infrastructure. Context var update BLOCKED by nested HIR LoadContext gap. Optional-chain-in-ternary needs new validation pattern (see gap note below).

**Stage 4e-A done: +7 fixtures gained.**
**Stage 4e-B progress: +2 fixtures gained (444 total, pre-1d).**
**Stage 4e-D partial: +3 fixtures gained (450->453).** Fixed via Terminal::For detection + file-level bail propagation.
1. Fixed hooks-in-for-loop detection: `find_conditional_blocks` in `validate_hooks_usage.rs` now handles `Terminal::Branch` (for-loop continue/break targets), which was previously unmatched, causing the validator to miss hook calls inside for-loops.
2. Fixed ref-access detection for `error.validate-mutate-ref-arg-in-render.js`: `validate_no_ref_access_in_render.rs` now uses name-based fallback (`is_ref_name` / `ref_names` set) and `Type::Ref` checks on PropertyLoad/PropertyStore objects, in addition to ID-based tracking. This handles cases where inline_load_local_temps (Pass 9.6) eliminates LoadLocal instructions, causing ref IDs to not propagate. Also tracks source place IDs in LoadLocal (not just lvalue IDs).
**Remaining tractable gain (4e-B): +3 fixtures, no new infrastructure.**
**Remaining ref-access fixtures:** `error.invalid-pass-ref-to-function.js` needs ref-through-function-call tracking (detecting when a ref is passed as argument to a function that accesses `.current`). The other ref-access false-positive bail-outs (Stage 2e, 8 fixtures) are a separate category where we incorrectly bail on valid code.
**Full potential (all remaining sub-tasks): +26 fixtures.**
**Risk:** LOW for 4e-B. MEDIUM-HIGH for 4e-C/D/E.

#### Blocker Report — Nested HIR LoadContext gap (2026-03-25)

**Affects:** `error.todo-handle-update-context-identifiers.js` (Stage 4e-E, 1 fixture)

**Approach attempted:** Walk the nested function HIR looking for `PostfixUpdate`/`PrefixUpdate` instructions whose operand is a "context variable" (captured from outer scope). Tried collecting local declaration IDs and treating any non-local variable as a context variable.

**Assumption that was wrong:** Expected nested HIR to contain `LoadContext` instructions for captured variables (as upstream does). In reality, our nested `HIRBuilder` emits `LoadLocal` for all variables, whether local or captured from outer scope.

**What was discovered:** The nested HIR builder creates a fresh scope and fresh `IdentifierId` numbering for each nested function. Variables captured from the parent are lowered as `LoadLocal` with a new ID, not `LoadContext`. The distinction between "local" and "context" is only resolved later in the pipeline (during scope inference / codegen), not at HIR construction time. This means a validation pass running on the raw nested HIR cannot distinguish context variables from locals.

**Prerequisites for a successful attempt:**

- Either emit `LoadContext` in nested HIR builders (requires threading parent scope bindings into child builder), OR
- Run this validation after scope inference when context variables are identified, OR
- Pass parent scope binding names to the validation and compare by name (fragile but possible)

**Do NOT attempt again until:** The HIR builder's nested function lowering is enhanced to distinguish context variables, or a post-scope-inference validation hook exists.

#### Gap: Optional Chain in Ternary Detection (2 fixtures)

**Fixtures:** `optional-call-chain-in-ternary.ts`, `todo-optional-call-chain-in-optional.ts` (also `propagate-scope-deps-hir-fork/todo-optional-call-chain-in-optional.ts`)

**What upstream does:** Upstream emits a `Todo` error when it encounters optional chaining (`?.()` or `?.`) inside ternary expressions within try blocks (or similar patterns). This is an unimplemented feature that upstream explicitly bails on.

**Current state:** Our validation does not check for optional chaining inside ternary expressions. We silently compile these fixtures instead of bailing. The existing `validate_no_unsupported_nodes` checks for some Todo patterns (try-without-catch, computed keys, etc.) but does not cover this pattern.

**What's needed:** Add detection for optional call chains (`?.()`) and optional member expressions (`?.`) when they appear inside conditional/ternary expressions within try blocks. This likely requires walking expression nodes during HIR lowering or validation to detect the specific pattern.

**Depends on:** None (standalone validation addition), but the detection pattern is non-trivial because the optional chaining may be nested arbitrarily deep inside the ternary operands.

#### Gap: Hoisting Inline Reference Detection (1 fixture)

**Fixture:** `error.dont-hoist-inline-reference.js`

**Current state:** Not investigated. Upstream bails with an error related to hoisting inline references. We compile instead. The specific upstream validation that catches this pattern has not been identified.

**What's needed:** Investigation to determine which upstream validation catches this pattern and what our gap is.

**Depends on:** Investigation needed before implementation.

#### Cross-Cutting Fix: File-Level Bail Propagation (ANY_FUNCTION_BAILED)

**Implemented in 4e-D (2026-03-26).** When ANY function in a file bails during compilation (e.g., due to a try-catch Todo error), the bail-out now propagates to the file level via an `ANY_FUNCTION_BAILED` thread-local flag. This ensures that fixtures where upstream bails the entire file (because one function has an unsupported pattern) are correctly handled by our compiler. Previously, we would bail on the individual function but still emit a transformed result for the file, causing the conformance test to see `transformed = true` when it should be `false`.

**Impact:** This is a cross-cutting improvement that affects ALL fixtures in the "we compile, upstream bails" category where the upstream bail comes from a nested function within the file. The 3 fixtures gained in 4e-D (+3, 450->453) were directly enabled by this propagation mechanism.

---

### Stage 5: "Both No Memo" — DCE + Constant Propagation (target: +30-50 fixtures)

**Pool:** 79 fixtures where neither side memoizes but output differs.
**Risk:** HIGH — requires implementing new compiler passes (DCE, constant propagation).

**Investigation finding (2026-03-25):** These are NOT cosmetic format diffs as originally assumed. The 76 fixtures produce different output because upstream runs dead-code elimination and constant propagation passes that simplify the output. Without these passes, our output includes dead assignments, unreachable branches, and un-folded constants that upstream eliminates.

#### Stage 5a: Dead Code Elimination Pass

- [ ] Implement DCE pass to remove unused variable assignments, unreachable branches
- [ ] Upstream reference: `src/Optimization/DeadCodeElimination.ts` (or equivalent)
- [ ] Must run after scope inference, before codegen
- [ ] **Risk:** HIGH — new pass, requires thorough testing
- [ ] **Prerequisite for:** Many of the 76 "both no memo" fixtures

#### Stage 5b: Constant Propagation / Folding

- [ ] Implement constant propagation to fold known-constant expressions
- [ ] Upstream reference: `src/Optimization/ConstantPropagation.ts` (or equivalent)
- [ ] **Risk:** HIGH — new pass

#### Stage 5c: Large Slot Diff Triage (est: +5-10 fixtures)

- [ ] Sample ±3+ slot diff fixtures for patterns sharing root cause with Stage 3 fixes
- [ ] Cherry-pick easy wins only

---

### Milestone Summary (revised post-investigation)

| Stage | Target | Cumulative (from 411) | Risk | Notes |
|-------|--------|-----------------------|------|-------|
| Stage 1b: Temp renumbering | +2 (done) | 405 | LOW | Completed. |
| Stage 1c: Minor codegen fixes | +5 (done) | 410 | LOW | Completed. |
| Stage 2a: Bail-out investigation | +0 (done) | 410 | -- | Completed. |
| Stage 2b: File-level bail-outs | +1 (done) | 411 | LOW | Completed. 108→89 bail-outs. |
| Stage 2c: `_exp` directive handling | +0 (done) | 411 | LOW | Completed. 20 fixtures moved to compile pools. |
| Stage 2d: Frozen-mutation false positives | +5-8 | 416-419 | MEDIUM | 11 fixtures |
| Stage 2e: Ref-access false positives | +0 (no impact) | -- | LOW | 8 fixtures, freed land in slots-DIFFER. Deprioritized. |
| Stage 2f: Reassignment false positives | +5-7 | 424-431 | MEDIUM | 10 fixtures |
| Stage 2g: Other bail-outs | +5-10 | 429-441 | MIXED | ~40 remaining fixtures |
| Stage 1d Phase 1: Declaration placement | +6 (done) | 450 | LOW | Completed. Phase 2/3 remain (+10-30). |
| B2: Variable name preservation | +20-30 | 464-501 | MEDIUM | 40 fixtures, scope output naming. **Finding (2026-03-26): scope-inference dependent, NOT codegen-only.** Many B2 fixtures also have scope boundary differences; pure codegen name changes won't pass them. |
| Stage 3: Scope inference (±1/±2 diffs) | +50-100 | 514-601 | HIGH | 688 pool (402 deficit, 286 surplus), scope MERGING is bottleneck (see 3b blocker) |
| Stage 4b: Preserve-memo validation | +15-25 | 544-646 | MEDIUM | 32 fixtures (revised down from 60; 3 sub-types: validateInferredDep, value-memoized, dep-mutated) |
| Stage 4c: Todo error detection | +15 (done, 5 remain) | 426 | LOW | 22/27 done (15 in 4c + 7 in 4e-A). Remaining 5 need hoisting, optional terminals, context vars. |
| Stage 4d: Frozen-mutation false negatives | +10 (done) | 435+1 | MEDIUM | Completed. 7/9 planned + 2 bonus + 1 follow-up (Check 4b). 1 remains (JSX capture). |
| Stage 4e-A: Upstream error bail-outs | +7 (done) | 442 | LOW | 7/43 done. 4e-B through 4e-E remain. |
| Stage 4e-B: Locals/ref/setState/hooks | +2 so far | 444 (pre-1d) | LOW | 2/5 done (hooks-in-loop, mutate-ref-arg). 3 remain. |
| Stage 1e: Misc codegen/harness | +6 (done) | 447 (from 441) | LOW | Completed. Gating parsing +3, empty catch +1, computed key +2, const/let +0. |
| Stage 4e-D: Todo-bail (partial) | +3 (done) | 453 | LOW | 3/10 done (for-in-try, bail propagation). 7 remain (optional-chain, hoisting). |
| Stage 4e-C/D2/E: Remaining upstream errors | +18-35 | 471-488 | MED-HIGH | 4e-C (3, MED), 4e-D2 preserve-memo (11, MED-HIGH), 4e-E (7, HIGH) |
| Stage 5: DCE + constant propagation | +30-50 | 604-754 | HIGH | 79 fixtures, new passes needed |
| **Total remaining** | **+147-301** | **600-754** | | From 453 base |

**Key learning from Stage 1b:** Temp renumbering alone is nearly worthless (+2). Naming and ordering are entangled — fixing one without the other does not pass conformance.

**Key learning from Stage 2a/2b:** Most bail-outs come from specific validations, not silent/0-scope issues. File-level bail-outs were low-hanging fruit (+1 net from removing 4).

**Key correction (2026-03-26):** The 88 error.* figure was pre-Stage-4c/4d. After Stage 4e-D partial + freeze follow-up, **33 error.* fixtures remain in known-failures** (31 top-level + 2 fbt/). Down from 43 pre-4e-A, 37 pre-4e-D, 34 pre-freeze-follow-up.

**Important: CompilationMode::All in conformance tests.** The conformance test harness (`tests/conformance_tests.rs`) uses `compilationMode:"all"`, meaning ALL functions in a fixture are compiled (not just those detected as components/hooks). This affects which fixtures pass/fail because validations run on every function body, not just component-shaped ones. When investigating fixture behavior, always account for this mode.

**Key learning from Stage 2c:** Fixing bail-outs does not directly increase conformance if the newly-compiling fixtures land in slots-DIFFER/MATCH pools. Bail-out fixes unblock fixtures for FUTURE scope/codegen improvements but yield +0 net on their own.

**Key learning from Stage 1d Phase 1:** Lazy declaration placement gained +6 (exceeded +5 estimate). Confirms that declaration ordering is a tractable codegen fix. Phases 2-3 (control-flow-scoped declarations, merged init) remain and target the larger A1 pool (39 fixtures).

**Key learning from Stage 2e investigation:** Not all bail-out fixes improve conformance. Ref-access false positives free 8 fixtures that land in slots-DIFFER, not matched. Additionally, 2 accidental Flow parse error matches would be lost. Always check where freed fixtures land before pursuing bail-out removal.

**Key learning from extended investigation (2026-03-25):**
- "Both no memo" is NOT format diffs — requires DCE + constant propagation (new compiler passes)
- "We compile, they don't" has 75 UPSTREAM ERROR fixtures — significant untapped pool if we match error formats
- Slots-MATCH B2 pattern (40 fixtures) is the single largest tractable codegen fix remaining
- `validatePreserveExistingMemoizationGuarantees` gaps account for 32 of the "we compile, they don't" fixtures

**Revised path to 600 (updated 2026-03-26):** Reachable via scope inference fixes (Stage 3, +50-100) + validation gaps (Stage 4, +33-76 remaining) + codegen fixes (B2, +10-20; 1d Phase 3 done +0 dormant). Note: 1d Phase 2 is now BLOCKED by scope inference (see finding #25). B2 also found to be scope-inference dependent (see finding #29). DCE/constant propagation (Stage 5) could push well past 600 but is the hardest work. Conservative floor: ~600 from 453 base. Optimistic: 700+.

**Key learning from Stage 3b investigation (2026-03-25):** The slot-diff deficit (402 fixtures) has diverse root causes (over-merging, missing outputs, wrong boundaries). Naively removing heuristics from `is_allocating_instruction` causes regressions (-5) because the problem is in scope MERGING, not scope CREATION. The `last_use > instr_id` heuristic is load-bearing for scope merging correctness. Future scope inference work must target the merging algorithm, not sentinel creation.

**Key learning from "we compile, they don't" re-analysis (2026-03-25):** The 189 fixtures break down as 60 preserve-memo (largest actionable sub-pool), 15 flow-parse (not actionable), 10 todo-bail, 6 invariant, 4 frozen-value. The 60 preserve-memo further split into 3 sub-types, making Stage 4b more tractable than previously thought (clear attack plan per sub-type).

**Key principle:** Each stage starts with investigation (sub-task "a") that produces a fixture-level breakdown. If the investigation shows estimates are wrong, the plan is updated before implementation begins. No blind implementation.

---

## Deferred / Blocked Work

### Phase 2 Remaining: Impure Function Handling — DEFERRED

**Files:** `src/inference/infer_mutation_aliasing_effects.rs`, `src/validation/`
- Impure function handling in legacy signatures — requires `validate_no_impure_functions_in_render` integration

### Phase 4c: Remove `validate_no_mutation_after_freeze.rs` — BLOCKED

**Files:** `src/validation/validate_no_mutation_after_freeze.rs`
- Cannot remove yet — standalone validator has hook-call-freezes-captures logic not in effects pass
- Phase 119 gap: LoadGlobal hook names not resolved in id_to_name

### Phase 4d: Switch to `mutable_range` — NOT READY

**Files:** `src/reactive_scopes/infer_reactive_scope_variables.rs`
- 6 failed attempts. Over-splitting regressions. `use_mutable_range` flag preserved for A/B testing.
- May be revisited after Stage 3 scope inference improvements

### Phase 5: Fault Tolerance — BLOCKED until 600+

- PanicThreshold change to CriticalErrors requires ~600+ conformance
- 132 bail-out fixtures produce wrong output when compiled instead of bailed

### Performance: O(n^2+) Scaling — DEFERRED

- Effects/aliasing passes have O(n^2+) scaling
- Deferred until correctness work stabilizes

---

## Critical Architecture Notes

**Read these before making ANY changes.**

### `effective_range` vs `mutable_range` — STILL NEEDED
File: `src/reactive_scopes/infer_reactive_scope_variables.rs`
Uses `effective_range = max(mutable_range.end, last_use + 1)`. 6 failed switch attempts. The `use_mutable_range` flag on EnvironmentConfig is preserved for A/B testing.

### `collect_all_scope_declarations` is load-bearing
File: `src/reactive_scopes/codegen.rs`
Pre-declares ALL scope output variables at function level. Removing it causes render to drop 96%->24%.

### Block iteration order != source order for loops
HIR blocks stored in creation order; for-loop constructs create blocks out of source order.

### Cross-scope `IdentifierId` mismatch
Nested function bodies have their own `IdentifierId` numbering. Name-based resolution needed.

### Build & test
```bash
cargo test                                            # All Rust tests
cargo test --test conformance_tests -- --nocapture    # Conformance (1717 fixtures)
cargo insta test --accept                             # Update snapshots
cd napi/react-compiler && npx @napi-rs/cli build --release  # Rebuild NAPI
cd benchmarks && npm run render:compare               # Render comparison
cd benchmarks && npm run e2e:quick                    # E2E Vite builds
```

---

## Key File Reference

| Purpose | Path |
|---------|------|
| Pipeline orchestration | `src/entrypoint/pipeline.rs` |
| HIR types | `src/hir/types.rs` |
| HIR builder (AST->HIR) | `src/hir/build.rs` |
| Code generation | `src/reactive_scopes/codegen.rs` |
| Aliasing effects | `src/inference/aliasing_effects.rs` |
| Mutation effects | `src/inference/infer_mutation_aliasing_effects.rs` |
| Mutation ranges | `src/inference/infer_mutation_aliasing_ranges.rs` |
| Function analysis | `src/inference/analyse_functions.rs` |
| Scope grouping | `src/reactive_scopes/infer_reactive_scope_variables.rs` |
| Scope dependencies | `src/reactive_scopes/propagate_dependencies.rs` |
| Scope pruning + rename | `src/reactive_scopes/prune_scopes.rs` |
| Frozen mutation validation | `src/validation/validate_no_mutation_after_freeze.rs` |
| Ref access validation | `src/validation/validate_no_ref_access_in_render.rs` |
| Global reassignment validation | `src/validation/validate_no_global_reassignment.rs` |
| Hooks usage validation | `src/validation/validate_hooks_usage.rs` |
| Shared function context | `src/validation/function_context.rs` |
| Memoization validation | `src/validation/validate_preserved_manual_memoization.rs` |
| Conformance runner | `tests/conformance_tests.rs` |
| Known failures | `tests/conformance/known-failures.txt` |

All paths relative to `crates/oxc_react_compiler/`.

---

## Upstream References

| Resource | URL |
|----------|-----|
| New aliasing model PR | `facebook/react#33494` |
| New model documentation | `compiler/.../Inference/MUTABILITY_ALIASING_MODEL.md` |
| Old code removal | `facebook/react#34028`, `#34029` |
| Fault tolerance | `facebook/react#35872` through `#35888` |
| Exhaustive deps validation | `facebook/react#34394` |
| Feature flag cleanup | `facebook/react#35825` |
| Fallback pipeline removed | `facebook/react#35827` |
| React Compiler v1.0 blog | `react.dev/blog/2025/10/07/react-compiler-1` |

---

## Lessons Learned

1. **effective_range is load-bearing.** 6 attempts to switch to mutable_range have shown regressions.
2. **collect_all_scope_declarations cannot be removed.** It prevents render collapse from 96% to 24%.
3. **PanicThreshold change to CriticalErrors requires ~600+ conformance.** 132 bail-out fixtures produce wrong output when compiled instead of bailed.
4. **Emitting 0-slot functions requires more error validations.** 68 divergences when attempted (Phase 121).
5. **Render regressions can be latent.** The PostfixUpdate/PrefixUpdate codegen bug existed for months.
6. **Fix low-risk bail-outs before high-risk scope inference.** Easy wins first.
7. **Preserve-memo validation needs `ManualMemoDependency` for full upstream fidelity.**
8. **Performance regression from Phases 113-130.** O(n^2+) scaling in effects/aliasing passes. Deferred.
9. **Expected file generation must use `compilationMode: "all"`.** Fixed in Phase 133.
10. **Each stage must start with investigation.** Blind implementation wastes effort. Investigate → plan → implement → verify → replan.
11. **Naming diffs co-occur with ordering diffs.** Temp renumbering alone gained only +2 (expected +25-40). Most "variable naming" fixtures also differ in instruction ordering or declaration placement. Fixing names without fixing ordering does not pass conformance.
12. **File-level bail-outs are wasteful.** 4 overly aggressive file-level bail-outs (lint mode, incompatible imports, eslint suppression, runtime import) were blocking 19 fixtures unnecessarily. Upstream handles these per-function. Always prefer per-function bail-outs over file-level.
13. **`validateNoDerivedComputationsInEffects` is the largest single bail-out source.** 20 of 89 remaining false-positive bail-outs come from this one validation. Fixing it is the highest-ROI next step for bail-out reduction.
14. **Fixing bail-outs yields +0 net conformance if output still differs.** Stage 2c moved 20 fixtures from bail to compile, but all landed in slots-DIFFER/MATCH pools. Bail-out fixes UNBLOCK fixtures for future scope/codegen fixes but do not directly increase conformance. Plan accordingly.
15. **"Both no memo" requires DCE + constant propagation.** Originally assumed to be cosmetic format diffs. Actually requires dead-code elimination and constant folding passes that don't exist yet. 76 fixtures affected.
16. **"We compile, they don't" has a large UPSTREAM ERROR sub-pool.** 75 of ~225 fixtures have error messages as expected output. These are a significant conformance opportunity if we match upstream error formats precisely.
17. **B2 (variable name preservation) is the dominant tractable slots-MATCH pattern.** 40 fixtures where we use temps but upstream preserves original names in scope outputs. Single largest fixable sub-pattern in the 237-fixture slots-MATCH pool.
18. **Todo error fixtures live in known-failures, not in the "not in KF" set.** The 16 fixtures initially identified as Todo-error targets were already passing. The actual Todo-error fixtures were in the UPSTREAM ERROR subset of known-failures. Always check known-failures.txt for UPSTREAM ERROR fixtures when looking for validation gaps.
19. **Low-hanging Todo errors yield good ROI.** Stage 4c gained +15 from simple pattern detection (try-without-catch, computed keys, value-blocks-in-try, throw-in-try, fbt locals). The remaining 12 need deeper compiler infrastructure (hoisting, optional terminals, default params).
20. **UPSTREAM ERROR conformance only checks `!transformed`, not error message content.** The conformance test (line 775-781 of conformance_tests.rs) passes an UPSTREAM ERROR fixture if `compile_result.transformed` is false. We do NOT need to match the exact upstream error string. This makes the task much simpler: just bail, don't need error format matching.
21. **Name-based freeze tracking trades false negatives for false positives.** Stage 4d gained +9 by tracking frozen identifiers by name (solving cross-scope IdentifierId mismatch), but introduced 4 IIFE-pattern false positives where captured variables are mutated inside IIFEs. Net gain is still positive (+9 gained, -4 shifted to bail = +5 net new passing + 4 category shifts). Future improvement: scoped name tracking that understands IIFE boundaries.
22. **File-level bail propagation is necessary for conformance.** When a nested function bails (e.g., try-catch Todo error), upstream treats the entire file as "not transformed." Our compiler was bailing the individual function but still reporting the file as transformed. The `ANY_FUNCTION_BAILED` thread-local flag (added in 4e-D) propagates any function-level bail to the file level, matching upstream behavior. This cross-cutting fix enabled 3 fixtures and may enable more as we add new per-function bail-outs.
23. **Optional chaining in ternary/conditional expressions is a distinct Todo pattern.** Upstream bails on `?.()` and `?.` inside ternary expressions in try blocks. This is not covered by our existing Todo-pattern detection. The 2 remaining optional-chain fixtures (`optional-call-chain-in-ternary.ts`, `todo-optional-call-chain-in-optional.ts`) need a new detection path.
24. **Stage 1d Phase 2 (declaration placement) is a scope inference problem, not codegen.** Moving declarations inside control flow blocks (if/for/try) requires scope inference to produce scopes bounded by those blocks. Currently scope inference merges scopes across control flow boundaries. Phase 2 is BLOCKED until Stage 3 scope inference improvements.
25. **Slots-MATCH pool is dominated by scope inference differences.** While B2 (variable name preservation, 40 fixtures) is the largest single tractable pattern, the broader 227-fixture pool cannot be substantially reduced by codegen changes alone. Most differences trace back to scope inference producing differently-shaped scopes than upstream.
26. **"We compile, they don't" revised breakdown (2026-03-26).** 134 of 186 fixtures have no upstream error header — they are not actionable without identifying the specific upstream validation that rejects each one. Only 32 are preserve-memo (revised down from 60). The 134-fixture "no header" pool makes this category harder than previously estimated.
27. **Dynamic gating parsing was a test harness bug, not a compiler bug.** 3 fixtures gained by fixing conformance test directive parsing for `@gating` patterns. Always check whether a fixture failure is a harness issue before assuming it's a compiler bug.
28. **Nested HIR builders don't emit LoadContext instructions.** When a nested function is lowered by a child `HIRBuilder`, context variables (captured from outer scope) are represented as plain `LoadLocal` in the nested HIR, not `LoadContext`. This means walking the nested HIR cannot distinguish context variables from local variables. The upstream compiler uses `LoadContext` to identify captured variables in nested lambdas. Fixing `error.todo-handle-update-context-identifiers.js` requires either (a) emitting `LoadContext` in nested builders, or (b) passing parent scope binding information to the validation pass. This is a structural limitation, not a simple pattern-matching fix.
29. **B2 (variable name preservation) is scope-inference dependent, NOT codegen-only.** Investigation of the 40 B2 fixtures revealed that many also have scope boundary differences driven by scope inference. Changing which variable name is used for scope outputs (temp vs original) is a codegen change, but it does not pass conformance if the scope itself has different boundaries than upstream. B2 is therefore only partially addressable by codegen; the remainder requires scope inference improvements (Stage 3). This downgrades B2 from "largest tractable codegen fix" to "partially tractable, scope-dependent."
30. **Re-enabling removed bail-outs as per-function bails can gain fixtures.** The known-incompatible import bail and ESLint suppression bail were removed in Stage 2b as file-level bails. Re-enabling them as per-function bails (matching upstream behavior) gained +4 fixtures (+3 from incompatible imports, +1 from ESLint suppression). The lesson: removing a bail-out entirely is wrong if upstream still bails per-function. The fix is to change the granularity (file-level -> per-function), not remove the bail entirely.
31. **Object property key quoting matters for conformance.** Codegen must quote object property keys that are reserved words or contain special characters to match upstream output. A single property key formatting difference causes a fixture to fail even if the semantics are identical.
