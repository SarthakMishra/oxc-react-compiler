# oxc-react-compiler Backlog

> Last updated: 2026-03-25 (post Phase 133)
> Conformance: **403/1717 (23.5%)**. Render: **92% (23/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Re-baselined expected files with `compilationMode: "all"` in Phase 133. Known-failures: 1314.

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

#### Stage 1a: Investigate "Slots MATCH" Patterns (est: 0 fixtures, prerequisite)

- [ ] Sample 20-30 "slots MATCH" fixtures, diff our output vs expected
- [ ] Categorize into sub-patterns: (a) variable naming, (b) instruction ordering, (c) dependency list ordering, (d) scope boundary placement, (e) other
- [ ] For each sub-pattern, count affected fixtures and estimate fix difficulty
- [ ] **Deliverable:** Sub-pattern breakdown with fixture counts, prioritized by ROI
- **If estimate is wrong:** Re-scope Stage 1b/1c based on actual findings

#### Stage 1b: Fix Dominant "Slots MATCH" Pattern (est: +25-40 fixtures)

- [ ] Implement fix for the most common sub-pattern from 1a
- [ ] Run conformance, verify gains, update known-failures.txt
- **If blockers found:** Document in Stage 1d investigation task

#### Stage 1c: Fix Secondary "Slots MATCH" Patterns (est: +15-20 fixtures)

- [ ] Implement fixes for remaining tractable sub-patterns from 1a
- [ ] Run conformance, verify gains, update known-failures.txt

#### Stage 1d: Replan — "Slots MATCH" Residual (est: 0 fixtures, planning)

- [ ] Categorize remaining "slots MATCH" failures after 1b/1c
- [ ] If >50 remain, investigate whether they share a common root cause
- [ ] Update this plan with new sub-tasks or mark as deferred

---

### Stage 2: False-Positive Bail-outs — "We Bail, They Compile" (target: +50-70 fixtures)

**Pool:** 108 fixtures where we incorrectly reject valid code.
**Risk:** MEDIUM — each bail-out removal must not introduce wrong output.

#### Stage 2a: Investigate Bail-out Categories (est: 0 fixtures, prerequisite)

- [ ] For each of the 108 fixtures, extract our error message/reason for bailing
- [ ] Group by validation pass: silent (0 scopes), frozen-mutation, ref-access, setState, hooks, reassignment, preserve-memo, other
- [ ] For "silent bailouts" (est ~51): determine if they're 0-slot functions (no reactive deps) or actual bugs
- [ ] **Deliverable:** Bail-out breakdown by pass, with fixture lists per group

#### Stage 2b: Silent Bail-outs / 0-Scope Functions (est: +30-40 fixtures)

- [ ] Investigate why these produce 0 reactive scopes with no error
- [ ] If 0-slot: implement 0-slot function emission (emit original code, no cache wrapper)
- [ ] If scope inference bug: fix scope creation to not drop valid scopes
- **Known blocker:** Phase 121 attempted 0-slot emission and got 68 regressions. Must investigate which regressions remain after Phase 130-133 improvements before re-attempting.
- **If blocker confirmed:** Add investigation task, defer to Stage 5

#### Stage 2c: Frozen Mutation / Ref-Access Relaxation (est: +10-15 fixtures)

- [ ] Review `validate_no_mutation_after_freeze` for over-strict patterns (11 fixtures)
- [ ] Review `validate_no_ref_access_in_render` for false positives (8 fixtures)
- [ ] For each, compare our validation logic against upstream's to find divergence
- [ ] Implement targeted relaxations without losing true-positive detections

#### Stage 2d: Other Bail-out Fixes (est: +10-15 fixtures)

- [ ] Fix remaining false-positive bail-outs: setState (4), hooks (3), reassignment (7), other (17)
- [ ] Each fix: compare upstream validation logic, adjust our thresholds

#### Stage 2e: Replan — Bail-out Residual (est: 0 fixtures, planning)

- [ ] Categorize remaining "we bail, they compile" after 2b-2d
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

| Stage | Target | Cumulative | Risk |
|-------|--------|------------|------|
| Stage 1: Slots MATCH codegen | +40-60 | 443-463 | LOW |
| Stage 2: False-positive bails | +50-70 | 493-533 | MEDIUM |
| Stage 3: ±1/±2 slot diffs | +30-50 | 523-583 | HIGH |
| Stage 4: Missing validations | +20-30 | 543-613 | LOW |
| Stage 5: Format + stretch | +10-20 | 553-633 | MIXED |
| **Total** | **+150-230** | **553-633** | |

**Conservative path to 600:** Stages 1-4 (est 543-613). Stage 3 must deliver well.
**Optimistic path to 600:** Stages 1-3 alone if estimates hold (est 523-583 + Stage 4 = 543-613).

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
