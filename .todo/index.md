# oxc-react-compiler Backlog

> Last updated: 2026-04-03
> Conformance: **540/1717 (31.5%)** (known-failures.txt has 1177 non-comment entries). Render: **92% (23/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Note: +44 this session: preserve-memo default (+30, 496->526), try-catch value block bail (+10, 526->536), object expression computed key bail (+4, 536->540).
> Known-failures: 1177. False-positive bails: ~174 (83 preserve-memo, 14 frozen-mutation, 9 reassign, 9 silent, 7 ref-access, 7 context-variable, 7 setState-in-effect, 5 MethodCall codegen, rest misc).
> WE-COMPILE-THEY-DON'T: ~94 (69 scope-surplus with no upstream error, 15 "Found 1 error" bail-outs remaining, 10 Flow parse errors). Down from 108 after Stage 4f Groups A+C.
> Note: Conformance tests use `compilationMode:"all"` which affects how fixtures are tested (all functions compiled, not just components/hooks).

---

## Road to 600+ Conformance (540 → 600+, need +60)

### Failure Category Summary (revised 2026-04-03, post-session: preserve-memo +30, try-catch bail +10, computed key bail +4)

| Category | Count | Description |
|----------|-------|-------------|
| Both compile, slots DIFFER | ~579 (49%) | Scope inference accuracy — different cache slot counts. Deficit (our < expected): -1 (143), -2 (110), -3+ (183). Surplus (our > expected): +1 (74), +2 (41), +3+ (28). **Largest pool, requires scope inference fixes. BLOCKED by Stage 3b.** |
| Both compile, slots MATCH | ~240 (20%) | Same slots, codegen structure diffs. **Dominated by variable naming/scope inference. Codegen-only ceiling reached.** |
| We bail, they compile | ~174 (15%) | False-positive bail-outs. 83 preserve-memo, 14 frozen-mutation, 9 reassign, 9 silent, 7 ref-access, 7 context-variable, 7 setState-in-effect, 5 MethodCall codegen, rest misc. |
| We compile, they don't | ~94 (8%) | 69 scope-inference surplus (no upstream error, 0-slot fixtures), **15 "Found 1 error" remaining** (was 29, -10 try-catch bail, -4 computed key bail), 10 Flow parse errors. **15 remaining -- see Stage 4f.** |
| Both no memo (format diff) | ~90 (8%) | Neither side memoizes. **Blocked by 0-slot codegen (attempted twice, -50+ regression).** |

### Key Investigation Findings (2026-03-25, updated 2026-03-26)

1. **"Both no memo" (~85 remaining) — DCE/CP/branch-elimination ceiling reached.** DCE + constant propagation + dead branch elimination passes all implemented (Stages 5a+5b), gained +7 fixtures (457->464). Dead branch elimination (Stage 5b) gained +0 because branch conditions are rarely constant at Pass 32.5. The remaining ~85 fixtures are **blocked by 0-slot codegen** — our compiler wraps functions in `_c(0)` memoization structure even when upstream emits them as passthrough with no memoization. This is a scope inference issue (we create scopes where upstream doesn't), NOT a DCE/CP gap. Further DCE/CP work (binary folding, string concat) has diminishing returns on this pool.

2. **"We compile, they don't" (~160 fixtures, revised from 191 after preserve-memo +31) CRITICAL CORRECTION (2026-03-26):**
   - **134 are SCOPE INFERENCE SURPLUS, NOT validation gaps.** These fixtures have expected output with 0 `_c()` calls and NO `// UPSTREAM ERROR:` header. Upstream DID compile them (structurally transformed code) but produced 0 reactive scopes. We produce >0 scopes (over-memoization). These OVERLAP with the 286 surplus fixtures in slots-DIFFER. Understanding WHY upstream produces 0 scopes on these is the key to a large conformance gain.
   - **~12 are UPSTREAM ERROR** (in known-failures) — must bail to pass.
   - **~11 are preserve-memo** — `validatePreserveExistingMemoizationGuarantees` remaining gaps (value-memoized + dep-mutated sub-types). The validateInferredDep sub-type is now LARGELY COMPLETE (+31 fixtures, see Stage 4b).
   - **~3 remaining:** mixed (flow-parse not actionable, other validation gaps).

3. **"Slots MATCH" (227 fixtures) is dominated by scope inference differences, not just codegen.** The B2 pattern (40 fixtures, temps vs original names) remains the largest tractable sub-pattern, but the broader pool is driven by scope inference accuracy. Stage 1d Phase 2 (declaration placement inside control flow) was found to be a scope inference issue, not a codegen issue — declarations can only move inside control flow if scope inference correctly places scope boundaries within those blocks.

4. **Stage 2c (`_exp` directive handling) is COMPLETE** — moved 20 fixtures from "we bail, they compile" to "both compile" categories. Net conformance +0 because the newly-compiling fixtures land in slots-DIFFER/MATCH pools (their output doesn't match yet). But this unblocks those 20 fixtures for future scope/codegen fixes.

5. **Stage 1d Phase 2 is a scope inference issue (2026-03-26).** Moving declarations inside control flow blocks (if/for/try) requires that scope inference itself produce scopes that are scoped to those blocks. The current scope inference merges scopes across control flow boundaries, so there is no control-flow-scoped scope to place declarations into. This is NOT a codegen-only fix — it requires scope inference improvements (Stage 3) as a prerequisite.

6. **B2 (variable name preservation) is scope-inference dependent (2026-03-26).** Many B2 fixtures (temps vs original names) also have scope boundary differences. Pure codegen name changes won't pass them without scope inference fixes. Stage 1d Phase 3 (merge decl+init) also implemented and gained +0 (dormant). The codegen-only ceiling for slots-MATCH has been reached.

7. **Re-enabling removed bail-outs as per-function bails gained +4 (2026-03-26).** Known-incompatible import bail (+3) and ESLint suppression bail (+1) were re-enabled as per-function bails matching upstream behavior. The initial full removal was too aggressive -- upstream bails per-function, not file-level.

8. **Scope inference experiment results confirm merging bottleneck (2026-03-25, re-confirmed 2026-03-26).** Three experiments on `is_allocating_instruction` heuristics all produced net-negative results: removing `last_use > instr_id` gate (-5 net), inclusive gate (no-op), removing `check_nested_method_call_as_argument` (-2 net). The `merge_overlapping_reactive_scopes_hir` pass is the root cause -- it over-merges when given additional sentinel scopes. The SLOTS-MATCH pool (238 fixtures) is dominated by variable naming divergence (~70+), scope boundary ordering (~30+), and declaration placement (~13), confirming that codegen-only fixes have reached their ceiling. No single-session path to 600+ exists; progress requires fundamental scope merging algorithm improvements.

9. **Recursive ref check and PropertyLoad hook-name check are both load-bearing (2026-03-26).** Removing the recursive ref access check in `validate_no_ref_access_in_render` caused -9 regression — many fixtures depend on the recursive check to correctly bail. Separately, modifying the `PropertyLoad` callee-name hook check in `validate_hooks_usage` was net-zero. Additionally, hook-as-value false positives from locally-declared names (e.g., `let useFeature = makeObject()`) were fixed via a `locally_declared_names` set, but the 3 affected fixtures are still caught by a separate PropertyLoad check, so no net conformance change. The PropertyLoad check remains a future opportunity once it can be made more precise without regressions.

### Revised Path to 600+

The path is clearer but requires significant compiler infrastructure work:

| Work Item | Pool Size | Potential Gain | Difficulty |
|-----------|-----------|---------------|------------|
| Scope inference fixes (slots-DIFFER) | 688 | +50-100 | HIGH — cascading regression risk, scope MERGING is bottleneck (see 3b blocker reports: heuristic removal 2026-03-25, merging investigation 2026-03-26). Three additional approaches failed (-17 to -107 net). Requires mutable range accuracy + dep.reactive audit first. |
| DCE + constant propagation (both-no-memo) | ~85 remaining (7 done) | +5-15 remaining (revised down) | MEDIUM-HIGH — DCE+CP+branch-elim all implemented. Remaining ~85 blocked by 0-slot codegen, not DCE/CP. Binary/string folding may chip away at a few. |
| `validatePreserveExistingMemoizationGuarantees` gaps | 32 (revised from 60) | +30 done (enable default) + prior +31, remaining BLOCKED | MEDIUM — Enabling preserve-memo default in harness was +30 free win (496->526). validateInferredDep further gains BLOCKED: temp-name-skipping caused -56 regression (4th failed approach). See Stage 2i blocker. |
| Variable name preservation in codegen (B2) | 40 | +10-20 | MEDIUM-HIGH — scope output naming changes + scope inference dependency (see lesson #29) |
| Declaration placement / instruction ordering (A1) | 55+ | +15-30 | HIGH — BLOCKED: Phase 2 requires scope inference (Stage 3). Phase 3 (merge decl+init) DONE (+0 dormant). |
| Remaining bail-out fixes (2d-2g, 2j residual) | ~74 total bail pool (was ~78, -4 from Stage 2j) | +15-25 | MEDIUM — per-validation fixes. 3 Infer fixtures remain (need directive support). |
| Stage 4f "Found 1 error" bails (remaining) | 15 (was 29, -10 try-catch, -4 computed key) | +10-15 remaining | LOW — zero regression risk, bail-to-pass. Groups B/D/E/F/H tractable. |
| Todo error detection (remaining) | 4 | +2-4 | LOW-MED — need optional-chain-in-ternary, hoisting, context var |
| Frozen-mutation validation fixes | 1 remains | +10 done (Stage 4d + follow-up) | MEDIUM | 1 remaining needs JSX capture analysis |

**Conservative estimate:** +60-200 from 540 base = 600-740. Reaching 600 requires scope inference work (the largest and highest-risk category). Non-scope-inference gains: Stage 4f remaining (15 "Found 1 error" bails, +10-15), individual error.* bail-outs (+1-2 each), 3 Infer mode fixtures (directive support). Stage 2i preserve-memo false-positive bails DEFINITIVELY BLOCKED (4 approaches, all net-negative, worst -56). DCE/CP potential revised down to +5-15 (blocked by 0-slot codegen).

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

#### Stage 1g: Gating Directive Stripping + Hooks Validation Hardening -- COMPLETE (+2, 505->507)

Completed 2026-03-26. Two improvements shipped, netting +2 conformance.

1. **Gating directive comment stripping (+2):** `codegen.rs` `apply_compilation` now filters `// @gating` and `// @dynamicGating` comment lines from compiled output when gating mode is active. Upstream's Babel plugin removes these annotations during compilation; our source-edit-based approach preserved them. The fix iterates over output lines and suppresses those whose trimmed content starts with `@gating` or `@dynamicGating`. Trailing newline preservation also corrected. Fixtures gained: `gating/multi-arrow-expr-export-gating-test.js`, `gating/multi-arrow-expr-gating-test.js`.

2. **Hook-as-value false positive prevention (+0, correctness fix):** `validate_hooks_usage.rs` gained `locally_declared_names` — a `HashSet<String>` populated by walking `DeclareLocal` and `Destructure` instructions (the latter via a new `collect_destructure_names` helper that recursively extracts all bound names from `DestructurePattern`). Rule 3 (hooks-as-values check) now skips `LoadLocal` of any name present in the set, preventing false bails on patterns like `let useFeature = makeObject()`. No net conformance change — the 3 affected fixtures are still caught by the `PropertyLoad` callee-name check (a separate issue), so this is a correctness improvement that prevents future false bails as the hook validation evolves.

3. **Bail-out fixture name tracking (+0, diagnostics):** `conformance_tests.rs` gained per-fixture name display in the bail-out breakdown diagnostic. Each bail-out error category now lists up to 8 fixture names, making it possible to trace which specific test is responsible for each bail-out category without manual investigation.

**Investigated but reverted (net zero):**
- **PropertyLoad hook-name check modification (net zero):** Attempted to modify how `PropertyLoad` callee names are resolved for hook detection. No conformance change in either direction.
- **Scope inference `is_allocating_instruction` gate removal (-5, reverted):** Removing the `last_use > instr_id` gate created extra sentinel scopes that `merge_overlapping_reactive_scopes_hir` over-merged, breaking 12 previously-passing fixtures. Documented in Stage 3b Extended Experiment Results.
- **Recursive ref check removal (-9, reverted):** Attempted removing recursive ref access checks. Caused -9 regression because the check is load-bearing for ref validation correctness.
- **`check_nested_method_call_as_argument` removal (-2, reverted):** Despite being a false-positive bail for 6 fixtures, removing it caused those 6 to produce bad codegen output. Our codegen genuinely cannot handle nested method calls yet. Documented in Stage 3b Extended Experiment Results.

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

#### Stage 2d: Fix Frozen-Mutation False Positives (26 remaining per latest breakdown) -- BLOCKED

- [ ] Review `validate_no_mutation_after_freeze` / `InferMutableRanges` for over-reporting mutations on frozen values
- [ ] Compare our validation logic against upstream's to find divergence
- [ ] Implement targeted relaxations without losing true-positive detections
- [ ] **NEW (post Stage 4d):** Fix 4 IIFE-pattern false positives introduced by name-based freeze tracking (`capturing-func-alias-*-iife.js`). These fixtures mutate a captured variable inside an IIFE, but the name-based tracker incorrectly treats this as mutation-after-freeze. Future fix: implement scoped name tracking that distinguishes IIFE-internal mutations from true post-freeze mutations.
- **Note (2026-03-26):** Destructure freeze propagation and Check 4b effect callback analysis were completed as part of Stage 4d follow-up. These were true-positive detection improvements, not false-positive fixes. The 26 remaining false positives in the bail pool are the false-positive problem.
- **BLOCKED (2026-03-26):** Three approaches attempted and failed. Root cause is transitive freeze propagation in aliasing pass. See [bail-out-investigation.md](bail-out-investigation.md) for full blocker report.
- **Risk:** HIGH — requires aliasing pass changes with high regression risk
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)

#### Stage 2e: Fix Ref-Access False Positives (8 fixtures) — LOW PRIORITY, NO CONFORMANCE IMPACT

- **Investigated (2026-03-25):** Thoroughly analyzed whether relaxing ref-access false positives would improve conformance. Result: **no gain**. Freed fixtures land in slots-DIFFER (not matched); 2 accidental Flow parse error matches would be lost. Net: -2 to +0.
- **Decision:** Deprioritized until scope inference improvements (Stage 3) make freed fixtures matchable.
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)

#### Stage 2f: Fix Reassignment False Positives (10 fixtures) -- FAILED (2026-03-26)

- [ ] Review `validateLocalsNotReassignedAfterRender` for false positives
- [ ] Compare against upstream validation
- **Risk:** MEDIUM
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)

##### Blocker Report -- Stage 2f validateLocalsNotReassignedAfterRender (2026-03-26)

**Approach attempted:** Relaxing the `validateLocalsNotReassignedAfterRender` check to reduce false positives.

**What was discovered:** Relaxing the check caused 5 regressions vs 1 gain (-4 net). The validation fires on patterns involving `DeclareContext`/`StoreContext` HIR instructions which we do not lower. Without correct context variable tracking in the HIR, the validator cannot distinguish legitimate reassignments from false positives.

**Prerequisites for a successful attempt:**
- `DeclareContext`/`StoreContext` HIR lowering must be implemented so the validator can correctly identify context variables
- This is the same prerequisite as the nested HIR LoadContext gap (Stage 4e-E)

**Do NOT attempt again until:** DeclareContext/StoreContext HIR lowering is implemented.

#### Stage 2g: Other Bail-out Fixes (remaining ~34 fixtures, excluding preserve-memo) -- PARTIALLY COMPLETE (+6, 499->505)

**Latest gains (2026-03-26, error fixture sweep +6):**
1. **Duplicate fbt tags detection (+2):** `check_fbt_duplicate_tags` in `validate_no_unsupported_nodes.rs`. Two-pass analysis: collects fbt/fbs identifiers via LoadLocal/LoadContext/LoadGlobal, then counts `_enum`/`_plural`/`_pronoun` MethodCall sub-tags. Bails if any sub-tag type appears 2+ times. **Note:** `import fbt from 'fbt'` creates LoadLocal not LoadGlobal (fbt is not in built-in globals list). Fixtures: `fbt/error.todo-fbt-unknown-enum-value.js`, `fbt/error.todo-multiple-fbt-plural.tsx`.
2. **Ref-to-function detection (+1):** Added CallExpression arg check in `validate_no_ref_access_in_render.rs` — detects when a ref identifier is passed as argument to a non-hook function call. Fixture: `error.invalid-pass-ref-to-function.js`.
3. **Self-referencing const declarations (+1):** `check_self_referencing_declarations` in `validate_no_unsupported_nodes.rs`. Detects `const x = identity(x)` pattern where LoadLocal references the same IdentifierId before the matching StoreLocal. Only fires for `Const` kind (not `Let`). Initial broader version caused -11 regression on destructured params; final version scoped to non-temp identifiers with DeclareLocal/Destructure boundaries. Fixture: `error.dont-hoist-inline-reference.js`.
4. **Dynamic gating invalid identifier validation (+2):** `is_valid_js_identifier` in `program.rs`. Validates `'use memo if(cond)'` directive conditions: must be valid JS identifier (not keyword/literal like `true`/`false`/`null`). Fixtures: `gating/dynamic-gating-invalid-identifier-nopanic.js`, `gating/error.dynamic-gating-invalid-identifier.js`.

**Also attempted but REJECTED: 0-slot codegen** — Tried emitting passthrough code (no `_c()` wrapper) when cache slot count is 0. Caused **-52 regression** (505->453). Root cause: many fixtures have 0 expected slots but different structural transformations in expected output; removing the wrapper changes codegen structure in ways that don't match. 0-slot codegen is NOT viable until scope inference accuracy improves to reduce surplus scopes.

- [ ] Fix remaining false-positive bail-outs: setState-in-render (4), setState-in-effect (2), hooks (3), exhaustive-deps (1), silent (8), other (~10)
- [ ] Each fix: compare upstream validation logic, adjust our thresholds
- [ ] Re-categorize after 2c-2f to identify new patterns

#### Stage 2i: Fix Preserve-Memo False-Positive Bails (55 fixtures) -- BLOCKED (2026-03-26)

**BLOCKED:** Four approaches attempted and ALL failed. (1) pre-inline temp map, (2) skip unnamed in propagation, (3) skip "tN" in validation — all caused -31 regression. (4) **temp-name-skipping in validateInferredDep (2026-04-03)** — skipping deps with temp names (`t0`-`t99`) in the validation comparison. Caused **-56 regression** (540->484). Definitively confirms this approach family is not viable. See blocker reports below and in [bail-out-investigation.md](bail-out-investigation.md).

**Pool:** 55 "Existing memoization could not be preserved" false-positive bails (was 4 pre-validateInferredDep, +51 introduced by Stage 4b). Single largest bail-out category.

**Root cause:** `validate_preserved_manual_memoization.rs` `validate_scope_deps` fires incorrectly because scope dep IdentifierIds resolve to SSA temporaries (via `resolve_scope_dep`), not original named variables. When the resolved dep's name is a temp (e.g., `t5`) that doesn't match any manual memo dep (e.g., `props.x`), the check incorrectly reports a mismatch.

**Fix approach:** Improve the `build_temporaries_map` / `build_temporaries_map_from_hir` to cover more instruction chains. The current map handles LoadLocal -> PropertyLoad chains but may miss:
- Chains broken by SSA phi nodes (the HIR temp's defining LoadLocal is in a different block)
- Chains involving Destructure instructions
- Chains where intermediate instructions were removed by optimization passes before the pre-inline map was captured
- Cases where the scope dep identifier has a name but it's a temp name (e.g., `t5`), not the original variable name

**Investigation steps:**
1. List all 55 false-positive bail fixtures by name
2. Sample 10 fixtures and trace which scope deps fail resolution
3. Categorize: (a) dep in temporaries map but wrong name, (b) dep NOT in temporaries map, (c) dep resolved correctly but comparison logic wrong
4. Fix the most common resolution failure pattern

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/Validation/ValidatePreserveExistingMemoization.ts`
**Our file:** `crates/oxc_react_compiler/src/validation/validate_preserved_manual_memoization.rs`

**Potential gain:** Eliminating 51 false-positive bails. Even if only 20-30% of freed fixtures match upstream output, that's +10-15 direct conformance gain. The rest move from bail to slots-DIFFER/MATCH, unblocked for future scope inference fixes.

**Why this WAS the top priority (now BLOCKED):** It is the single largest bail-out category (55 fixtures), but four implementation attempts confirmed that fixing the false bails causes massive regression. See blocker reports below and in [bail-out-investigation.md](bail-out-investigation.md).

##### Blocker Report — Temp-name-skipping in validateInferredDep (2026-04-03)

**Approach attempted:** Skip validation of inferred deps whose resolved name is a temp pattern (`t0` through `t99`) in `validate_preserved_manual_memoization.rs`. The hypothesis was that skipping temp-named deps would eliminate false-positive "cannot preserve memoization" bails without affecting true-positive detection.

**Assumption that was wrong:** Assumed that skipping temp-named deps would only suppress false positives. In reality, many error.* fixtures that SHOULD bail rely on the validateInferredDep check firing on temp-named deps. When those checks are skipped, error.* fixtures lose their bail path and incorrectly compile, moving from "we bail = pass" to "we compile, they don't = fail."

**What was discovered:** The **-56 regression** (540->484) is the worst result of any Stage 2i approach. The regression is LARGER than the -31 from the "skip tN" approach (attempt #3) because:
1. More error.* fixtures depend on temp-named dep validation than on named dep validation
2. De-bailed fixtures uniformly land in slots-DIFFER (our codegen doesn't match upstream), not passing
3. The temp-naming pattern (`tN`) is too broad — it catches legitimate validation targets, not just false positives

**Regression details:** -56 net (540->484). Zero new passes. All regression from error.* fixtures losing their bail path.

**Conclusion:** ALL four approaches to reducing preserve-memo false-positive bails produce net-negative results. The fundamental issue is that the false-positive bails are entangled with true-positive bails on error.* fixtures — any change that reduces false positives also reduces true positives. The only viable path is to fix the UNDERLYING scope dep resolution problem (SSA temp -> named variable mapping) so that the validation can correctly distinguish between true and false positives.

**Do NOT attempt any further "skip" or "filter" approaches.** The next attempt must fix scope dep resolution (see Deferred/Blocked: Scope Dep Resolution).

#### Stage 2j: Tighten CompilationMode::Infer Heuristics -- COMPLETE (+4, KF reconciliation absorbed into 507->496)

Completed 2026-04-03. Added `body_has_hooks_or_jsx` function to `program.rs` that performs a shallow AST walk to detect hook calls and JSX elements in Infer mode. Functions without hooks or JSX in their own body (not nested functions) are now skipped in Infer mode, matching upstream's `hasHooksOrJsx` heuristic.

**Implementation details:**
- `body_has_hooks_or_jsx(stmts)`: shallow walk over statements, descends into control flow (if/for/while/switch/try) but NOT into nested function expressions/arrow functions
- `expr_has_hooks_or_jsx(expr)`: checks for JSX elements/fragments and hook calls (via `call_is_hook`)
- `call_is_hook(call)`: checks if callee name starts with "use" (Identifier or StaticMemberExpression)
- Applied to `should_compile`, `should_compile_default_export`, and all call sites (variable declarations, function declarations, default exports, discovery pass)
- For components (not hooks), the body check gates compilation; hooks always compile regardless

**Fixtures gained (4, removed from KF):**
- `dont-memoize-primitive-function-call-non-escaping.js` — returns string, no JSX
- `infer-skip-components-without-hooks-or-jsx.js` — no hooks/JSX, returns function call
- `infer-no-component-nested-jsx.js` — JSX only in nested function
- `infer-no-component-obj-return.js` — returns object, not JSX

**Fixtures NOT gained (3 of original 7):**
- `dont-memoize-primitive-function-call-non-escaping-useMemo.js` — has `useMemo` hook call, so body_has_hooks_or_jsx returns true (correctly compiles)
- `should-bailout-without-compilation-infer-mode.js` — requires gating + panicThreshold:none directive support
- `valid-setState-in-useEffect-controlled-by-ref-value.js` — requires `@enableAllowSetStateFromRefsInEffects` directive support

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/Entrypoint/Pipeline.ts` (`hasHooksOrJsx`)
**Our file:** `crates/oxc_react_compiler/src/entrypoint/program.rs` (`body_has_hooks_or_jsx`, `should_compile`, `should_compile_default_export`)

#### Stage 2h: Replan -- Bail-out Residual (est: 0 fixtures, planning)

- [ ] Categorize remaining "we bail, they compile" after 2c-2g+2i+2j
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
- **Both-no-memo (was 79, now ~76 after Stage 5a DCE+CP):** DCE + constant propagation passes partially implemented. 7 fixtures now passing. Remaining need dead branch elimination.
- **Slot-diff deficit distribution:** -1 (131), -2 (120), -3 (35), -4 (42), -5 (16), -6 (22), -7 (4), -8+ (32). Total deficit: 402. Total surplus: 286.

#### Stage 3a2: Investigate & Fix 0-Slot Surplus Fixtures (134 fixtures, est: +30-80) — INVESTIGATED/BLOCKED (+1, 456->457)

**Pool:** 134 fixtures where upstream produces 0 cache slots (no memoization) but we produce >0 slots. These are currently miscategorized as "we compile, they don't" but are actually scope inference surplus fixtures where expected_slots = 0.

**Critical insight (2026-03-26):** These fixtures were previously described as "no upstream error header -- not actionable without specific validation ports." This was WRONG. Investigation revealed upstream DID compile them successfully -- it just produced 0 reactive scopes. The expected output files contain structurally transformed code (e.g., extracted arrow functions, renamed variables) but zero `_c()` calls. We produce memoized output with cache slots.

**Examples:**
- `arrow-function-one-line-directive.js`: Upstream extracts arrow to `_temp` function, 0 slots. We memoize with slots.
- `call-spread-argument-mutable-iterator.js`: Upstream produces passthrough-like output, 0 slots.
- `block-scoping-switch-dead-code.js`: Upstream transforms switch/dead-code structure, 0 slots.

**Completed work (2026-03-26):**
- [x] Enhanced `prune_non_escaping_scopes` to detect condition-test-only scope declarations and prune those scopes (+1 fixture: `escape-analysis-not-if-test.js`). Added ~300 lines: `collect_test_position_ids`, `collect_value_use_ids`, `propagate_alias_chains`, `is_scope_only_test_used`. This is a DIVERGENCE from upstream (which uses `ValueKind::Primitive` / escape flags); we use set-based analysis instead.
- [x] Investigated 3 remaining `escape-analysis-not-*` fixtures (`conditional-test`, `switch-case`, `switch-test`) — **BLOCKED by scope inference merging (Stage 3b)**. The array scope (`[...].map(...)`) is merged with the result scope at the HIR level, so both map to the same reactive scope and the result identifier never appears isolated in the test position.
- [x] Investigated root cause of 134 zero-slot surplus fixtures — **primarily scope inference issues (scopes spanning hook calls, over-merging), NOT missing prune logic**. The pruning enhancements can chip away at individual patterns but the dominant root cause is scope inference creating scopes that upstream doesn't create in the first place.

**Remaining investigation plan:** ~~SUPERSEDED by blocker report below.~~ The 2026-03-26 deep-work session investigated three approaches (per-function reactive guard, `is_allocating` guard removal, pruning analysis) and confirmed the root cause is scope CREATION (mutable range width), not pruning. No further investigation of pruning-based approaches is warranted. The path forward is Stage 3b (scope merging / mutable range accuracy).

**Upstream files:** `src/ReactiveScopes/PruneNonEscapingScopes.ts`, `src/ReactiveScopes/PruneNonReactiveDependencies.ts`, `src/ReactiveScopes/PruneUnusedScopes.ts`, `src/Optimization/DeadCodeElimination.ts`
**Our files:** `prune_scopes.rs`, `infer_reactive_scope_variables.rs`

**Why this is high priority:** 134 fixtures is the single largest addressable pool. If even 30% share a common root cause, fixing it yields +40 conformance. Removing scopes (making output more conservative) is SAFER than adding scopes -- less regression risk.

**Key finding (2026-03-26):** The remaining 3 escape-analysis-not fixtures and the broader 134-fixture surplus pool are both dominated by scope inference merging issues. The pruning layer can only eliminate scopes that exist as discrete entities — when scope inference merges two conceptually separate scopes into one, the pruning layer cannot split them back apart. This reinforces that Stage 3b (scope merging fixes) is the critical path for large conformance gains in the surplus pool.

### Blocker Report — Stage 3a2 Zero-Slot Surplus Investigation (2026-03-26)

**Approach attempted:** Three strategies to reduce scope surplus in the 134 zero-slot fixtures:

1. **Per-function reactive guard (`function_has_any_reactive`):** Add an early-exit in scope inference that skips scope creation for functions with no reactive identifiers.
2. **`prune_unused_scopes` `is_allocating` guard removal:** Remove the guard that prevents pruning of allocating scopes, allowing more scopes to be pruned away.
3. **Analysis of pruning vs. creation:** Determine whether the surplus comes from missing prune logic or over-aggressive scope creation.

**Assumption that was wrong:** The 134 surplus fixtures were assumed to be fixable via pruning enhancements or simple guards. In reality, the surplus is a scope CREATION problem, not a pruning problem.

**What was discovered:**

1. **Per-function reactive guard:** Too aggressive. Caused -44 regression (464 to 420) because most functions DO have reactive identifiers even when individual scope sets are allocating-only. The guard needs to be per-scope-set, not per-function, but per-scope-set guards require knowing the scope boundaries before they are created (circular dependency).

2. **`prune_unused_scopes` `is_allocating` guard removal:** Gained +3 but caused unresolved reference bug (`setActive` in `semantic_conditional_component`). Root cause: some allocating scopes have declarations whose IDs are not in `scope.declarations` because they are introduced by patterns inside the scope (like destructuring from `useState`). Removing the guard causes scope pruning that drops needed variable bindings.

3. **`prune_non_escaping_scopes` already correctly keeps escaping allocating scopes.** The surplus is not a pruning problem.

4. **The real root cause:** `infer_reactive_scope_variables` creates sentinel scopes for allocating instructions even when those allocations do not need memoization. Upstream avoids this because their scope creation uses narrower mutable ranges that do not group allocating instructions into scopes when they have no reactive deps. Our `last_use_map` extension causes wider ranges that group more instructions into scopes.

**Regression details:**
- Per-function reactive guard: 464 to 420 (-44 regression)
- `is_allocating` guard removal: +3 gain but introduced unresolved reference (`setActive` in `semantic_conditional_component` fixture)

**Prerequisites for a successful attempt:**

- Mutable range accuracy must be improved so that `infer_reactive_scope_variables` does not group allocating instructions into scopes when they have no reactive dependencies. This is the same prerequisite as Stage 3b (scope merging).
- The `last_use_map` extension in `infer_mutation_aliasing_ranges.rs` must be narrowed or replaced. Currently it extends ranges to last USE (not just last MUTATION), which is wider than upstream. But removing it causes codegen regressions because scope containment depends on wide ranges. This requires: (a) receiver mutation effects for MethodCall/Apply, and (b) a reverse scope propagation pass.
- Per-scope-set reactive analysis (not per-function) would be needed for a guard-based approach, but this creates a circular dependency with scope creation.

**Useful findings to carry forward:**

- `infer_reactive_scope_variables.rs` — the `is_allocating_instruction` function creates sentinel scopes. The upstream equivalent uses narrower mutable ranges that naturally exclude non-reactive allocations.
- `prune_scopes.rs` — `prune_unused_scopes` has an `is_allocating` guard that is load-bearing: removing it exposes `scope.declarations` incompleteness for destructuring patterns.
- `prune_non_escaping_scopes` — already correct for escaping allocating scopes. No further pruning gains available here.
- The 134 zero-slot surplus fixtures overlap with the 286 total surplus fixtures in slots-DIFFER. They are the subset where `expected_slots = 0`.
- `scope.declarations` does not include all identifiers introduced within a scope (e.g., destructured bindings from `useState`). Any future scope pruning work must account for this.

**Do NOT attempt again until:** Stage 3b (scope merging / mutable range accuracy) prerequisites are resolved. The 134-fixture surplus requires fundamental scope inference changes (`last_use_map` narrowing, scope grouping algorithm), not pruning fixes. This is the same root cause as Stage 3b.

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

##### Blocker Report — Scope Inference Merging Investigation (2026-03-26)

**Approach attempted:** Three approaches from the Stage 3b plan were implemented and tested:

1. **Operand liveness change (Step 1):** Changed Phase 2 operand liveness check to use `mutable_range.end` instead of `effective_range` (max(mutable_range.end, last_use+1)). Two variants tested:
   - Blanket change (all instructions): -24 net (2 new passes, 26 regressions). Massive over-splitting — fixtures showed ours=_c(3) vs expected=_c(1).
   - Targeted change (allocating instructions only): -17 net (0 new passes, 17 regressions). Still over-splits because even allocating instruction operands have narrow mutable ranges.

2. **0-slot function codegen:** Attempted outputting DCE'd code for 0-slot functions by removing `has_cache_slots` check and conditionally adding runtime import. -51 net regression. Codegen output format doesn't match source text (different whitespace, ordering, structure). Previous attempt also documented: -52 in Stage 2g.

3. **Prune non-reactive dependencies (Step 3):** Implemented `scope_block.scope.dependencies.retain(|dep| dep.reactive)` to remove non-reactive deps. -107 net regression. Removing non-reactive deps turns dep-based scopes into sentinel scopes (0 deps), drastically changing codegen output.

**Assumptions that were wrong:**
- The plan assumed Step 1 (operand-only change) was distinct from prior `use_mutable_range` experiments. While technically true (prior experiments changed both lvalue gate + operand check), the over-splitting is caused by narrow mutable ranges in BOTH paths.
- The plan assumed allocating instructions could be treated differently for operand liveness. In practice, allocating instruction operands also have narrow mutable ranges that need effective_range extension.
- The plan assumed `prune_non_reactive_deps` was safe because upstream does it. In practice, our scope dep model differs significantly — many deps that are marked non-reactive in our system ARE reactive in upstream's model.

**Regression details:**
- Operand liveness (blanket): -24 net (2 new passes, 26 regressions)
- Operand liveness (targeted, allocating only): -17 net (0 new passes, 17 regressions)
- 0-slot codegen: -51 net regression
- Non-reactive dep pruning: -107 net regression

**Prerequisites for a successful attempt:**
- **Fix mutable range accuracy FIRST** — before any operand liveness changes. Our `infer_mutation_aliasing_ranges` produces narrower ranges than upstream's because we lack: (a) receiver mutation effects for MethodCall, (b) reverse scope propagation. Until mutable ranges are widened, any switch from effective_range to mutable_range will over-split.
- **The dependency reactive flag is unreliable** — many deps marked `reactive=false` still need to be tracked for correct codegen. The reactive flag assignment in `propagate_dependencies.rs` needs audit against upstream's logic before pruning deps.
- **0-slot codegen requires a separate code path** — the current codegen reconstructs functions from IR, which produces fundamentally different output from source text. A viable approach would need to track which specific instructions were removed by DCE/CP and surgically edit the source text rather than reconstructing from IR.

**Useful findings to carry forward:**
- Over-splitting root cause is narrow mutable ranges, not the operand liveness check itself. The check is downstream of range accuracy.
- Non-reactive deps in our model do NOT correspond to non-reactive deps in upstream. The `dep.reactive` flag assignment in `propagate_dependencies.rs` diverges from upstream semantics.
- 0-slot codegen has been attempted twice now (-52 in Stage 2g, -51 here). Both confirm IR reconstruction cannot match source text formatting. Any future attempt MUST use source-text-editing approach, not IR reconstruction.

**Confirmed: No single-session path to 600+.** All three plan steps confirmed what the honest assessment already states: reaching 600+ requires fundamental infrastructure work on mutable range accuracy and scope grouping algorithms. No incremental changes to the current system can bridge the 93-fixture gap.

**Do NOT attempt again until:** (a) mutable range accuracy is improved (receiver mutation effects for MethodCall, reverse scope propagation), (b) `dep.reactive` flag semantics are audited against upstream, (c) 0-slot codegen approach is redesigned to use source-text editing rather than IR reconstruction.

##### Extended Experiment Results (2026-03-25)

Three additional approaches were tested to validate and expand on the blocker report findings:

1. **Removing `last_use > instr_id` gate entirely:** -5 net (505->500). The gate currently prevents sentinel scope creation for calls whose results are immediately consumed. Removing it creates extra scopes that then get over-merged by `merge_overlapping_reactive_scopes_hir`, inflating counts for 12 previously-passing fixtures. The -1 deficit improved by 4 (146->142) but this was offset by 12 regressions in other categories. **Confirms blocker report conclusion:** scope MERGING is the bottleneck, not scope creation.

2. **`last_use >= instr_id` (inclusive gate):** No effect at all. `last_use` is never exactly equal to `instr_id` in our HIR because `last_use` tracks the RESULT identifier's last reference, which is always a different instruction from the defining instruction. This variant is a no-op.

3. **Removing `check_nested_method_call_as_argument`:** -2 net (505->503). Despite being a false-positive bail for 6 fixtures (upstream compiles them), removing the check causes those 6 to produce bad codegen output. Our codegen actually DOES have the MethodCall property issue -- the bail is load-bearing until codegen handles nested method calls correctly.

**Silent bail analysis (8 fixtures):** 8 fixtures produce 0 scopes but upstream produces scopes. Root causes: (a) IIFE patterns not creating scopes, (b) function inference not triggering on React.memo wrappers, (c) Flow type cast default values. All require scope inference improvements, not bail-out fixes.

**SLOTS-MATCH analysis (238 fixtures, normalized diff patterns):** After normalization, the dominant divergence patterns in the slots-MATCH pool are:
- **Variable naming (~70+ fixtures):** We use temp names (`t0`, `t1`) where upstream preserves original names. Overlaps with B2 pattern but broader than previously estimated.
- **Scope boundary ordering (~30+ fixtures):** Our scopes start/end at different instructions than upstream, causing structural output differences even when slot counts match.
- **Declaration placement (~13 fixtures as first diff):** `const x;` + `x = value;` vs `const x = value;`. Stage 1d Phase 3 implemented but dormant (these fixtures also differ in other ways).
- **Gating import order (~6 fixtures):** Minor ordering differences in gating imports; 2 fixtures have only 4 diffs total, making them near-passing.

**Honest assessment of 600+ goal:** No single-session path exists. Reaching 600+ requires either:
- (a) Scope inference fixes that net positive (current best attempt: -5), OR
- (b) Variable naming preservation (systemic, ~40+ fixtures affected), OR
- (c) 0-slot codegen (blocked, -52 regression when attempted, ~85 fixtures)

Each path requires fundamental infrastructure work. The most promising remains scope inference (largest pool, ~688 fixtures), but it carries the highest regression risk.

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

**Pool:** ~187 fixtures where we produce memoized output but upstream doesn't (was 191, -4 from validation fixes session). **CORRECTED (2026-03-26):** 134 are scope inference surplus (upstream compiles with 0 slots, NOT validation gaps -- see Stage 3a2). Remaining ~53: 18 UPSTREAM ERROR (in KF, need bail), ~24 preserve-memo, ~3 other.
**Risk:** LOW — adding validations is safe (bail-out = pass-through = correct).

#### Stage 4a: Categorize Missing Validations -- COMPLETE (investigation)

Completed 2026-03-25 (extended investigation), revised 2026-03-26.

**Revised breakdown (2026-03-26):** Of the ~186 "we compile, they don't" fixtures:
- **134 are scope inference surplus** — upstream compiles with 0 reactive scopes, we produce >0. NOT validation gaps. See Stage 3a2 investigation.
- **32 are preserve-memo** — `validatePreserveExistingMemoizationGuarantees` gaps (revised down from 60).
- **Remaining ~20:** todo-bail (4), frozen-value (2), flow-parse (15, not actionable), mixed.

| Sub-category | Count | Action Needed |
|-------------|-------|---------------|
| Scope inference surplus (0 expected slots) | 134 | Scope inference issue — upstream compiles with 0 reactive scopes, we over-memoize. See Stage 3a2. |
| UPSTREAM ERROR fixtures (expected output IS the error) | 12 remaining in KF (was 75, 22 fixed in Stages 4c+4e-A, +31 by preserve-memo, +4 by validation fixes session, +6 by Stage 2g sweep) | Must bail (not transform) to pass — error message matching NOT required |
| `validatePreserveExistingMemoizationGuarantees` gaps | 32 (revised from 60) | Extend existing preserve-memo validation |
| `Todo` error detection (unimplemented features) | 2 remaining (27 done, +3 from 4e-D, +1 from validation fixes session, +1 from Stage 2g: self-ref const) | 2 need optional-chain-in-ternary (2), context var detection (1). |
| Frozen-mutation detection gaps | 1 remains (10 fixed) | 10 fixed in Stage 4d + follow-up; 1 remains (JSX capture) |
| Other validation gaps (ref-access, reassignment, hooks) | ~73 (was ~80, -4 from validation fixes session, -3 from Stage 2g: ref-to-function + fbt overlap) | Various per-validation fixes |

#### Stage 4b: Implement `validatePreserveExistingMemoizationGuarantees` Fixes (32 preserve-memo fixtures in "we compile, they don't")

**Updated breakdown (2026-03-26, revised down from 60 to 32):** The preserve-memo fixtures in the "we compile, they don't" category. Previous count of 60 included fixtures that are actually in other categories. Revised to 32 after re-analysis. These break into 3 distinct sub-types:

| Sub-type | Count | Status | What's needed |
|----------|-------|--------|---------------|
| `validateInferredDep` not implemented | 26 | **PARTIALLY COMPLETE** — 3 of 32 target error fixtures pass; 29 BLOCKED by scope dep resolution | Port upstream's `validateInferredDep` checks — validates that inferred dependencies match manual memo deps |
| "value was memoized" check improvement | 17 | Not started | Improve detection of whether a value was actually memoized by the compiler (our check is too permissive) |
| "dependency may be mutated" tracking | 17 | Not started | Track whether dependencies of manual memos may be mutated, triggering preserve-memo bail-out |

- [x] Audit our `validate_preserved_manual_memoization.rs` against upstream
- [~] Port `validateInferredDep` checks (26 fixtures — largest sub-type) — **3 of 32 target error fixtures now pass, 29 BLOCKED by scope dep IdentifierId mismatch (see blocker report below)**
- [ ] Fix "value was memoized" detection (17 fixtures)
- [ ] Add "dependency may be mutated" tracking (17 fixtures)
- [ ] **Risk:** MEDIUM — our implementation exists but has known gaps
- [ ] **Potential gain:** +30-45 fixtures (some may also need other fixes), but validateInferredDep remaining 29 are BLOCKED

##### validateInferredDep Implementation Notes (2026-03-26)

**What was implemented:** Ported the core `validateInferredDep` algorithm from upstream `validatePreserveExistingMemoizationGuarantees.ts`. The implementation:
- Extracts inferred dependencies from scope declarations associated with `FinishMemoize` instructions
- Compares inferred deps against manual deps from `StartMemoize` instructions
- Emits `CannotPreserveMemoization` error when an inferred dep is not found in the manual dep list
- Handles the `pruned` flag on `FinishMemoize` to skip validation when the compiler already pruned the scope

**What works (3 fixtures passing):** Cases where scope deps happen to use resolved variable names that match manual memo deps. These are cases where the dependency is a simple named variable that survives SSA without being renamed to a temp.

**What's blocked (29 fixtures):** Scope dependencies after SSA have IdentifierIds that correspond to SSA temporaries (e.g., `t1`, `t2`), NOT the original named variables. When `validateInferredDep` tries to match a scope dep against a manual memo dep (which uses the original variable name like `props.x`), the comparison fails because the scope dep's IdentifierId resolves to a temp name, not `props.x`. This is the fundamental scope dep resolution blocker.

**New false-positive bails (CORRECTED 2026-03-26: +51, not +3):** Conformance run shows **55 total** "Existing memoization could not be preserved" false-positive bails in the "we bail, they compile" category. Pre-validateInferredDep baseline was only **4**. So the validateInferredDep implementation introduced **51 new false-positive bails**, not the 3 originally documented. The original +3 count was based on manual inspection of a few fixtures; the actual regression is much larger. Despite this, the net conformance impact was still positive (+31 from 464->495) because the implementation correctly bails on many true-positive cases. **Fixing the scope dep resolution problem would eliminate up to 51 false-positive bails and potentially recover +10-30 conformance** (depending on how many freed fixtures match upstream output).

##### Blocker Report — Scope dep IdentifierIds don't resolve to named locals after SSA (2026-03-26)

**Approach attempted:** Implemented `validateInferredDep` by iterating over scope declarations and comparing their dependency IdentifierIds against manual memo dep IdentifierIds. Used `identifier_name` (the HIR identifier's name field) to resolve deps back to named variables for comparison.

**Assumption that was wrong:** Assumed that scope dependency IdentifierIds would resolve to the original named variable (e.g., `props`, `x`, `obj.a`). In reality, after SSA renaming, scope dep IdentifierIds point to SSA temporaries (e.g., `t1`, `t2`) because the dependency was computed through a chain of instructions (LoadLocal -> PropertyLoad -> etc.) and the scope dep captures the final temp, not the original source variable.

**What was discovered:** The scope dep resolution problem is structural:
1. Scope dependencies are captured as `Place` references with IdentifierIds
2. These IdentifierIds are assigned during HIR construction and then renumbered during SSA
3. After SSA, the IdentifierId on a scope dep points to a temporary that holds the computed value (e.g., the result of `LoadLocal props` + `PropertyLoad .x`)
4. Manual memo deps reference the original source-level names (e.g., `props.x`)
5. There is no reverse mapping from SSA temp IdentifierIds back to the original source-level property path
6. Upstream solves this by maintaining richer dependency tracking through `PropagateScopeDependencies` that preserves the original property access path. Our `propagate_dependencies.rs` does not preserve this path information.

**Regression details:** +3 new false-positive bails (67->70 total). These are fixtures where `validateInferredDep` fires incorrectly because it cannot match scope deps (SSA temps) to manual deps (named variables).

**Prerequisites for a successful attempt (remaining 29 fixtures):**
- Scope dep resolution must be able to map from an SSA temp IdentifierId back to the original named variable / property path
- This likely requires `propagate_dependencies.rs` to preserve the original dependency path (e.g., `props.x`) alongside or instead of just the final temp IdentifierId
- Alternatively, a post-SSA reverse mapping pass could trace each temp back through its definition chain to find the original source variable
- This is the same fundamental problem that affects B2 variable name preservation — SSA temps obscure original variable identity

**Useful findings to carry forward:**
- The `validateInferredDep` algorithm itself is correctly ported — the issue is purely in dep resolution, not validation logic
- `validate_preserved_manual_memoization.rs` is the implementation file
- Manual memo deps come from `StartMemoize` instruction's `deps` field
- Scope deps come from the reactive scope's `declarations` map
- The 3 passing fixtures are cases where deps happen to be simple named variables that survive SSA without temp indirection

**Do NOT attempt to fix the remaining 29 until:** Scope dep resolution is improved to map SSA temp IdentifierIds back to original named variable paths. This may require changes to `propagate_dependencies.rs` or a new reverse-mapping utility.

##### Previous Investigation Notes (2026-03-25)

~~An investigation was started but not completed. Key finding: our validation exists at Pass 61 but fails to detect errors because `finish_in_scope` is true -- our scope inference wraps `FinishMemoize` in reactive scopes, which causes the validation to skip checks it should be performing. Additionally, upstream has `validateInferredDep` checks that we skip entirely.~~

**Resolved (2026-03-26):** The `finish_in_scope` issue was addressed during the validateInferredDep implementation. The core validation logic now correctly processes FinishMemoize instructions regardless of scope wrapping. The remaining blocker is scope dep IdentifierId resolution (see blocker report above).

#### Stage 4c: Add Todo Error Detection -- PARTIALLY COMPLETE (+15 net, 411->426)

Completed 2026-03-25. Implemented bail-outs for 15 of 27 Todo-error fixtures:
- Try-without-catch blocks (2 fixtures) — added in `hir/build.rs`
- Computed object keys (4 fixtures) — added in `hir/build.rs`
- Value blocks in try/catch (7 fixtures) — added in `validation/validate_no_unsupported_nodes.rs`
- Throw in try (1 fixture) — added in `validation/validate_no_unsupported_nodes.rs`
- Fbt local variables (1 fixture) — added in `validation/validate_no_unsupported_nodes.rs`

**Key finding:** The 16 fixtures originally identified as targets were already passing. The actual Todo-error fixtures were in the known-failures list (UPSTREAM ERROR set). Of 27 in that set, 15 fixed, 12 remain.

**Remaining 4 Todo-error fixtures** (require more complex handling — 7 of original 12 fixed in Stage 4e-A, 1 fixed in Stage 2g):
- Hoisting patterns (2) — `error.todo-functiondecl-hoisting.tsx`, `error.todo-valid-functiondecl-hoisting.tsx` — need function-level hoisting infrastructure
- Optional terminal issues (1) — `error.todo-preserve-memo-deps-mixed-optional-nonoptional-property-chain.js` — need optional chaining terminal handling
- Update expression on context vars (1) — `error.todo-handle-update-context-identifiers.js` — BLOCKED: nested HIR builders don't emit LoadContext (see blocker report in Stage 4e-A)
- ~~For-loop context vars (1) — `error.todo-for-loop-with-context-variable-iterator.js`~~ — already fixed in 4e-D
- **Fixed in 4e-A (moved from this list):** hoisted-function-in-unreachable-code, hoist-function-decls, hook-call-spreads-mutable-iterator, default-param-accesses-local, fbt-as-local, bug-invariant-couldnt-find-binding-for-decl, hoisting-simple-function-declaration
- **Fixed in Stage 2g:** `error.dont-hoist-inline-reference.js` — actually a self-referencing const declaration pattern (`const x = identity(x)`), not a hoisting issue. Fixed via `check_self_referencing_declarations`.

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

#### Stage 4e: UPSTREAM ERROR Fixture Handling (29 "Found 1 error" fixtures in WE-COMPILE-THEY-DON'T pool)

**Critical correction (2026-03-25):** The conformance test does NOT require matching exact error messages. It only checks `!compile_result.transformed` (line 781 of conformance_tests.rs). To pass an UPSTREAM ERROR fixture, we just need to bail (not transform). This is much simpler than originally described.

**Latest gains (2026-03-26, +4 from 495->499):**
- `error.bug-invariant-codegen-methodcall.js` (+1, MethodCall invariant) — added bail-out in `validate_no_unsupported_nodes.rs` for MethodCall codegen patterns
- `error.todo-nested-method-calls-lower-property-load-into-temporary.js` (+1, MethodCall invariant) — same bail-out covers nested method call patterns
- `error.call-args-destructuring-asignment-complex.js` (+1, destructuring assignment) — added bail-out in `build.rs` for complex destructuring assignment in call args
- `error.invalid-setState-in-useMemo-indirect-useCallback.js` (+1, setState-in-useMemo indirect) — fixed `validate_no_set_state_in_render.rs` to detect indirect setState calls through useCallback within useMemo

**Revised breakdown of 12 error.* fixtures remaining in known-failures (was 18 pre-Stage-2g, was 30 pre-validation-fixes, was 36 pre-validateInferredDep):**

| Sub-category | Count | What we need to bail |
|-------------|-------|---------------------|
| "Compilation Skipped: preserve-memo" | 8 (was 11, 3 fixed by validateInferredDep) | `validatePreserveExistingMemoizationGuarantees` must detect and bail — overlaps Stage 4b. Remaining 8 BLOCKED by scope dep resolution. |
| "Todo: hoisting/optional/context-var/etc" | 1 (was 3, -1: self-referencing const fixed in Stage 2g, -1: fbt duplicate tags overlap) | Remaining: optional-chain-in-ternary (2), context var update (1). Note: `error.dont-hoist-inline-reference.js` fixed via `check_self_referencing_declarations` in Stage 2g. |
| "Invariant: ..." (upstream internal errors) | 1 (was 3, -2: MethodCall invariant + destructuring assignment fixed in 499 session) | Remaining: inconsistent destructuring (1), unnamed temporary (1). |
| "Error: This value cannot be modified" | 2 | Frozen-mutation detection — overlaps Stage 4d remaining (1 fixed: effect callback Check 4b) |
| "Error: Cannot modify locals after render" | 2 | `validateLocalsNotReassignedAfterRender` gaps |
| "Error: Cannot access refs during render" | 2 (was 3, -1: ref-to-function fixed in Stage 2g) | `validateNoRefAccessInRender` gaps. `error.invalid-pass-ref-to-function.js` FIXED — ref passed as argument to non-hook function now detected. 2 remaining need further investigation. |
| "Error: setState from useMemo" | 0 (was 1, fixed in 499 session) | `error.invalid-setState-in-useMemo-indirect-useCallback.js` fixed — indirect setState detection through useCallback within useMemo now works. |
| "Error: validate-*" | 3 | validate-blocklisted-imports (1), validate-object-entries/values-mutation (2) |
| Compiled output (NOT UPSTREAM ERROR) | 5 | Slots-DIFFER/MATCH issues, not bail-out issues |

**Tractable sub-tasks (no new infrastructure needed):**

- [x] **4e-A: Mixed bail-outs — COMPLETE (+7, 435->442)** — implemented 7 new bail-outs across 3 files: hoisted function decls in unreachable code (3 fixtures: `error.todo-hoist-function-decls.js`, `error.todo-hoisted-function-in-unreachable-code.js`, `error.hoisting-simple-function-declaration.js`), fbt parameter name detection (1: `fbt/error.todo-fbt-as-local.js`), default-param arrow/function expressions (1: `error.default-param-accesses-local.js`), catch clause destructuring (1: `error.bug-invariant-couldnt-find-binding-for-decl.js`), hook spread arguments (1: `error.todo-hook-call-spreads-mutable-iterator.js`). Files: `validate_no_unsupported_nodes.rs`, `build.rs`, `known-failures.txt`. **Note:** `error.todo-handle-update-context-identifiers.js` (Group 6, UpdateExpression on context vars) was NOT fixed — nested HIR builders don't emit `LoadContext` instructions, so context variables can't be detected by walking the nested HIR. See blocker report below.
- [~] **4e-B: Locals-reassigned + ref-access + setState bail-outs (5 fixtures)** — tighten existing validators (`validate_no_ref_access_in_render`, `validate_locals_not_reassigned_after_render`, setState checks, hooks-in-loop) to catch these specific patterns. **Progress:** +4 fixtures (hooks-in-for-loop via Terminal::Branch handling in `validate_hooks_usage.rs`; ref-access detection for `error.validate-mutate-ref-arg-in-render.js` via name-based + Type::Ref fallback in `validate_no_ref_access_in_render.rs`; `error.invalid-setState-in-useMemo-indirect-useCallback.js` via indirect setState detection through useCallback within useMemo in `validate_no_set_state_in_render.rs`; `error.invalid-pass-ref-to-function.js` via CallExpression arg check in Stage 2g). Remaining potential gain: +1.
- [~] **4e-C: Frozen-mutation remaining (2 fixtures, was 3)** — overlaps Stage 4d remaining. 1 fixed (effect callback Check 4b, 2026-03-26). Remaining: `error.invalid-jsx-captures-context-variable.js` (JSX capture analysis) + 1 other. Potential gain: +2.
- [~] **4e-D: Todo-bail fixtures (10 fixtures) — PARTIALLY COMPLETE (+3, 450->453).** Fixed 3 of 10: `repro-declaration-for-all-identifiers.js` (for-in-try detection via Terminal::For), `repro-for-loop-in-try.js` (same), `repro-nested-try-catch-in-usememo.js` (file-level bail propagation via ANY_FUNCTION_BAILED thread-local). **7 remaining:** `optional-call-chain-in-ternary.ts`, `todo-optional-call-chain-in-optional.ts`, `propagate-scope-deps-hir-fork/todo-optional-call-chain-in-optional.ts`, `error.dont-hoist-inline-reference.js`, and ~3 others. See new gap notes below.
- [~] **4e-D2: Preserve-memo gaps (11 fixtures)** — overlaps Stage 4b. `finish_in_scope` issue resolved; validateInferredDep partially implemented (+3 passing). Remaining fixtures BLOCKED by scope dep resolution (SSA temp IdentifierIds don't resolve to named variables). Potential gain: +8 remaining but requires scope dep resolution fix.
- [ ] **4e-E: Todo remaining (3 fixtures, was 7, 3 fixed in 4e-D, 1 fixed in Stage 2g)** — overlaps Stage 4c remaining. Need optional-chain-in-ternary (2), context var update (1). Potential gain: +3 but requires new infrastructure. Context var update BLOCKED by nested HIR LoadContext gap. Optional-chain-in-ternary needs new validation pattern (see gap note below). `error.dont-hoist-inline-reference.js` fixed in Stage 2g via self-referencing const detection.

**Stage 4e-A done: +7 fixtures gained.**
**Stage 4e-B progress: +4 fixtures gained** (hooks-in-loop, mutate-ref-arg, setState-in-useMemo-indirect, ref-to-function). Latest: `error.invalid-pass-ref-to-function.js` (+1, Stage 2g) from ref passed as argument to non-hook function detection.
**Stage 4e-D partial: +3 fixtures gained (450->453).** Fixed via Terminal::For detection + file-level bail propagation.
**Stage 4e new (validation fixes session, 495->499): +4 fixtures gained.**
1. Fixed hooks-in-for-loop detection: `find_conditional_blocks` in `validate_hooks_usage.rs` now handles `Terminal::Branch` (for-loop continue/break targets), which was previously unmatched, causing the validator to miss hook calls inside for-loops.
2. Fixed ref-access detection for `error.validate-mutate-ref-arg-in-render.js`: `validate_no_ref_access_in_render.rs` now uses name-based fallback (`is_ref_name` / `ref_names` set) and `Type::Ref` checks on PropertyLoad/PropertyStore objects, in addition to ID-based tracking. This handles cases where inline_load_local_temps (Pass 9.6) eliminates LoadLocal instructions, causing ref IDs to not propagate. Also tracks source place IDs in LoadLocal (not just lvalue IDs).
3. Fixed nested setState-in-useMemo indirect detection: `validate_no_set_state_in_render.rs` now detects setState calls through useCallback closures within useMemo. Gained `error.invalid-setState-in-useMemo-indirect-useCallback.js`.
4. Added MethodCall invariant bail-outs: `validate_no_unsupported_nodes.rs` now bails on MethodCall patterns that upstream flags as invariant violations. Gained `error.bug-invariant-codegen-methodcall.js` and `error.todo-nested-method-calls-lower-property-load-into-temporary.js`.
5. Added destructuring assignment bail-out: `build.rs` now bails on complex destructuring assignment patterns in call arguments. Gained `error.call-args-destructuring-asignment-complex.js`.

**Stage 2g error fixture sweep (499->505): +6 fixtures gained.**
1. `check_fbt_duplicate_tags` in `validate_no_unsupported_nodes.rs`: Two-pass analysis — collects fbt/fbs identifiers (LoadLocal/LoadContext/LoadGlobal), counts `_enum`/`_plural`/`_pronoun` MethodCall sub-tags. Fixtures: `fbt/error.todo-fbt-unknown-enum-value.js`, `fbt/error.todo-multiple-fbt-plural.tsx`.
2. Ref-to-function detection in `validate_no_ref_access_in_render.rs`: CallExpression arg check — ref identifier passed to non-hook function. Fixture: `error.invalid-pass-ref-to-function.js`.
3. `check_self_referencing_declarations` in `validate_no_unsupported_nodes.rs`: Detects `const x = identity(x)` (LoadLocal before StoreLocal with same IdentifierId). Fixture: `error.dont-hoist-inline-reference.js`.
4. `is_valid_js_identifier` in `program.rs`: Validates dynamic gating directive conditions against JS identifier rules. Fixtures: `gating/dynamic-gating-invalid-identifier-nopanic.js`, `gating/error.dynamic-gating-invalid-identifier.js`.
**Remaining tractable gain (4e-B): +1 fixture, no new infrastructure.**
**Remaining ref-access fixtures:** `error.invalid-pass-ref-to-function.js` FIXED in Stage 2g (+1) — ref passed as argument to non-hook function now detected via CallExpression arg check. The other ref-access false-positive bail-outs (Stage 2e, 8 fixtures) are a separate category where we incorrectly bail on valid code.
**Full potential (all remaining sub-tasks): +16 fixtures** (was +22, -6 fixed in Stage 2g sweep).
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

#### Gap: Hoisting Inline Reference Detection (1 fixture) -- COMPLETE (Stage 2g, +1)

~~**Fixture:** `error.dont-hoist-inline-reference.js`~~

~~**Current state:** Not investigated. Upstream bails with an error related to hoisting inline references.~~

**Completed (2026-03-26):** Investigation revealed this is actually a self-referencing const declaration pattern (`const x = identity(x)`), not a hoisting issue. Fixed via `check_self_referencing_declarations` in `validate_no_unsupported_nodes.rs`. The implementation detects `LoadLocal` referencing the same `IdentifierId` before the matching `StoreLocal` within a `Const` declaration block. Upstream's EnterSSA pass catches this as "identifier used before defined." **Note:** Only handles `Const` kind, not `Let` — potential future TDZ fixtures may need `Let` support.

#### Cross-Cutting Fix: File-Level Bail Propagation (ANY_FUNCTION_BAILED)

**Implemented in 4e-D (2026-03-26).** When ANY function in a file bails during compilation (e.g., due to a try-catch Todo error), the bail-out now propagates to the file level via an `ANY_FUNCTION_BAILED` thread-local flag. This ensures that fixtures where upstream bails the entire file (because one function has an unsupported pattern) are correctly handled by our compiler. Previously, we would bail on the individual function but still emit a transformed result for the file, causing the conformance test to see `transformed = true` when it should be `false`.

**Impact:** This is a cross-cutting improvement that affects ALL fixtures in the "we compile, upstream bails" category where the upstream bail comes from a nested function within the file. The 3 fixtures gained in 4e-D (+3, 450->453) were directly enabled by this propagation mechanism.

#### Stage 4f: "Found 1 error" Bail-Out Sweep (29 fixtures, 14 done, 15 remaining, target: +15 more)

**Updated 2026-04-03.** Fresh analysis of the 108 WE-COMPILE-THEY-DON'T fixtures revealed 29 with `// UPSTREAM ERROR: Found 1 error:` headers. For all 29, bailing (not transforming) = passing. Zero regression risk. **14 of 29 completed this session (Groups A + C).**

**Group A: Try-catch value blocks + for-loops (10 fixtures) -- COMPLETE (+10, 526->536)**

~~Detect optional chains, ternaries, logical expressions, and for-in/for-of loops inside try-catch blocks.~~

**Completed (2026-04-03):** Implemented AST-level `check_try_catch_value_blocks` in `program.rs` (pre-lowering bail). Walks try block statements to detect: (a) optional chains (`?.`, `?.()`), (b) ternary/conditional expressions, (c) logical expressions (`&&`, `||`, `??`), (d) for-in/for-of loops. Fires `ANY_FUNCTION_BAILED` to propagate file-level bail. All 10 fixtures now bail correctly, matching upstream's "Todo: Support value blocks in try/catch" behavior.

**Fixtures gained (10, removed from KF):**
- `try-catch-optional-chaining.js`, `try-catch-optional-call.js`, `try-catch-nullish-coalescing.js`
- `try-catch-nested-optional-chaining.js`, `try-catch-multiple-value-blocks.js`, `try-catch-logical-and-optional.js`
- `repro-for-of-in-try.js`, `repro-for-in-in-try.js`, `repro-declaration-for-all-identifiers.js`, `repro-for-loop-in-try.js`

**File:** `crates/oxc_react_compiler/src/entrypoint/program.rs` (`check_try_catch_value_blocks`)

**Group B: Optional terminal in non-optional context (3 fixtures)**
- `optional-call-chain-in-ternary.ts` -- "Todo: Unexpected terminal kind `optional` for ternary test block"
- `todo-optional-call-chain-in-optional.ts` -- "Todo: Unexpected terminal kind `optional` for optional fallthrough block"
- `propagate-scope-deps-hir-fork/todo-optional-call-chain-in-optional.ts` -- same as above
- **Implementation:** Detect nested optional chains (`?.()`) inside ternary test expressions or optional fallthrough paths. See existing Gap note at line ~686.

**Group C: Object expression computed keys (4 fixtures) -- COMPLETE (+4, 536->540)**

~~Bail when ObjectExpression has computed keys that are CallExpression or SequenceExpression.~~

**Completed (2026-04-03):** Implemented AST-level `check_object_expression_computed_keys` in `program.rs` (pre-lowering bail). Walks ObjectExpression properties and bails when a computed key is a non-Identifier expression (CallExpression, SequenceExpression, MemberExpression, etc.). Fires `ANY_FUNCTION_BAILED` to propagate file-level bail. Matches upstream's "Todo: Expected Identifier, got X key" behavior.

**Fixtures gained (4, removed from KF):**
- `object-expression-computed-key-modified-during-after-construction.js`
- `object-expression-computed-key-mutate-key-while-constructing-object.js`
- `object-expression-member-expr-call.js`
- `object-expression-computed-key-modified-during-after-construction-sequence-expr.js`

**File:** `crates/oxc_react_compiler/src/entrypoint/program.rs` (`check_object_expression_computed_keys`)

**Group D: Frozen mutation / value modification (3 fixtures)**
- `error.invalid-jsx-captures-context-variable.js` -- "Error: This value cannot be modified"
- `error.todo-for-loop-with-context-variable-iterator.js` -- "Error: This value cannot be modified"
- `new-mutability/error.mutate-frozen-value.js` -- "Error: This value cannot be modified"
- **Implementation:** Improve frozen-mutation detection for context variables and JSX capture patterns.

**Group E: Post-render mutation (2 fixtures)**
- `error.invalid-pass-mutable-function-as-prop.js` -- "Error: Cannot modify local variables after render completes"
- `error.invalid-return-mutable-function-from-hook.js` -- "Error: Cannot modify local variables after render completes"
- **Implementation:** Tighten `validateLocalsNotReassignedAfterRender` for mutable function prop/return patterns.

**Group F: Context/hoisting (4 fixtures)**
- `error.todo-functiondecl-hoisting.tsx` -- "Todo: PruneHoistedContexts"
- `error.todo-valid-functiondecl-hoisting.tsx` -- "Todo: PruneHoistedContexts"
- `error.todo-handle-update-context-identifiers.js` -- "Todo: Handle UpdateExpression" (BLOCKED by nested HIR LoadContext gap)
- `error.bug-invariant-unnamed-temporary.js` -- "Invariant: Expected temporaries promoted"
- **Implementation:** Function declaration hoisting bail (2 fixtures) is potentially simple. Context variable update (1) is BLOCKED. Unnamed temporary invariant (1) may need investigation.

**Group G: Preserve-memo (2 fixtures)**
- `error.repro-preserve-memoization-inner-destructured-value-mistaken-as-dependency-later-mutation.js`
- `error.repro-preserve-memoization-inner-destructured-value-mistaken-as-dependency-mutated-dep.js`
- **Implementation:** These need the preserve-memo validation to detect the specific destructured dependency pattern. Overlaps Stage 4b/2i.

**Group H: Hoisting access-before-declare (1 fixture)**
- `error.invalid-hoisting-setstate.js` -- "Error: Cannot access variable before it is declared"
- **Implementation:** Detect setState calls to variables declared later in the scope.

**Recommended execution order (remaining):** B (3) -> F (2-3 tractable) -> E (2) -> D (3) -> H (1) -> G (2, if unblocked). Groups A and C COMPLETE.

---

### Stage 5: "Both No Memo" — DCE + Constant Propagation (target: +30-50 fixtures)

**Pool:** ~85 fixtures where neither side memoizes but output differs. (Was 79 pre-Stage 5a; some fixtures shifted categories after DCE changes.)
**Risk:** HIGH — requires implementing new compiler passes (DCE, constant propagation).

**Investigation finding (2026-03-25, updated 2026-03-26):** These are NOT cosmetic format diffs as originally assumed. Upstream runs dead-code elimination and constant propagation passes that simplify the output. **Stage 5a (2026-03-26) partially addressed this:** extended DCE removes dead StoreLocal/PrefixUpdate/PostfixUpdate, phi-node CP folds constant phis. +7 fixtures gained. **Stage 5b (2026-03-25) added dead branch elimination:** If/Branch/Ternary/Optional terminals handled, but gained +0 because branch conditions are rarely constant at Pass 32.5. The remaining ~85 "both no memo" fixtures are **blocked by 0-slot codegen** (our compiler emitting `_c(0)` or equivalent wrapper structure around code that upstream emits as passthrough), NOT by DCE/CP gaps. Further DCE/CP work has diminishing returns on this pool.

#### Stage 5a: Dead Code Elimination Pass — PARTIALLY COMPLETE (+7, 457->464)

Completed 2026-03-26. Extended the existing DCE pass with three key improvements:

1. **Dead StoreLocal/PrefixUpdate/PostfixUpdate removal:** Added `collect_read_identifiers` function that walks all instructions and terminals to collect every identifier appearing in a read (operand) position, explicitly excluding write targets (lvalue of StoreLocal/DeclareLocal). A StoreLocal is pruned if its stored value (the `value` operand, not the lvalue) is not in the read set. PrefixUpdate/PostfixUpdate are pruned if their place is not in the read set.
2. **Placement invariant:** Extended DCE placed at Pass 32.5 — AFTER all validators (Pass 21-32) have run. Pre-validation DCE (Pass 10/18) must NOT remove StoreLocal/DeclareLocal because validators depend on them. Post-validation DCE can be aggressive.
3. **Iterative CP+DCE loop:** The extended DCE and constant propagation run in alternation at Pass 32.5 until a fixed point (no further changes), ensuring constants exposed by DCE enable further propagation and vice versa.

**Fixtures gained (7 removed from known-failures.txt):**
- `call.js`
- `capturing-func-mutate-2.js`
- `capturing-func-mutate-3.js`
- `capturing-nested-member-call.js`
- `constructor.js`
- `invalid-jsx-lowercase-localvar.jsx`
- `ssa-call-jsx.js`

**What is NOT yet implemented:**
- [ ] Dead branch elimination (removing unreachable if/else branches when condition is constant) — this is the next DCE frontier
- [ ] More aggressive constant folding (binary operators, string concatenation, etc.)
- [ ] Upstream reference: `compiler/packages/babel-plugin-react-compiler/src/Optimization/DeadCodeElimination.ts`
- [ ] **Remaining pool:** ~76 "both no memo" fixtures still need further DCE/CP improvements
- **Files:** `dead_code_elimination.rs`, `constant_propagation.rs`, `pipeline.rs`

#### Stage 5b: Dead Branch Elimination + Constant Propagation — COMPLETE (+0 net, infrastructure correct)

**Implemented 2026-03-25.** Infrastructure correct, 0 net conformance gain. Branch conditions are rarely constant at Pass 32.5. If/Branch/Ternary/Optional terminals handled.

**Dead branch elimination implemented:** When a terminal's condition is a known constant (from constant propagation), the dead branch is eliminated entirely. Handles `If`, `Branch`, `Ternary`, and `Optional` terminals. The live branch's block replaces the conditional, and the dead branch becomes unreachable (removed by subsequent DCE).

**Phi-node constant propagation implemented (2026-03-26):** After the main forward propagation loop, the pass now inspects every phi node operand. If all operands resolve to the same constant value, the phi output is replaced with that constant for all downstream uses. This enables DCE to remove the dead branches that fed the phi.

**Why 0 net conformance gain:** Branch conditions are rarely constant after validation passes (Pass 32.5). The constant propagation pass can fold phi nodes with identical constant operands, but most branch conditions in real fixtures depend on runtime values (props, state, hook returns). The infrastructure is correct and will fire when constants are available, but the supply of constant conditions at Pass 32.5 is very limited.

**What is NOT yet implemented:**
- [ ] Binary operator folding (e.g., `1 + 2` -> `3`)
- [ ] String concatenation folding
- [ ] Upstream reference: `compiler/packages/babel-plugin-react-compiler/src/Optimization/ConstantPropagation.ts`
- **File:** `constant_propagation.rs`

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
| Stage 2d: Frozen-mutation false positives | +5-8 | 416-419 | **BLOCKED** | 26 fixtures. Root cause: transitive freeze propagation in aliasing pass. See blocker report. |
| Stage 2e: Ref-access false positives | +0 (no impact) | -- | LOW | 8 fixtures, freed land in slots-DIFFER. Deprioritized. |
| Stage 2f: Reassignment false positives | +5-7 | 424-431 | MEDIUM | 10 fixtures |
| Stage 2g: Other bail-outs | +6 (done, partial) | 505 | MIXED | +6 from error fixture sweep (499->505). ~34 remaining fixtures. |
| Stage 1d Phase 1: Declaration placement | +6 (done) | 450 | LOW | Completed. Phase 2/3 remain (+10-30). |
| B2: Variable name preservation | +20-30 | 464-501 | MEDIUM | 40 fixtures, scope output naming. **Finding (2026-03-26): scope-inference dependent, NOT codegen-only.** Many B2 fixtures also have scope boundary differences; pure codegen name changes won't pass them. |
| Stage 3: Scope inference (±1/±2 diffs) | +50-100 | 514-601 | HIGH | 688 pool (402 deficit, 286 surplus), scope MERGING is bottleneck (see 3b blocker) |
| Stage 4b: Preserve-memo validation | +3 done, +12-22 remaining | 544-646 | MEDIUM | 32 fixtures (revised down from 60; 3 sub-types: validateInferredDep (3/32 done, 29 BLOCKED by scope dep resolution), value-memoized, dep-mutated) |
| Stage 4c: Todo error detection | +15 (done, 5 remain) | 426 | LOW | 22/27 done (15 in 4c + 7 in 4e-A). Remaining 5 need hoisting, optional terminals, context vars. |
| Stage 4d: Frozen-mutation false negatives | +10 (done) | 435+1 | MEDIUM | Completed. 7/9 planned + 2 bonus + 1 follow-up (Check 4b). 1 remains (JSX capture). |
| Stage 4e-A: Upstream error bail-outs | +7 (done) | 442 | LOW | 7/43 done. 4e-B through 4e-E remain. |
| Stage 4e-B: Locals/ref/setState/hooks | +3 so far | 444 (pre-1d) + 1 (495->496) | LOW | 3/5 done (hooks-in-loop, mutate-ref-arg, setState-in-useMemo-indirect). 2 remain. |
| Stage 1e: Misc codegen/harness | +6 (done) | 447 (from 441) | LOW | Completed. Gating parsing +3, empty catch +1, computed key +2, const/let +0. |
| Stage 4e-D: Todo-bail (partial) | +3 (done) | 453 | LOW | 3/10 done (for-in-try, bail propagation). 7 remain (optional-chain, hoisting). |
| Stage 4e validation fixes (495->499) | +4 (done) | 499 | LOW | MethodCall invariant +2, destructuring assignment +1, setState-in-useMemo indirect +1. |
| Stage 2g error fixture sweep (499->505) | +6 (done) | 505 | LOW | fbt duplicate tags +2, ref-to-function +1, self-referencing const +1, dynamic gating invalid identifier +2. |
| Stage 2j: Tighten Infer mode heuristics | +4 (done) | 496 (post-reconciliation) | LOW | Completed. body_has_hooks_or_jsx added. 4/7 infer fixtures gained; 3 remain (need directive support). |
| Stage 4e-C/D2/E: Remaining upstream errors | +8-25 (was +14-31, -6 done in Stage 2g) | 513-530 | MED-HIGH | 4e-C (2, MED), 4e-D2 preserve-memo (8, MED-HIGH, BLOCKED), 4e-E (2, HIGH) |
| Stage 5a: DCE + phi-node CP | +7 (done) | 464 | MEDIUM | Completed. 7 fixtures from dead StoreLocal/Prefix/Postfix removal + phi CP. |
| Stage 5b: Dead branch elimination | +0 (done) | 464 | MEDIUM | Completed. Infrastructure correct, 0 net gain. Branch conditions rarely constant at Pass 32.5. |
| Stage 5 remaining: Binary/string folding | +5-15 | 510-520 | MEDIUM-HIGH | ~85 "both no memo" remain. **Blocked by 0-slot codegen (scope inference), not DCE/CP.** 0-slot codegen attempted in Stage 2g and REJECTED (-52 regression). Binary/string folding has diminishing returns. |
| Stage 3a2: Zero-slot surplus investigation | +1 (done) | 457 | BLOCKED | Completed +1 (escape-analysis-not-if-test.js). Investigation confirmed 134 surplus fixtures require fundamental scope inference changes (mutable range accuracy, scope grouping), not pruning. BLOCKED by Stage 3b prerequisites. See blocker report. |
| Stage 1g: Gating directive stripping | +2 (done) | 507 (pre-reconciliation) | LOW | Completed. Gating comment filtering +2, hook-as-value false positive fix +0, bail-out diagnostics +0. |
| KF reconciliation (2026-04-03) | -11 (reconciliation) | 496 | N/A | 38 pre-existing divergences added to KF, 24 newly-passing removed. Reconciles phantom 507 to actual 496. |
| Preserve-memo default enable (2026-04-03) | +30 (done) | 526 | LOW | Enabled `validatePreserveExistingMemoizationGuarantees` default=true in test harness. Massive free win. |
| Stage 4f-A: Try-catch value block bail (2026-04-03) | +10 (done) | 536 | LOW | AST-level `check_try_catch_value_blocks` in `program.rs`. 10 fixtures bail correctly. |
| Stage 4f-C: Object expression computed key bail (2026-04-03) | +4 (done) | 540 | LOW | AST-level `check_object_expression_computed_keys` in `program.rs`. 4 fixtures bail correctly. |
| Stage 2i: Temp-name-skipping attempt (2026-04-03) | -56 (reverted) | 540 | BLOCKED | 4th failed approach to preserve-memo false-positive bails. Definitively blocked. |
| Stage 4f remaining (B/D/E/F/H) | +10-15 remaining | 550-555 | LOW | 15 "Found 1 error" fixtures still actionable. Zero regression risk. |
| **Total remaining** | **+60-200** | **600-740** | | From 540 base |

**Key learning from Stage 1b:** Temp renumbering alone is nearly worthless (+2). Naming and ordering are entangled — fixing one without the other does not pass conformance.

**Key learning from Stage 2a/2b:** Most bail-outs come from specific validations, not silent/0-scope issues. File-level bail-outs were low-hanging fruit (+1 net from removing 4).

**Key correction (2026-03-26):** The 88 error.* figure was pre-Stage-4c/4d. After Stage 4e-D partial + freeze follow-up + validateInferredDep partial + validation fixes session + Stage 2g sweep, **12 error.* fixtures remain in known-failures** (10 top-level + 0 fbt/ error.*). Down from 18 pre-Stage-2g, 22 pre-validation-fixes, 30 pre-preserve-memo, 43 pre-4e-A, 37 pre-4e-D, 34 pre-freeze-follow-up, 33 pre-validateInferredDep.

**Important: CompilationMode::All in conformance tests.** The conformance test harness (`tests/conformance_tests.rs`) uses `compilationMode:"all"`, meaning ALL functions in a fixture are compiled (not just those detected as components/hooks). This affects which fixtures pass/fail because validations run on every function body, not just component-shaped ones. When investigating fixture behavior, always account for this mode.

**Key learning from Stage 2c:** Fixing bail-outs does not directly increase conformance if the newly-compiling fixtures land in slots-DIFFER/MATCH pools. Bail-out fixes unblock fixtures for FUTURE scope/codegen improvements but yield +0 net on their own.

**Key learning from Stage 1d Phase 1:** Lazy declaration placement gained +6 (exceeded +5 estimate). Confirms that declaration ordering is a tractable codegen fix. Phases 2-3 (control-flow-scoped declarations, merged init) remain and target the larger A1 pool (39 fixtures).

**Key learning from Stage 2e investigation:** Not all bail-out fixes improve conformance. Ref-access false positives free 8 fixtures that land in slots-DIFFER, not matched. Additionally, 2 accidental Flow parse error matches would be lost. Always check where freed fixtures land before pursuing bail-out removal.

**Key learning from extended investigation (2026-03-25):**
- "Both no memo" DCE/CP/branch-elimination ceiling reached — Stages 5a+5b gained +7 total. ~85 remain, blocked by 0-slot codegen (scope inference), not DCE/CP
- "We compile, they don't" has 75 UPSTREAM ERROR fixtures — significant untapped pool if we match error formats
- Slots-MATCH B2 pattern (40 fixtures) is the single largest tractable codegen fix remaining
- `validatePreserveExistingMemoizationGuarantees` gaps account for 32 of the "we compile, they don't" fixtures (3 now fixed via validateInferredDep, 29 BLOCKED by scope dep resolution)

**Revised path to 600 (updated 2026-04-03):** Reachable via scope inference fixes (Stage 3, +50-100) + validation gaps (Stage 4, +20-63 remaining) + codegen fixes (B2, +10-20; 1d Phase 3 done +0 dormant) + remaining DCE/CP (Stage 5, +5-15 revised down). Stage 2j (Infer heuristics) now COMPLETE (+4). Note: 1d Phase 2 is BLOCKED by scope inference (see finding #25). B2 also found to be scope-inference dependent (see finding #29). Stage 4b validateInferredDep remaining 29 fixtures BLOCKED by scope dep resolution (see blocker report). **Stage 3a2 investigation (2026-03-26) CONFIRMED: 134 zero-slot surplus fixtures require fundamental scope inference changes, not pruning.** Stage 5a+5b DCE+CP+branch-elimination gained +7 total (457->464). "Both no memo" pool blocked by 0-slot codegen (scope inference), not DCE/CP. **0-slot codegen attempted and REJECTED in Stage 2g: -52 regression.** Conservative floor: ~600 from 496 base. Optimistic: 700+. **KF reconciliation (2026-04-03):** 38 pre-existing divergences added, 24 newly passing removed. Non-scope-inference work nearly exhausted.

**Key learning from Stage 3b investigation (2026-03-25, extended experiments added):** The slot-diff deficit (402 fixtures) has diverse root causes (over-merging, missing outputs, wrong boundaries). Three separate experiments confirmed the blocker report: (1) removing `last_use > instr_id` gate: -5 net, improves deficit by 4 but causes 12 regressions from over-merging; (2) inclusive gate (`>=`): no-op because `last_use` never equals `instr_id`; (3) removing `check_nested_method_call_as_argument`: -2 net, our codegen genuinely cannot handle nested method calls yet. The `last_use > instr_id` heuristic is load-bearing for scope merging correctness. Future scope inference work must target the merging algorithm (`merge_overlapping_reactive_scopes_hir`), not sentinel creation or bail removal. Additionally, 8 silent-bail fixtures require IIFE scope creation, React.memo function inference, and Flow type cast handling -- all scope inference improvements.

**Key learning from "we compile, they don't" re-analysis (2026-03-25):** The 189 fixtures break down as 60 preserve-memo (largest actionable sub-pool), 15 flow-parse (not actionable), 10 todo-bail, 6 invariant, 4 frozen-value. The 60 preserve-memo further split into 3 sub-types, making Stage 4b more tractable than previously thought (clear attack plan per sub-type).

**Key principle:** Each stage starts with investigation (sub-task "a") that produces a fixture-level breakdown. If the investigation shows estimates are wrong, the plan is updated before implementation begins. No blind implementation.

**Key learning from Stage 2g (2026-03-26):**
- **0-slot codegen is NOT viable.** Attempted emitting passthrough code (no `_c()` wrapper) for functions with 0 cache slots. Caused **-52 regression** (505->453). Many 0-slot fixtures have expected output with structural transformations that differ from passthrough. 0-slot codegen must wait for scope inference accuracy to reduce surplus scopes to near-zero for these fixtures.
- **Self-referencing check only handles `Const`, not `Let`.** The `check_self_referencing_declarations` function only fires for `InstructionKind::Const` declarations. `Let` declarations also have TDZ semantics in JavaScript, but no current conformance fixtures test `let x = f(x)`. If future fixtures appear, extend the check to cover `Let` kind.
- **`import fbt from 'fbt'` creates `LoadLocal`, not `LoadGlobal`.** Because `fbt` is not in the built-in globals list (`GlobalCollector`), an `import fbt` statement is lowered to a local binding. The `check_fbt_duplicate_tags` function handles this by checking both `LoadLocal` and `LoadGlobal`, but any future fbt-related work must be aware of this distinction. If the globals list is ever extended to include `fbt`, the LoadLocal path would no longer fire.

**Key learning from post-3b analysis (2026-03-26, updated 2026-04-03):**
- **Infer mode heuristics fixed (Stage 2j COMPLETE).** `body_has_hooks_or_jsx` added to `program.rs` — shallow AST walk that skips functions without hooks/JSX in Infer mode. +4 fixtures gained. 3 of original 7 remain (need directive support for gating/enableAllowSetStateFromRefsInEffects).
- **Nearly ALL remaining conformance gains require scope inference.** After exhaustive analysis of all 1210 diverged fixtures: slots-DIFFER (615) = scope inference, slots-MATCH (236) = scope inference + naming, both-no-memo (92) = 0-slot codegen (scope inference), we-compile-they-don't (140) = 123 scope surplus + 17 error.*, we-bail-they-compile (127) = 52 preserve-memo BLOCKED + 15 frozen BLOCKED + rest various. The only non-scope-inference work remaining is individual error.* bail-outs (+1-2 each) and 3 remaining Infer mode fixtures (directive support).
- **KF reconciliation (2026-04-03).** 38 pre-existing divergences from prior uncommitted work added to KF. Categories: fbt (5), gating (7), error-handling (8), exhaustive-deps (6), other (12). 24 newly-passing fixtures removed. Net: 507->496.

**Key learning from KF reconciliation (2026-04-03):**
- **Uncommitted work must be reconciled into KF before claiming conformance numbers.** Prior sessions accumulated code changes without updating KF, creating a phantom 507 figure. Actual passing count after reconciliation: 496. The 38 newly-added KF entries break down by category: fbt/ (5 entries: fbs-params, fbt-preserve-whitespace-subtree, fbt-unknown-enum-value, fbt-plural, recursively-merge-scopes-jsx), gating/ (7 entries), error.* (8 entries: bailout-on-suppression, known-incompatible x3, reassign-variable, unconditional-set-state, todo-missing-source, validate-blocklisted-imports), exhaustive-deps/ (6 entries: effect-events, dep-on-ref, disallow-unused-stable, extra-only, full exhaustive, missing-nonreactive), other (12 entries: component-declaration-basic.flow, various).
- **Always run full conformance and reconcile KF before and after each session.** Stale KF entries create false signals about which work items will gain fixtures.

**Key learning from Stage 1g (2026-03-26):**
- **Gating directive comments must be stripped from output.** Upstream's Babel plugin removes `@gating`/`@dynamicGating` directives during compilation. Our source-edit-based approach was preserving them, causing 2 gating fixture mismatches. Simple line-level filtering in `apply_compilation` suffices.
- **Recursive ref check removal is catastrophic (-9).** The `validate_no_ref_access_in_render` recursive check is load-bearing for a significant number of fixtures. Do not attempt to remove or relax it without a very targeted replacement.
- **Hook-as-value locally_declared_names prevents future false bails.** While the 3 currently affected fixtures are also caught by a separate PropertyLoad check (so net-zero today), the `locally_declared_names` fix ensures that as the PropertyLoad check is refined in the future, these fixtures won't regress into false bails. This is a correctness guard, not a conformance gain.
- **Per-fixture bail-out name tracking is invaluable for diagnostics.** Knowing exactly which fixtures contribute to each bail-out category dramatically speeds up investigation. The `bail_fixture_names` HashMap in `conformance_tests.rs` maps error keys to fixture paths and prints up to 8 per category.

---

## Deferred / Blocked Work

### Phase 2 Remaining: Impure Function Handling — DEFERRED

**Files:** `src/inference/infer_mutation_aliasing_effects.rs`, `src/validation/`
- Impure function handling in legacy signatures — requires `validate_no_impure_functions_in_render` integration

### Scope Dep Resolution (SSA temp -> named variable mapping) — BLOCKED

**Affects:** Stage 4b validateInferredDep (29 remaining fixtures), potentially B2 variable name preservation, and any feature needing to resolve scope deps back to original source-level names.

**Problem:** After SSA, scope dependency IdentifierIds point to temporaries, not original named variables. No reverse mapping exists from SSA temp IdentifierIds back to the property access path (e.g., `props.x`) that produced them. `propagate_dependencies.rs` does not preserve this path information.

**Resolution options:**
1. Enhance `propagate_dependencies.rs` to carry the original dependency path (property access chain) alongside the IdentifierId
2. Build a post-SSA reverse mapping pass that traces each temp back through LoadLocal/PropertyLoad chains to the original source variable
3. Port upstream's richer `ReactiveScopeDependency` type which includes the full access path

**See:** Stage 4b blocker report in this file for full details.

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

### 0-Slot Codegen (passthrough for functions with no cache slots) — BLOCKED

**Attempted (2026-03-26):** Emitting passthrough code (no `_c()` wrapper, no memoization structure) when the compiler produces 0 reactive scopes / 0 cache slots. **Result: -52 regression (505->453).** Root cause: many fixtures have 0 expected slots but the expected output is NOT a simple passthrough — upstream still structurally transforms the code (e.g., extracts arrow functions, renames variables) even when it produces 0 slots. Our passthrough emits the original source code, which doesn't match the structurally transformed expected output.

**Do NOT attempt again until:** Scope inference accuracy is improved to the point where our compiler produces 0 scopes on the same set of fixtures where upstream produces 0 scopes. Currently we produce surplus scopes on ~134 fixtures where upstream produces 0. Only when the surplus is reduced to near-zero would 0-slot passthrough become viable without regression.

### Self-Referencing Declarations: `Let` Kind TDZ — DEFERRED (no fixtures)

**Current state:** `check_self_referencing_declarations` in `validate_no_unsupported_nodes.rs` only handles `Const` kind. `Let` declarations also have TDZ semantics (`let x = f(x)` is a runtime TDZ error), but no conformance fixtures currently test this pattern. If future fixtures appear, extend the check from `InstructionKind::Const` to also cover `InstructionKind::Let`.

### `import fbt` LoadLocal vs LoadGlobal — INFORMATIONAL

**Discovery (2026-03-26):** `import fbt from 'fbt'` is lowered to a local binding (`LoadLocal`), not `LoadGlobal`, because `fbt` is not in the built-in globals list in `GlobalCollector`. The `check_fbt_duplicate_tags` function handles both paths, but any future fbt work (e.g., fbt call detection, fbt parameter validation) must check both `LoadLocal` and `LoadGlobal` for the fbt identifier. If `fbt` is ever added to the globals list, the `LoadLocal` path would stop firing and only `LoadGlobal` would be needed.

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

### Scope dep IdentifierIds are SSA temps, not source names
After SSA, scope dependency IdentifierIds point to temporaries (e.g., `t1`), not original source variables (e.g., `props.x`). This blocks `validateInferredDep` (29 fixtures), B2 variable name preservation, and any feature needing to resolve scope deps to source-level names. See "Scope Dep Resolution" in Deferred/Blocked section.

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
| Dead code elimination | `src/optimization/dead_code_elimination.rs` |
| Constant propagation | `src/optimization/constant_propagation.rs` |
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
15. **"Both no memo" requires DCE + constant propagation — DCE/CP CEILING REACHED.** Originally assumed to be cosmetic format diffs. DCE, phi-node CP, and dead branch elimination passes all implemented (Stages 5a+5b, +7 fixtures total). Dead branch elimination gained +0 (branch conditions rarely constant at Pass 32.5). The key architectural discovery: pre-validation DCE (Pass 10/18) must NOT remove StoreLocal/DeclareLocal because validators at Pass 21-32 depend on them being present. Extended DCE placed at Pass 32.5 after all validators. **Updated finding:** Remaining ~85 "both no memo" fixtures are blocked by 0-slot codegen (scope inference creates scopes where upstream doesn't), NOT by DCE/CP gaps.
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
26. **"We compile, they don't" CORRECTED (2026-03-26).** 134 of 191 fixtures are scope inference SURPLUS (upstream compiles with 0 reactive scopes, not validation gaps). Previously mischaracterized as "no upstream error header -- not actionable." Upstream DID compile these fixtures but produced 0 cache slots. We produce >0 slots (over-memoization). This makes scope inference the dominant problem in this category. Only ~57 of the 191 are actual validation gaps (30 UPSTREAM ERROR, ~24 preserve-memo, ~3 other).
27. **Dynamic gating parsing was a test harness bug, not a compiler bug.** 3 fixtures gained by fixing conformance test directive parsing for `@gating` patterns. Always check whether a fixture failure is a harness issue before assuming it's a compiler bug.
28. **Nested HIR builders don't emit LoadContext instructions.** When a nested function is lowered by a child `HIRBuilder`, context variables (captured from outer scope) are represented as plain `LoadLocal` in the nested HIR, not `LoadContext`. This means walking the nested HIR cannot distinguish context variables from local variables. The upstream compiler uses `LoadContext` to identify captured variables in nested lambdas. Fixing `error.todo-handle-update-context-identifiers.js` requires either (a) emitting `LoadContext` in nested builders, or (b) passing parent scope binding information to the validation pass. This is a structural limitation, not a simple pattern-matching fix.
29. **B2 (variable name preservation) is scope-inference dependent, NOT codegen-only.** Investigation of the 40 B2 fixtures revealed that many also have scope boundary differences driven by scope inference. Changing which variable name is used for scope outputs (temp vs original) is a codegen change, but it does not pass conformance if the scope itself has different boundaries than upstream. B2 is therefore only partially addressable by codegen; the remainder requires scope inference improvements (Stage 3). This downgrades B2 from "largest tractable codegen fix" to "partially tractable, scope-dependent."
30. **Re-enabling removed bail-outs as per-function bails can gain fixtures.** The known-incompatible import bail and ESLint suppression bail were removed in Stage 2b as file-level bails. Re-enabling them as per-function bails (matching upstream behavior) gained +4 fixtures (+3 from incompatible imports, +1 from ESLint suppression). The lesson: removing a bail-out entirely is wrong if upstream still bails per-function. The fix is to change the granularity (file-level -> per-function), not remove the bail entirely.
31. **Object property key quoting matters for conformance.** Codegen must quote object property keys that are reserved words or contain special characters to match upstream output. A single property key formatting difference causes a fixture to fail even if the semantics are identical.
32. **Scope dep IdentifierIds don't match original variable names after SSA.** Scope dependencies captured by reactive scopes have IdentifierIds that correspond to SSA temporaries (e.g., `t1`, `t2`), not the original source-level named variables (e.g., `props.x`). This prevents `validateInferredDep` from matching scope deps against manual memo deps (which use original names). The same problem affects any feature that needs to resolve a scope dep back to its original variable identity. Root cause: `propagate_dependencies.rs` does not preserve the original dependency path through SSA. This is a cross-cutting blocker that affects validateInferredDep (29 fixtures), and potentially B2 variable name preservation and other scope-dep-dependent features.
33. **validateInferredDep partial success pattern.** Of 32 target error fixtures, only 3 pass because their deps happen to be simple named variables that survive SSA without temp indirection. The remaining 29 fail because their deps go through PropertyLoad chains that produce SSA temps. The algorithm itself is correct; the resolution layer is the blocker.
34. **134 "no error header" fixtures are SCOPE INFERENCE SURPLUS, not validation gaps (2026-03-26).** Previously described as "not actionable without identifying specific upstream validations." Investigation revealed upstream DID compile these fixtures -- it produced structurally transformed output with 0 reactive scopes. We produce >0 scopes (over-memoization). These are part of the 286-fixture surplus pool where our_slots > expected_slots, specifically the subset where expected_slots = 0. This makes scope inference surplus the DOMINANT conformance gap: 134 (zero-slot surplus) + 152 (non-zero surplus where our_slots > expected_slots) = 286 surplus fixtures total. Understanding why upstream produces 0 scopes on the 134 is the key investigation for large conformance gains.
35. **Pruning cannot fix scope inference merging problems (2026-03-26).** The `prune_non_escaping_scopes` enhancement (test-position detection) gained +1 fixture but the remaining 3 `escape-analysis-not-*` fixtures are BLOCKED because scope inference merges the array scope with the result scope. The pruning layer operates on reactive scopes as they exist after inference -- it cannot split a merged scope. The 134 zero-slot surplus fixtures are similarly dominated by scope inference issues (scopes spanning hook calls, over-merging), not missing prune logic. This means the scope inference merging algorithm (Stage 3b) is the gating factor for both the escape-analysis fixtures and the broader surplus pool.
36. **Divergence approach for test-position escape analysis works but has limits (2026-03-26).** Upstream uses `ValueKind::Primitive` / escape flags (a type-level system) to prune non-escaping scopes used only in test positions. Our DIVERGENCE uses set-based analysis (collect test-position IDs, subtract write targets, propagate aliases). This works for the simple case (`escape-analysis-not-if-test.js`) where the scope output is directly used as an if-test. It fails for cases where scope inference has already merged the scope with another, making the output identifier appear in non-test contexts. The divergence approach is correct but bounded by scope inference quality.
37. **Pre-validation DCE must not remove StoreLocal/DeclareLocal (2026-03-26).** Validators at Pass 21-32 depend on StoreLocal and DeclareLocal instructions being present to check for reassignment, mutation, and other patterns. The existing pre-validation DCE (Pass 10 and Pass 18) can safely remove truly unused instructions (where the lvalue is never referenced), but aggressive removal of StoreLocal/DeclareLocal breaks validation. The solution: place extended DCE at Pass 32.5, after all validators have run, where it can safely remove dead stores. This is a fundamental architectural invariant of the pipeline.
38. **Phi-node constant propagation enables cascading DCE (2026-03-26).** When all operands of a phi node resolve to the same constant value, the phi output can be replaced with that constant. This turns downstream conditional branches into known-constant conditions, which DCE can then eliminate. The iterative CP+DCE loop at Pass 32.5 runs until a fixed point, ensuring these cascading simplifications are fully exploited. Dead branch elimination (removing the unreachable branch entirely) is NOT yet implemented — the current pass only removes dead assignments, not dead control flow.
39. **"Both no memo" (~85 fixtures) blocked by 0-slot codegen, NOT DCE/CP (2026-03-26, updated 2026-03-25 session).** Despite implementing DCE + CP + dead branch elimination (Stages 5a+5b, +7 fixtures), ~85 "both no memo" fixtures remain. Dead branch elimination (Stage 5b) gained +0 because branch conditions are rarely constant at Pass 32.5. Investigation confirms the remaining fixtures are blocked by 0-slot codegen: our compiler wraps functions in `_c(0)` memoization structure where upstream emits passthrough. This is scope inference over-creation (same root cause as the 134 zero-slot surplus fixtures in Stage 3a2), not a DCE/CP gap. Further DCE/CP improvements (binary folding, string concat) have diminishing returns on this pool.
40. **Slots DIFFER (666 fixtures) and slots MATCH (243 fixtures) dominated by variable naming/scope inference (2026-03-25).** These two categories account for the vast majority of remaining failures (909 of ~1253). Both are fundamentally driven by scope inference accuracy — different scope boundaries produce different slot counts (DIFFER) and different declaration placement/variable naming (MATCH). Codegen-only fixes have reached their ceiling (Stage 1 exhausted). The path forward requires scope inference improvements (Stage 3), which carry high regression risk.
41. **Branch elimination gains limited by supply of constant conditions after validation (2026-03-25).** Dead branch elimination infrastructure is correct (handles If/Branch/Ternary/Optional terminals) but the constant propagation pass at Pass 32.5 produces very few constant branch conditions. Most branch conditions depend on runtime values (props, state, hook returns). The phi-node CP pass can fold identical-constant phis, but the cascading effect to branch conditions is minimal in practice. This makes dead branch elimination a correct-but-low-impact optimization for the current fixture set.
42. **Zero-slot surplus (134 fixtures) is a scope CREATION problem, not a pruning problem (2026-03-26).** Three approaches attempted and all failed: (a) per-function reactive guard caused -44 regression because most functions have reactive identifiers even when scopes are allocating-only; (b) `prune_unused_scopes` `is_allocating` guard removal gained +3 but caused unresolved reference bugs because `scope.declarations` is incomplete for destructuring patterns; (c) `prune_non_escaping_scopes` already correctly handles escaping allocating scopes. The root cause is `infer_reactive_scope_variables` creating sentinel scopes for allocating instructions that upstream does not create, because our `last_use_map` extension produces wider mutable ranges than upstream. This is the same root cause as Stage 3b (scope merging). No further pruning-based approaches should be attempted; the fix requires mutable range accuracy improvements.
43. **validateInferredDep false-positive count is 51, not 3 (CORRECTED 2026-03-26).** Conformance run shows 55 total "Existing memoization could not be preserved" false-positive bails. Pre-validateInferredDep baseline was 4. The +51 regression was far larger than the 3 originally documented (which was based on manual inspection of a small sample). Despite this, net conformance was still positive (+31) because the implementation correctly bails on many UPSTREAM ERROR fixtures. **Fixing scope dep resolution is the single highest-leverage task**: it would eliminate ~51 false-positive bails, potentially recover +10-30 conformance from freed fixtures matching upstream, AND unblock the remaining 29 UPSTREAM ERROR preserve-memo fixtures.
44. **Stage 2f (validateLocalsNotReassignedAfterRender relaxation) FAILED (2026-03-26).** Approach of relaxing the check caused 5 regressions vs 1 gain (-4 net). Root cause: the validator fires on patterns involving DeclareContext/StoreContext HIR instructions which we do not lower. Requires DeclareContext/StoreContext HIR lowering as a prerequisite. Do not attempt again until that infrastructure exists.
45. **Error.* fixtures are still a productive target at 18 remaining (2026-03-26).** The validation fixes session gained +4 (495->499) from 4 diverse error.* fixtures: nested setState detection, MethodCall invariant bail-outs, destructuring assignment bail-out. Each fix was small and self-contained. The remaining 18 error.* fixtures in KF are spread across preserve-memo (8, BLOCKED), todo patterns (3), invariant (1), frozen-mutation (2), reassignment (2), ref-access (3), validate-* (3). The non-BLOCKED ones (10 fixtures) remain tractable targets for incremental gains.
46. **Operand liveness in scope inference is coupled to mutable range accuracy (2026-03-26).** Changing one without the other causes cascading regressions (-17 to -24 net from operand-only changes). The operand liveness check uses `effective_range` (max of mutable_range.end and last_use+1) as a compensating mechanism for narrow mutable ranges. Switching to `mutable_range.end` alone causes over-splitting because our `infer_mutation_aliasing_ranges` produces narrower ranges than upstream (missing receiver mutation effects, reverse scope propagation). Both the blanket change (-24) and allocating-only change (-17) confirmed this coupling.
47. **Non-reactive dep pruning is too aggressive (-107 net) (2026-03-26).** Our `dep.reactive` flag doesn't match upstream semantics — many deps marked non-reactive are actually needed for correct codegen. Pruning them turns dep-based scopes into sentinel scopes (0 deps), which drastically changes codegen output structure. The reactive flag assignment in `propagate_dependencies.rs` needs a full audit against upstream's logic before any pruning is safe.
48. **0-slot function codegen via IR reconstruction is fundamentally wrong (-51 net) (2026-03-26).** Two attempts now (-52 in Stage 2g, -51 here). The current codegen reconstructs functions from IR, which produces different whitespace, ordering, and structure from source text. Any viable 0-slot codegen approach must surgically edit source text rather than reconstructing from IR. This is an architectural limitation, not a tuning problem.
