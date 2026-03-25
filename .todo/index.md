# oxc-react-compiler Backlog

> Last updated: 2026-03-25 (post silent-bailout + validation tuning)
> Conformance: **441/1717 (25.7%)**. Render: **96% (24/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Re-baselined against upstream main on 2026-03-21. Fixture count unchanged (1717) but many files updated. 298 upstream error fixtures. Known-failures: 1276.
> Bail-outs reduced: 75->60 (Phase 128). Silent bail-outs: 23->7. Flow component/hook preprocessing added.

---

## Conformance Gap Analysis (1276 failures)

| Category | Count | % | Description |
|----------|-------|---|-------------|
| Both compile, slots DIFFER | 712 | 55.8% | We compile but produce wrong scope groupings or slot counts |
| Both compile, slots MATCH | 231 | 18.1% | Correct scopes but cosmetic diffs (structure, variable names) |
| We compile, they don't | 193 | 15.1% | Missing validations -- we should bail but don't |
| We bail, they compile | 60 | 4.7% | Overly strict bail-outs -- we reject valid programs |
| Both no memo (format diff) | 80 | 6.3% | Both pass through but output format differs |

### Bail-out breakdown (60 "we bail, they compile")

| Cause | Count |
|-------|-------|
| Preserve-memo validation | 4 |
| Silent bail (no error, 0 scopes) | 7 |
| Frozen mutation | 12 |
| Ref access | 8 |
| Global reassignment | 7 |
| Local variable reassignment | 7 |
| Other | 15 |

---

## Open Work -- Prioritized by Impact

### ~~1. Relax preserve-memo validation (+58 fixtures)~~ DONE (Phase 124)

**Completed:** Removed non-upstream `start_scope != finish_scope` check. Preserved `finish_scope.is_none()` check. Reduced preserve-memo bail-outs from 58 to 4. Trade-off: 31 error fixtures that require `validateInferredDep` (dep comparison infrastructure: `ManualMemoDependency`, source deps on `StartMemoize`) moved to known-failures. Net: 54 fewer bail-outs, 31 new known-failures for dep comparison errors we can't detect without source deps.

**Follow-up needed:** Implement `ManualMemoDependency` type, store source deps on `StartMemoize`, implement `validateInferredDep` to recover the 31 error fixtures.

### ~~2. Variable name preservation in codegen (+47 fixtures)~~ RECLASSIFIED

**Investigation (Phase 125):** Deep investigation revealed this is NOT a codegen naming issue. The ~26 fixtures showing `const t0` vs `const x` patterns have **different scope boundaries** than upstream. Our scope wraps a narrower set of instructions (e.g., only the array allocation), while upstream wraps more (allocation + mutation). As a result, the scope output is a temp (the intermediate computation result) rather than the named variable. Fixing this requires scope inference improvements (item #10), not codegen changes.

**Partial improvement committed:** Added scope output promotion in codegen (`build_scope_output_promotions`) that replaces temp scope declarations with named variables when a StoreLocal immediately follows a scope. This produces cleaner output but doesn't recover fixtures because the scope body content also differs.

### ~~3. fbt call preservation (+36 fixtures)~~ PARTIALLY DONE (Phase 126)

**Completed (14/38 fixtures):** Root cause was that upstream runs `babel-plugin-fbt` alongside the React Compiler, transforming `<fbt>` JSX to `fbt._()` calls. Added `preprocess-fbt.mjs` to pre-process fixture inputs the same way. 14 fixtures now pass. Remaining 24 failures: 4 error.todo (need validation), ~16 scope inference (item #10), 4 structural diffs.

**Follow-up needed:** Implement `memoize_fbt_and_macro_operands_in_same_scope` pass (currently a no-op stub) to merge fbt operand scopes -- may recover some of the remaining scope divergences.

### 4. Constant propagation and DCE improvements (+25-30 fixtures) -- PARTIALLY BLOCKED

**Files:** `src/optimization/constant_propagation.rs`, `src/optimization/dead_code_elimination.rs`
**Difficulty:** HARD | **Risk:** MEDIUM

**Investigation (Phase 127):** Analyzed 40+ const-prop/DCE known-failure fixtures. Finding: nearly all (35+) have 0 cache slots -- these are non-component helper functions where upstream applies const-prop/DCE but produces no memoization. Recovery requires emitting optimized 0-slot functions, which is blocked (Phase 121: 68 regressions when attempted). The ~5 memoized fixtures require deep improvements: phi-node constant propagation across branches, branch elimination on constant tests, and function extraction with constant inlining. Reclassified from MEDIUM to HARD difficulty.

**What would help:** Emitting 0-slot functions (blocked on error validation) would recover ~25 fixtures. Deep phi-node const-prop would recover ~5 more memoized fixtures.

### ~~5. Gating codegen (+27 fixtures)~~ PARTIALLY DONE (Phase 127)

**Files:** `src/reactive_scopes/codegen.rs`, `src/entrypoint/program.rs`, `src/entrypoint/options.rs`

**Completed (4/27 fixtures):** Implemented per-function gating ternary wrapper matching upstream's pattern (`const Name = gatingFn() ? compiled : original`). Handles all function contexts: declarations, export-default, export-named, variable declarations. Also fixed `should_compile_default_export` for annotation/syntax modes. 4 fixtures now pass: `gating-test`, `gating-test-export-function`, `gating-test-export-default-function`, `gating-preserves-function-properties`.

**Remaining 23 gating fixtures:** 8 have import ordering divergences (gating import sorts differently from user imports due to prepend placement), 6 are `@dynamicGating` fixtures (different gating function per fixture -- need to parse per-function gating annotations), 4 are validation-related (conflicting gating, invalid identifiers), 5 are other structural diffs (component syntax, wrapper calls).

### ~~6. Fix silent bailouts (+23 fixtures)~~ PARTIALLY DONE (Phase 128)

**Completed (16/23 fixtures):** Added Flow component/hook syntax preprocessor in `program.rs` that converts `component Foo(...)` to `function Foo({...})` and `hook useFoo(...)` to `function useFoo(...)` before parsing. 16 fixtures recovered from silent bailout (moved to compilation). 3 of those now pass conformance. Trade-off: 3 error fixtures that were accidentally matching now expose real validation gaps (added to known-failures).

**Remaining 7 silent bailouts:** Flow type cast expressions `(x: Foo)` (2 fixtures), `@compilationMode:"infer"` edge cases (2 fixtures: `infer-functions-component-with-ref-arg.js`, gating fixtures), `unused-object-element-with-rest.js` (destructuring), `exhaustive-deps` edge case (1 fixture).

### ~~7. Frozen mutation / ref validation tuning (+20 fixtures)~~ PARTIALLY DONE (Phase 128)

**Completed (3/20 fixtures):** Added `useEffectEvent` to ref-access validator's non-render hook list (-2 ref-access bail-outs). Fixed `@directive false` space-separated parsing. Skipped ref-typed captures in frozen mutation hook-call-freezes-captures logic (-1 frozen-mutation bail-out). Added return-value FEs to non-render IDs.

**Remaining:** 12 frozen-mutation bail-outs (mostly effects-level issues requiring changes to `infer_mutation_aliasing_effects`), 8 ref-access bail-outs (5 need FE data-flow analysis to distinguish render-time from deferred calls, 3 from newly-compiling Flow fixtures).

**Investigation findings:** Removing `is_ref_name` heuristic entirely fixes 7+ fixtures but regresses 3 error fixtures that need name-based ref detection for props like `Component({ref})` and `props.ref.current`. The heuristic IS needed for prop-based ref detection. Deeper fix requires tracking whether a name comes from `useRef()` vs destructured props.

### 8. Missing validations (+50 fixtures)

**Files:** `src/validation/`, `src/error.rs`
**Difficulty:** MEDIUM | **Risk:** MEDIUM

We incorrectly compile 50+ fixtures where upstream emits an error. Need to add error detection for patterns upstream rejects (subset of the 152 "we compile, they don't" category).

### 9. Try/catch scope handling (+25 fixtures)

**Files:** `src/hir/build.rs`, `src/reactive_scopes/`
**Difficulty:** HARD | **Risk:** MEDIUM

25 fixtures diverge due to incorrect scope handling in try/catch blocks. Includes for-loops in try/catch, optional/logical expressions in try/catch.

### 10. Scope inference fixes (+300-400 fixtures)

**Files:** `src/reactive_scopes/infer_reactive_scope_variables.rs`, `src/reactive_scopes/propagate_dependencies.rs`
**Difficulty:** HARD | **Risk:** MEDIUM

The single biggest bucket -- 300-400 fixtures in the "both compile, slots differ" category are due to scope inference producing different groupings than upstream. This is the hardest category to fix but has the largest potential payoff.

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

### Phase 4d: Switch to `mutable_range` -- FAILED 5x, DO NOT ATTEMPT

**Files:** `src/reactive_scopes/infer_reactive_scope_variables.rs`
**Status:** FAILED -- do NOT re-attempt without prerequisite investigation

Attempted 5 times, most recently 2026-03-22 (Phase 117). Every attempt drops render from 96% to 36% (9/25). The effective_range approximation (`max(mutable_range.end, last_use + 1)`) is still load-bearing. Root cause: our `infer_mutation_aliasing_ranges` computes mutation propagation ranges, but upstream's `mutableRange` additionally includes usage extension. The effective_range approximation compensates for this gap.

**Do NOT attempt again without first investigating how upstream extends ranges to include usage reach.**

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

---

## Critical Architecture Notes

**Read these before making ANY changes.**

### `effective_range` vs `mutable_range` -- STILL NEEDED (5 failed attempts)
File: `src/reactive_scopes/infer_reactive_scope_variables.rs`

Uses `effective_range = max(mutable_range.end, last_use + 1)` because mutable ranges are too narrow for scope inference. **5 attempts** to switch to `mutable_range` have all failed (96%->36% render). The problem persists even with the new inference model (Phases 2-3). Root cause: our `infer_mutation_aliasing_ranges` computes mutation propagation ranges only. Upstream's mutableRange includes usage extension that our model doesn't. **Do NOT attempt again** without first investigating how upstream extends ranges to include usage reach.

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
