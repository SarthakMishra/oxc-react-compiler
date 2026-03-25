# oxc-react-compiler Backlog

> Last updated: 2026-03-25 (Phase 130: mutation range propagation fixes)
> Conformance: **528/1717 (30.8%)**. Render: **92% (23/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Re-baselined against upstream main on 2026-03-21. Fixture count unchanged (1717) but many files updated. 298 upstream error fixtures. Known-failures: 1189.
> Phase 130: +93 fixtures from mutation range propagation fixes (all 5 gaps). Render 23/25 is pre-existing (command-menu + canvas-sidebar).

---

## 1. Fix Mutation Range Propagation (Critical Path) -- DONE (Phase 130)

~~**Goal**: Fix `infer_mutation_aliasing_ranges.rs` so mutable ranges match upstream~~

**Completed**: All 5 gaps fixed in Phase 130 (+93 conformance fixtures, 0 regressions):

1. **StoreContext range extension** -- extends context var mutableRange.end when stored
2. **Phi mutableRange.start fixup** -- sets phi start to firstInstructionOfBlock - 1 when mutated after creation with start==0
3. **Operand mutableRange.start fixup** -- sets operand start to instr.id when range.end > instr.id and start==0
4. **fn.returns assignment from return terminals** -- wires return values to fn.returns in aliasing graph (required new `returns_id` parameter on `run_pipeline`)
5. **MaybeThrow terminal effect processing** -- processes Alias effects from MaybeThrow and Return terminal effects
6. **Lvalue mutableRange.start/end fixup** -- sets lvalue start/end when ==0 (discovered from upstream Part 2)
7. **Operand effect alignment** -- switched from `is_mutated_after_creation(id)` to upstream's `mutableRange.end > instr.id` check for Assign/Alias/Capture/CreateFrom/MaybeAlias source effects

**Remaining work**: Phase D (switch to mutable_range) NOT yet attempted. The effective_range workaround is still active. With ranges now more correct, switching may finally work. Next step: test with `use_mutable_range` flag.

---

## Remaining Phase Work

### Phase 2 Remaining: Impure Function Handling

**Files:** `src/inference/infer_mutation_aliasing_effects.rs`, `src/validation/`
**Status:** Deferred

- Impure function handling in legacy signatures -- requires `validate_no_impure_functions_in_render` integration
- Currently no validation that flags impure function calls in render scope

### Phase 4c: Remove `validate_no_mutation_after_freeze.rs` -- BLOCKED

**Files:** `src/validation/validate_no_mutation_after_freeze.rs`
**Status:** BLOCKED

Cannot remove yet. The standalone validator has independent hook-call-freezes-captures logic (freezes captured variables of function args passed to hook calls) that the effects pass does not handle. Removing would lose detection of mutations like `x.value += count` after `useIdentity(() => { setPropertyByKey(x, ...) })`. The effects pass would need to gain hook-argument-capture-freezing logic first.

**Note (Phase 119):** The hook-call-freezes-captures logic has a gap: imported hook names (via LoadGlobal) are not resolved in id_to_name. Added LoadGlobal tracking but the error fixtures `error.hook-call-freezes-captured-identifier.tsx` and `error.hook-call-freezes-captured-memberexpr.jsx` still don't trigger bail-out. Deeper investigation needed into how the HIR represents the CallExpression args for these patterns -- the FunctionExpression temp may not be linking to func_captures correctly after passes like inline_load_local_temps.

### Phase 4d: Switch to `mutable_range` -- READY TO RE-ATTEMPT

**Files:** `src/reactive_scopes/infer_reactive_scope_variables.rs`
**Status:** Previously FAILED 5x, but now mutation ranges are fixed (Phase 130, +93 fixtures). The 7 upstream gaps have been closed. Ready for another attempt.

**Approach:** Add `use_mutable_range: bool` flag to EnvironmentConfig (default false). In `infer_reactive_scope_variables.rs`, conditionally use raw mutable_range instead of effective_range. Test and compare.

### Phase 5: Fault Tolerance & Error Handling -- BLOCKED

**Files:** `src/error.rs`, `src/entrypoint/pipeline.rs`, all validation passes
**Status:** BLOCKED on compilation quality

**5a. Accumulate errors instead of early bail:**
- `Environment` / `ErrorCollector` should accumulate errors across all passes
- Passes wrapped in `try_record` -- a pass failure doesn't stop the pipeline
- `lower()` (HIR builder) always produces `HIRFunction` even on error
- Final error check after all passes complete

**5b. Remove local `CompilerError` bail-outs:**
Current pattern: each pass checks `errors.should_bail()` and returns `Err(())`. New pattern: pass records errors and continues. Pipeline checks aggregate errors at the end.

#### Blocker Report: PanicThreshold Default Change (Phase 119)

**Attempted:** Changing default PanicThreshold from AllErrors to CriticalErrors (matching upstream fault tolerance PRs #35872-35888).

**Result:** Conformance dropped 453->269 (-184). Reverted immediately.

**Root cause:** 132 fixtures that bail with AllErrors produce pass-through output matching expected. With CriticalErrors, they compile but produce WRONG output (different scope groupings, wrong slot counts), causing divergences. Upstream can use CriticalErrors because their compilation quality is higher -- when they don't bail, they produce correct output.

**Do NOT re-attempt** until conformance reaches ~600+ fixtures (35%+) and the "both compile, slots differ" category drops below 400.

### Phase 8 Remaining: Minor Improvements

- [ ] Try-catch support improvements (for loops, optional/logical in try/catch)
- [ ] IIFE inlining improvements
- [x] Improved scope merging for scopes that invalidate together (Phase 123: zero-dep eligibility, updateScopeDeclarations pruning, temporaries-aware output-to-input chain)
- [ ] Props spread optimization
- [ ] `ControlDominators.ts` utility (needed by Phase 2)
- [ ] Emitting 0-slot functions -- BLOCKED until more error validations are implemented (68 divergences when attempted in Phase 121)

### Other Open Work

- [ ] Constant propagation and DCE improvements (+25-30 fixtures) -- PARTIALLY BLOCKED — [details](#4-constant-propagation-and-dce-improvements-25-30-fixtures----partially-blocked)
- [ ] Missing validations (+50 fixtures) -- PARTIALLY DONE — [details](#8-missing-validations-50-fixtures----partially-done-phase-129)
- [ ] Remaining gating codegen (23 fixtures) -- import ordering, @dynamicGating, validation
- [ ] Remaining silent bailouts (7) -- Flow type casts, compilationMode:infer edge cases
- [ ] Remaining frozen mutation / ref validation (20 fixtures) -- effects-level issues, FE data-flow

---

## Critical Architecture Notes

**Read these before making ANY changes.**

### `effective_range` vs `mutable_range` -- READY TO RE-TEST (Phase 130 fixed ranges)
File: `src/reactive_scopes/infer_reactive_scope_variables.rs`

Uses `effective_range = max(mutable_range.end, last_use + 1)` because mutable ranges were too narrow for scope inference. **5 previous attempts** to switch all failed (96%->36% render). Phase 130 fixed 7 gaps in `infer_mutation_aliasing_ranges` (+93 fixtures). The mutable ranges should now be more correct. The effective_range workaround may no longer be needed -- re-test recommended.

### `collect_all_scope_declarations` is load-bearing
File: `src/reactive_scopes/codegen.rs`

Pre-declares ALL scope output variables at function level. Removing it causes render to drop 96%->24%.

### Block iteration order != source order for loops
The HIR blocks are stored in creation order, but for-loop constructs create blocks out of source order. The frozen mutation validator uses `frozen_at` instruction ID tracking.

### Render Regression Investigation (23/25 -> 24/25 FIXED)

**Symptom:** After Phase 2 commits, render dropped from 24/25 (96%) to 23/25 (92%). The `multi-step-form` fixture regressed -- `completedFields` useMemo returned `{completed: 0, total: 0}` instead of `{completed: 0, total: 1}`.

**Root cause:** `PostfixUpdate` and `PrefixUpdate` instructions were missing from the side-effect allowlist in `build_inline_map()` (codegen.rs line ~652). Fix: Added them to the side-effect match alongside `PropertyStore`, `StoreLocal`, etc.

**Note (Phase 121):** `command-menu` and `canvas-sidebar` re-regressed to `semantic_divergence`. Root cause NOT from Phase 121 changes. Needs investigation.

### Cross-scope `IdentifierId` mismatch
Nested function bodies have their own `IdentifierId` numbering. Name-based resolution needed for cross-scope tracking.

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

1. **effective_range is load-bearing.** 5 attempts to switch to mutable_range have failed. Do not attempt without understanding upstream's usage extension logic.
2. **collect_all_scope_declarations cannot be removed.** It prevents render collapse from 96% to 24%.
3. **PanicThreshold change to CriticalErrors requires ~600+ conformance.** 132 bail-out fixtures produce wrong output when compiled instead of bailed.
4. **Emitting 0-slot functions requires more error validations.** 68 divergences when attempted (Phase 121).
5. **Render regressions can be latent.** The PostfixUpdate/PrefixUpdate codegen bug existed for months but only appeared when the new inference model enabled deeper function body compilation.
6. **Fix low-risk bail-outs before high-risk scope inference.** The gap analysis shows ~200 fixtures recoverable from validation tuning and codegen fixes (items 1-7) vs ~400 from hard scope inference work (item 10). Pick the easy wins first.
7. **"Both compile, slots match" (245 fixtures) are mostly cosmetic.** Variable naming and structural diffs -- lower priority than correctness gaps but good cleanup targets.
8. **Preserve-memo validation needs `ManualMemoDependency` for full upstream fidelity.** Without source deps on `StartMemoize` and `validateInferredDep`, we can't detect dep mismatch errors. The `start_scope != finish_scope` check was an accidental proxy that caught both true positives (31 error fixtures) and false positives (54 valid fixtures). Removing it trades 31 undetectable errors for 54 recovered compilations.

---

## Completed Work (Phases 124-129)

| # | Item | Status | Phase |
|---|------|--------|-------|
| 1 | Relax preserve-memo validation (+58 fixtures) | DONE | 124 |
| 2 | Variable name preservation in codegen (+47 fixtures) | RECLASSIFIED -- actually scope inference issue | 125 |
| 3 | fbt call preservation (+36 fixtures) | PARTIALLY DONE (14/38) | 126 |
| 4 | Constant propagation and DCE improvements (+25-30) | PARTIALLY BLOCKED -- most are 0-slot functions | 127 |
| 5 | Gating codegen (+27 fixtures) | PARTIALLY DONE (4/27) | 127 |
| 6 | Fix silent bailouts (+23 fixtures) | PARTIALLY DONE (16/23) | 128 |
| 7 | Frozen mutation / ref validation tuning (+20 fixtures) | PARTIALLY DONE (3/20) | 128 |
| 8 | Missing validations (+50 fixtures) | PARTIALLY DONE (4/54) | 129 |
| 9 | Try/catch scope handling (+37 fixtures) | INVESTIGATED -- blocked on scope inference | 129 |
