# oxc-react-compiler Backlog

> Last updated: 2026-03-25 (post Stage 2b partial)
> Conformance: **411/1717 (23.9%)**. Render: **92% (23/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Stage 2b partial: bail-outs reduced 108→89, +1 net conformance (410→411). Known-failures: 1306.

---

## Road to 600+ Conformance (403 → 600+, need +197)

### Failure Category Summary

| Category | Count | Description |
|----------|-------|-------------|
| Both compile, slots DIFFER | 668 (50.8%) | Scope inference accuracy — different cache slot counts |
| Both compile, slots MATCH | 239 (18.2%) | Same slots, codegen structure diffs (naming, ordering) |
| We compile, they don't | 223 (17.0%) | Missing validations — we should bail but don't |
| We bail, they compile | 108 (8.2%) | False-positive bail-outs — we reject valid code |
| Both no memo (format diff) | 76 (5.8%) | Neither side memoizes, cosmetic output diffs |

---

### Stage 1: Codegen Structure — "Slots MATCH" Fixes (target: +40-60 fixtures)

**Pool:** 239 fixtures where slot count matches upstream but codegen differs.
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

#### Stage 1d: Declaration Placement / Instruction Ordering (est: +15-30, HIGH risk)

- [ ] Redesign `collect_all_scope_declarations` to emit declarations at narrowest possible scope instead of function level
- [ ] Fix hook call ordering (hook calls before temp declarations, not after)
- [ ] This is the dominant remaining "slots MATCH" blocker — 55+ fixtures depend on instruction ordering, and many of the 126 "naming" fixtures also need this
- **Risk:** HIGH — `collect_all_scope_declarations` is load-bearing (removing it collapses render 96%→24%). Requires careful incremental approach.
- **Prerequisite:** Must understand exactly which declarations can be moved safely vs which must stay at function level

---

### Stage 2: False-Positive Bail-outs — "We Bail, They Compile" (target: +50-70 fixtures)

**Pool:** Originally 108 fixtures, now **89 remaining** after Stage 2b partial fixes.
**Risk:** MEDIUM — each bail-out removal must not introduce wrong output.

#### Stage 2a: Investigate Bail-out Categories -- COMPLETE

Completed 2026-03-25. Full results in [bail-out-investigation.md](bail-out-investigation.md).
Categorized all 108 bail-outs by error type. Found 4 overly aggressive file-level bail-outs (lint mode, incompatible imports, eslint suppression, runtime import check).

#### Stage 2b: Remove Overly Aggressive File-Level Bail-outs -- PARTIALLY COMPLETE (+1 net, 410→411)

Completed 2026-03-25. Removed 4 file-level bail-outs in `program.rs`:
- Removed `OutputMode::Lint` early return (+2 net passing, 42 fixtures now compile)
- Removed `has_known_incompatible_import` file-level bail (+0 net)
- Refined `has_compiler_runtime_import` to only bail on `c`/`useMemoCache` imports (+0 net)
- Removed `has_eslint_suppression_for_rules` file-level bail (+1 net passing)
Net result: bail-outs reduced 108→89, conformance +1 (410→411). 2 error.todo fixtures regressed (added to known-failures).
Remaining 89 bail-outs require per-validation fixes (see stages 2c-2f below).

#### Stage 2c: Fix `validateNoDerivedComputationsInEffects` (20 fixtures, largest group)

- [ ] Investigate why our validation fires on `@validateNoDerivedComputationsInEffects_exp` directive fixtures when upstream compiles
- [ ] Likely issue: we treat this as a bail condition when upstream only emits it as a diagnostic
- [ ] Compare our implementation against upstream's handling of this directive
- [ ] Fix: either skip bail for `_exp` directives, or correct the validation logic
- **Risk:** LOW-MEDIUM — 20 fixtures, highest bang-for-buck in remaining bail-outs
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)#stage-2b-fix-validatenoderivedcomputationsineffects-20-fixtures--15-18-gained

#### Stage 2d: Fix Frozen-Mutation False Positives (11 fixtures)

- [ ] Review `validate_no_mutation_after_freeze` / `InferMutableRanges` for over-reporting mutations on frozen values
- [ ] Compare our validation logic against upstream's to find divergence
- [ ] Implement targeted relaxations without losing true-positive detections
- **Risk:** MEDIUM — requires mutable range analysis refinements
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)

#### Stage 2e: Fix Ref-Access False Positives (8 fixtures)

- [ ] Review `validateNoRefAccessInRender` for over-eager patterns
- [ ] Some patterns (assigning ref-accessing functions to properties, ref type casts) should be allowed
- [ ] Compare against upstream validation
- **Risk:** MEDIUM
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

### Stage 3: Scope Inference — Small Slot Diffs (target: +30-50 fixtures)

**Pool:** 246 fixtures with ±1 slot diff, 55 with ±2 slot diff (301 total).
**Root causes:** Scope merging too aggressive/conservative, dependency over/under-counting, mutable range gaps.
**Risk:** HIGH — scope inference changes can cause cascading regressions.

#### Stage 3a: Investigate ±1 Slot Diff Patterns (est: 0 fixtures, prerequisite)

- [ ] Sample 30+ fixtures from the ±1 group
- [ ] For each: compare scope boundaries, identify which scope is extra/missing
- [ ] Categorize: (a) extra scope (over-splitting), (b) missing scope (under-splitting), (c) scope merged wrong, (d) dependency miscounted
- [ ] **Deliverable:** Root cause distribution with fixture counts
- **Critical:** Must identify whether fixes would cause regressions in currently-passing fixtures

#### Stage 3b: Fix Dominant ±1 Pattern (est: +15-25 fixtures)

- [ ] Implement fix for most common ±1 root cause from 3a
- [ ] **Regression check:** Run full conformance, verify no currently-passing fixtures break
- [ ] If regressions: revert, add to 3e investigation

#### Stage 3c: Fix Secondary ±1 Patterns (est: +10-15 fixtures)

- [ ] Fix remaining tractable ±1 patterns from 3a
- [ ] Regression check on each

#### Stage 3d: ±2 Slot Diff Quick Wins (est: +5-10 fixtures)

- [ ] Sample ±2 fixtures, check if any share root cause with ±1 fixes
- [ ] Fix only if low regression risk

#### Stage 3e: Replan — Slot Diff Residual (est: 0 fixtures, planning)

- [ ] Analyze remaining slot diff failures
- [ ] Determine if mutable_range switch (Phase 4d) would help or if deeper scope inference changes needed
- [ ] Update plan

---

### Stage 4: Validation Gaps — "We Compile, They Don't" (target: +20-30 fixtures)

**Pool:** 223 fixtures where upstream bails with an error but we compile (incorrectly).
**Risk:** LOW — adding validations is safe (bail-out = pass-through = correct).

#### Stage 4a: Categorize Missing Validations (est: 0 fixtures, prerequisite)

- [ ] For each of 223 fixtures, extract upstream's error message from expected output
- [ ] Group by validation type: locals-reassigned, direct-props-mutation, jsx-in-try, hooks-in-try, exhaustive-deps, other
- [ ] Count per validation, estimate implementation difficulty
- [ ] **Deliverable:** Validation gap breakdown, prioritized by fixture count

#### Stage 4b: Implement Top Missing Validations (est: +15-20 fixtures)

- [ ] Implement the 2-3 highest-count missing validations from 4a
- [ ] Wire into pipeline, run conformance
- [ ] Update known-failures.txt

#### Stage 4c: Implement Secondary Validations (est: +5-10 fixtures)

- [ ] Implement remaining tractable validations from 4a
- [ ] Run conformance, verify

---

### Stage 5: Stretch — Format Diffs and Large Slot Diffs (target: +10-20 fixtures)

**Pool:** 76 "both no memo" + 367 fixtures with ±3+ slot diffs.
**Risk:** MIXED — format fixes are easy, large slot diffs are hard.

#### Stage 5a: "Both No Memo" Format Fixes (est: +5-10 fixtures)

- [ ] Investigate 76 fixtures where neither side memoizes
- [ ] Fix cosmetic output diffs (whitespace, semicolons, import ordering)

#### Stage 5b: Large Slot Diff Triage (est: +5-10 fixtures)

- [ ] Sample ±3+ fixtures for any patterns that share root cause with Stage 3 fixes
- [ ] Cherry-pick easy wins only

---

### Milestone Summary

| Stage | Target | Cumulative (from 411) | Risk | Notes |
|-------|--------|-----------------------|------|-------|
| Stage 1b: Temp renumbering | +2 (done) | 405 | LOW | Completed. |
| Stage 1c: Minor codegen fixes | +5 (done) | 410 | LOW | Completed. |
| Stage 2a: Bail-out investigation | +0 (done) | 410 | -- | Completed. Categorized all 108 bail-outs. |
| Stage 2b: File-level bail-outs | +1 (done) | 411 | LOW | Completed. 108→89 bail-outs remaining. |
| Stage 1d: Declaration placement | +15-30 | 426-441 | HIGH | collect_all_scope_declarations redesign |
| Stage 2c: Effect-derived-computations | +15-18 | 426-429 | LOW-MED | 20 fixtures, largest fixable group |
| Stage 2d: Frozen-mutation | +5-8 | 431-437 | MEDIUM | 11 fixtures |
| Stage 2e: Ref-access | +3-5 | 434-442 | MEDIUM | 8 fixtures |
| Stage 2f: Reassignment | +5-7 | 439-449 | MEDIUM | 10 fixtures |
| Stage 2g: Other bail-outs | +5-10 | 444-459 | MIXED | ~40 remaining fixtures |
| Stage 3: ±1/±2 slot diffs | +30-50 | 474-509 | HIGH | |
| Stage 4: Missing validations | +20-30 | 494-539 | LOW | |
| Stage 5: Format + stretch | +10-20 | 504-559 | MIXED | |
| **Total remaining** | **+93-178** | **504-589** | | From 411 base |

**Key learning from Stage 1b:** Temp renumbering alone is nearly worthless (+2). The real "slots MATCH" gains require instruction ordering changes (Stage 1d), which is high-risk.

**Key learning from Stage 2a/2b:** Bail-out investigation revealed that most bail-outs are from specific validations (not silent/0-scope as originally assumed). The `validateNoDerivedComputationsInEffects` validation alone accounts for 20 of 89 remaining bail-outs. File-level bail-outs were low-hanging fruit (+1 net from removing 4).

**Revised path to 600:** From 411 base, need +189. Stages 2c-2g (bail-outs) could yield +33-48. Stage 3 (slot diffs) +30-50. Stage 4 (validations) +20-30. Conservative estimate: 504-539 without Stage 1d. Reaching 600 likely requires Stage 1d or overperformance in Stages 3-4.

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
