# oxc-react-compiler Backlog

> Last updated: 2026-03-23 (post Phase 119, conformance housekeeping)
> Conformance: **453/1717 (26.4%)**. Render: **96% (24/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Re-baselined against upstream main on 2026-03-21. Fixture count unchanged (1717) but many files updated. 298 upstream error fixtures. Known-failures: 1264.

---

## HIGH PRIORITY: Upstream Alignment Port

The upstream React Compiler has undergone major architectural changes since our port was created (early 2025). Our code is based on the OLD architecture that has been **deleted upstream**. This section plans the full alignment port.

### Port Phase 0: Re-baseline Fixtures & Conformance ✅

~~**Effort:** 1 session~~
~~**Risk:** LOW — no compiler changes~~

**Completed:** 2026-03-21. Re-downloaded upstream fixtures from facebook/react main via tarball. 3437 files extracted (1446 .js, 97 .tsx, 165 .ts, 9 .jsx, 1718 .expect.md). Regenerated 1718 .expected files (1420 with code, 298 upstream errors). Conformance unchanged at 456/1717 (26.6%). 1 new divergence added to known-failures.txt (allow-modify-global-in-callback-jsx.js). 0 panics. All tests pass.

---

### Port Phase 1: HIR Type System Updates ✅

~~**Effort:** 1-2 sessions~~
~~**Risk:** MEDIUM — touches core types used everywhere~~

**Completed:** 2026-03-21. All type system changes implemented and compiling. Changes:
- 1a. `ValueReason` expanded to 12 variants matching upstream. `MutationReason` added. `AliasingEffect::Apply` gained `mutates_function` and `loc` fields. `Freeze` migrated from `FreezeReason` to `ValueReason`.
- 1b. `AliasingSignature` type added to `types.rs`.
- 1c. `HIRFunction.aliasing_effects`, `Terminal::Return.effects`, `Terminal::MaybeThrow.effects` added. `Instruction.effects` was already present.
- 1d. `AliasingSignatureConfig`, `AliasingEffectConfig`, `ApplyArgConfig` types added to `types.rs`.

24 files updated across the codebase. All tests pass, conformance unchanged at 456/1717.

---

### Port Phase 2: New InferMutationAliasingEffects (~2975 lines) — IN PROGRESS

**Effort:** 3-5 sessions (largest single piece)
**Risk:** HIGH — core abstract interpreter
**Files:** `src/inference/infer_mutation_aliasing_effects.rs` (rewrite)
**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/Inference/InferMutationAliasingEffects.ts`

**Session 1 (2026-03-22): Core architecture rewrite COMPLETE**

The following sub-items are done:

**2a. `InferenceState` — Abstract State: DONE**
- Two-layer model: `values: Vec<AbstractValue>` + `variables: FxHashMap<IdentifierId, FxHashSet<AbstractValueId>>`
- Handles phi nodes naturally (union of value sets from predecessors)
- `kind()` merges all values a variable may hold via lattice join
- `freeze()`, `mutate()`, `assign()`, `append_alias()`, `infer_phi()` all implemented

**2b. Fixpoint Iteration: DONE**
- Worklist-based: only re-processes blocks whose incoming state changed
- Merges states at join points via `InferenceState::merge()`
- Instruction signature cache: candidate effects computed once per instruction

**2c. Effect Application Logic: DONE**
- `applyEffect()` function with full refinement: Capture/Alias/MaybeAlias on frozen -> ImmutableCapture, MutateConditionally on frozen -> no-op, Mutate on frozen -> MutateFrozen error, etc.
- Matches upstream's recursive applyEffect pattern

**2d. Function Call Resolution: PARTIALLY DONE**
- Legacy signature resolution (FunctionSignature with per-param effects): DONE
- Conservative fallback (no signature): DONE
- AliasingSignature resolution (new-style): NOT YET (depends on 2e)
- Local FunctionExpression resolution (using aliasingEffects): NOT YET (depends on AnalyseFunctions rewrite in Phase 4)

**2e. Built-in Function Signatures: DONE** (Phase 117)
- `populate_builtin_signatures()` scans LoadGlobal instructions, inserts FunctionSignature for known globals
- 15 React hooks with precise per-param effects (useState, useRef, useEffect, useMemo, etc.)
- Pure globals: parseInt, parseFloat, isNaN, isFinite, encode/decodeURI(Component), atob, btoa, String, Number, Boolean, structuredClone
- Unknown hooks deliberately get no signature (conservative fallback is safer)
- Note: AliasingSignatureConfig types from Phase 1 are NOT yet used — FunctionSignature legacy path is used instead

**Conformance unchanged at 456/1717, 0 panics, +2 newly passing fixtures.**

**Session 2 (2026-03-22): Remaining Phase 2 items COMPLETE**

- Port `computeEffectsForSignature()`: DONE — AliasingSignature substitution with full identifier mapping
- Port `buildSignatureFromFunctionExpression()`: DONE — builds AliasingSignature from FE params + aliasing_effects
- Port `try_resolve_function_expression()`: DONE — checks if call target has known effects before fallback
- Port try/catch handler binding logic for MaybeThrow terminals: DONE — catch_handlers map + terminal aliasing
- Port `mutableOnlyIfOperandsAreMutable`: DONE — added to FunctionSignature with are_arguments_immutable()
- Refactored Apply effect handling into `apply_call_effect()` with 3-step resolution

**Remaining work for Phase 2 (deferred):**
- ~~Built-in function signatures (2e)~~ — DONE (Phase 117)
- ~~MethodCall signature resolution~~ — DONE (Phase 118). Added method-level signatures for Math, JSON, Object, Array (static + instance), Number, String. Propagation through Store/Load/Phi chains. Console methods intentionally left without signatures (impure). Limitation: propagation doesn't reach all receivers (e.g. arrays from function params/returns).
- Impure function handling in legacy signatures — requires `validate_no_impure_functions_in_render` integration

**This replaces our current abstract interpreter** which is the root cause of Gap 11 (~404 fixtures) and indirectly blocks Gap 5a (58 fixtures) and Gap 7 (175 fixtures).

---

### Port Phase 3: New InferMutationAliasingRanges (~737 lines) — MOSTLY DONE

**Effort:** 2-3 sessions (1.5 completed)
**Risk:** HIGH — directly computes mutable ranges that feed scope inference
**Files:** `src/inference/infer_mutation_aliasing_ranges.rs` (rewrite)
**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/Inference/InferMutationAliasingRanges.ts`

**3a. `AliasingState` Graph: DONE** (from prior work)
- Ordered data flow graph with typed edges (assign, alias, capture, createdFrom, maybeAlias)
- Time-indexed edges and mutations
- Graph built in `AliasingGraph` struct with `Node`, `Edge`, `BackEdge` types

**3b. Mutable Range Computation: DONE** (from prior work)
- BFS propagation via `mutate()` function with dedup and direction tracking
- Handles phi nodes, backward aliases, createdFrom, and captures

**3c. Per-Place Effect Annotation: DONE** (Phase 115)
- `annotate_place_effects()` sets Place.effect for all operands, lvalues, phis, terminals
- Effect-based refinements from instruction AliasingEffects
- Effect priority system prevents downgrades

**3d. Function Effect Inference: DONE** (Phase 115)
- `compute_aliasing_effects()` in analyse_functions.rs
- Emits Mutate/MutateConditionally for mutated params and context vars
- Stores on HIRFunction.aliasing_effects for caller resolution

**Remaining work for Phase 3:**
- None — all core items implemented. May need refinement when built-in signatures (Phase 2e) are populated.

**Critical interaction with `effective_range`:** Our current `effective_range = max(mutable_range.end, last_use + 1)` approximation in `infer_reactive_scope_variables.rs` is **still needed** even with the new inference model. The 5th attempt to switch to `mutable_range` (Phase 117) failed with render 96%→36%. Root cause: our `infer_mutation_aliasing_ranges` computes mutation propagation ranges, but upstream's mutableRange additionally includes usage extension. The effective_range approximation compensates for this gap.

---

### Port Phase 4: Update AnalyseFunctions & Pipeline — PARTIALLY DONE

**Effort:** 1-2 sessions
**Risk:** MEDIUM
**Files:** `src/inference/analyse_functions.rs`, `src/entrypoint/pipeline.rs`

**4a. Update `AnalyseFunctions`: DONE** (Phase 117)
Sub-pipeline now runs: InferTypes, InferMutationAliasingEffects, DeadCodeElimination, InferMutationAliasingRanges, annotate_last_use, RewriteInstructionKinds, InferReactiveScopeVariables. Built-in signatures are populated before effects inference.

**4b. Update Pipeline Pass Ordering: DONE** (already matched upstream)
Pipeline order already matches: AnalyseFunctions → populate_builtin_signatures → InferMutationAliasingEffects → validate_no_mutation_after_freeze → SSR → DCE → PruneMaybeThrows → InferMutationAliasingRanges → validations → InferReactivePlaces.

**4c. Remove `validate_no_mutation_after_freeze.rs`: BLOCKED**
Cannot remove yet. The standalone validator has independent hook-call-freezes-captures logic (freezes captured variables of function args passed to hook calls) that the effects pass does not handle. Removing would lose detection of mutations like `x.value += count` after `useIdentity(() => { setPropertyByKey(x, ...) })`. The effects pass would need to gain hook-argument-capture-freezing logic first.

**Note (Phase 119):** The hook-call-freezes-captures logic has a gap: imported hook names (via LoadGlobal) are not resolved in id_to_name. Added LoadGlobal tracking but the error fixtures `error.hook-call-freezes-captured-identifier.tsx` and `error.hook-call-freezes-captured-memberexpr.jsx` still don't trigger bail-out. Deeper investigation needed into how the HIR represents the CallExpression args for these patterns -- the FunctionExpression temp may not be linking to func_captures correctly after passes like inline_load_local_temps.

**4d. Try switching to `mutable_range` instead of `effective_range`: FAILED (5th attempt)**
Attempted 2026-03-22 (Phase 117). Render dropped 96%→36% (9/25), conformance dropped 456→432. Reverted. The effective_range approximation (`max(mutable_range.end, last_use + 1)`) is still load-bearing. The new inference model's mutable ranges cover mutation reach but NOT usage reach — scope inference needs usage extension to group values correctly. **Do NOT attempt again without first investigating why the ranges are too narrow.** The root cause is that upstream's `mutableRange` includes usage extension in its range computation, while our `infer_mutation_aliasing_ranges` only computes mutation propagation ranges.

---

### Port Phase 5: Fault Tolerance & Error Handling

**Effort:** 1 session
**Risk:** LOW — additive, doesn't change pass logic
**Files:** `src/error.rs`, `src/entrypoint/pipeline.rs`, all validation passes

**5a. Accumulate errors instead of early bail:**
- `Environment` / `ErrorCollector` should accumulate errors across all passes
- Passes wrapped in `try_record` — a pass failure doesn't stop the pipeline
- `lower()` (HIR builder) always produces `HIRFunction` even on error
- Final error check after all passes complete

**5b. Remove local `CompilerError` bail-outs:**
Current pattern: each pass checks `errors.should_bail()` and returns `Err(())`. New pattern: pass records errors and continues. Pipeline checks aggregate errors at the end.

### Blocker Report: PanicThreshold Default Change (Phase 119)

**Attempted:** Changing default PanicThreshold from AllErrors to CriticalErrors (matching upstream fault tolerance PRs #35872-35888).

**Result:** Conformance dropped 453→269 (-184). Reverted immediately.

**Root cause:** 132 fixtures that bail with AllErrors produce pass-through output matching expected. With CriticalErrors, they compile but produce WRONG output (different scope groupings, wrong slot counts), causing divergences. Upstream can use CriticalErrors because their compilation quality is higher -- when they don't bail, they produce correct output.

**What must happen first:** Core compilation quality (scope inference, codegen) must improve significantly before CriticalErrors can be the default. The 132 bail-out fixtures need to produce correct memoization output, not just "some" memoization. Estimated improvement needed: majority of the 892 "both compile but differ" fixtures must first be resolved.

**Do NOT re-attempt** until conformance reaches ~600+ fixtures (35%+) and the "both compile, slots differ" category drops below 400.

---

### Port Phase 6: New & Updated Validation Passes

**Effort:** 1-2 sessions
**Risk:** LOW — additive passes
**Files:** New files in `src/validation/`

**6a. `ValidateExhaustiveDependencies` (new):**
Compiler-side dependency checking for manual `useMemo`/`useCallback`. Checks that all reactive dependencies are included. Deduplicates with `ValidatePreservedManualMemoization` via `hasInvalidDeps` flag on `StartMemoize`.

**6b. `ValidateNoVoidUseMemo` (new):**
Catches `useMemo` calls with void returns (common mistake).

**6c. Update ref validation:**
Ref-like identifiers (names ending in `Ref`) treated as refs by default. Improved ref validation for non-mutating functions.

**6d. `PropagatePhiTypes` → merged into `InferTypes`:**
Delete `propagate_phi_types.rs` (if it exists). Merge its logic into `infer_types.rs`.

---

### Port Phase 7: Config & API Alignment

**Effort:** 1 session
**Risk:** LOW

**7a. `outputMode` replaces `noEmit`:**
New modes: `client` (default), `ssr`, `lint`, `null`. Reactive scope creation and codegen gated by `outputMode === 'client'`.

**7b. Feature flag cleanup:**
Remove: Fire, inline JSX, context selectors, instruction reordering flags.

**7c. `HIRFunction.returns` restructure:**
`returns: Place` → `returns: { place: Place }`.

---

### Port Phase 8: Minor Improvements

**Effort:** 1 session
**Risk:** LOW

- Try-catch support improvements (for loops, optional/logical in try/catch)
- IIFE inlining improvements
- Constant propagation for template literals and unary minus
- Improved scope merging for scopes that invalidate together
- Props spread optimization
- `ControlDominators.ts` utility (needed by Phase 2)

---

## Port Execution Strategy

### Recommended Order

```
Phase 0 (Re-baseline)         → 1 session, do FIRST
Phase 1 (Types)                → 1-2 sessions, no dependencies
Phase 2 (Effects inference)    → 3-5 sessions, depends on Phase 1
Phase 3 (Ranges)               → 2-3 sessions, depends on Phase 2
Phase 4 (Pipeline)             → 1-2 sessions, depends on Phase 2+3
Phase 5 (Error handling)       → 1 session, independent
Phase 6 (Validation passes)    → 1-2 sessions, independent
Phase 7 (Config)               → 1 session, independent
Phase 8 (Minor)                → 1 session, independent
```

**Total estimated effort: 12-18 sessions**

Phases 5-8 can be done in parallel with or after Phases 2-4. The critical path is:
```
Phase 0 → Phase 1 → Phase 2 → Phase 3 → Phase 4 → Re-test conformance
```

### Risk Mitigation

- **Phase 2 is the riskiest piece** (~2975 lines of abstract interpreter). Consider:
  - Port incrementally: start with a minimal interpreter that handles simple cases
  - Keep the old pass as a fallback behind a feature flag
  - Test against render benchmark after each increment
  - The new model is reportedly simpler, so it may be easier to port than the old one

- **Phase 4d (switching to `mutable_range`)** is the key validation:
  - If render stays at 96% with `mutable_range` after Phase 3, the port is successful
  - If render drops, the new ranges are still wrong and we need to investigate

- **Commit checkpoints:** After each phase, commit and run full conformance + render benchmarks. Record the numbers.

### Expected Conformance Impact

| Phase | Expected Impact |
|-------|----------------|
| Phase 0 (re-baseline) | Conformance number changes (could go up or down) |
| Phases 1-4 (new aliasing model) | **+200-400 fixtures** — unblocks Gap 11, Gap 5a, Gap 7 |
| Phase 5 (fault tolerance) | +10-20 (fewer false bail-outs from early error stopping) |
| Phase 6 (new validations) | +5-15 (fewer over-compiles where we should bail) |
| Phases 7-8 | +5-10 |

**Conservative estimate: 456 → 650-750 (38-44%)**
**Optimistic estimate: 456 → 800+ (47%+)**

---

## Critical Architecture Notes

**Read these before making ANY changes.**

### `effective_range` vs `mutable_range` — STILL NEEDED (5 failed attempts)
File: `src/reactive_scopes/infer_reactive_scope_variables.rs`

Uses `effective_range = max(mutable_range.end, last_use + 1)` because mutable ranges are too narrow for scope inference. **5 attempts** to switch to `mutable_range` have all failed (96%→36% render). The problem persists even with the new inference model (Phases 2-3). Root cause: our `infer_mutation_aliasing_ranges` computes mutation propagation ranges only. Upstream's mutableRange includes usage extension that our model doesn't. **Do NOT attempt again** without first investigating how upstream extends ranges to include usage reach.

### `collect_all_scope_declarations` is load-bearing
File: `src/reactive_scopes/codegen.rs`

Pre-declares ALL scope output variables at function level. Removing it causes render to drop 96%→24%.

### Block iteration order ≠ source order for loops
The HIR blocks are stored in creation order, but for-loop constructs create blocks out of source order. The frozen mutation validator uses `frozen_at` instruction ID tracking. After Phase 4c (removing the standalone validator), this is handled by the new inference.

### Render Regression Investigation (23/25 -> 24/25 FIXED)

**Symptom:** After Phase 2 commits (c99311b through e84a583), render dropped from 24/25 (96%) to 23/25 (92%). The `multi-step-form` fixture regressed -- `completedFields` useMemo returned `{completed: 0, total: 0}` instead of `{completed: 0, total: 1}`.

**Root cause:** `PostfixUpdate` and `PrefixUpdate` instructions were missing from the side-effect allowlist in `build_inline_map()` (codegen.rs line ~652). When their result temp had 0 uses (normal for `total++` as an expression statement), they were incorrectly marked as dead pure temps and skipped during emission.

**Why it only appeared after Phase 2:** Before Phase 2, the useMemo callback in `multi-step-form` was NOT being transformed by the compiler (passed through as-is). The new inference gives the compiler enough information to transform nested function bodies, which exposed the latent bug in codegen's dead-temp elimination.

**Fix:** Added `PrefixUpdate` and `PostfixUpdate` to the side-effect match in `build_inline_map()`, alongside `PropertyStore`, `StoreLocal`, etc. This ensures increment/decrement expression statements are never eliminated even when their result is unused.

**Bisect results:**
- 968ece3 (Phase 1 types): PASS (24/25)
- c99311b (Phase 2 core): FAIL (23/25) -- regression introduced here
- Fix applied on main: PASS (24/25)

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
| HIR builder (AST→HIR) | `src/hir/build.rs` |
| Code generation | `src/reactive_scopes/codegen.rs` |
| **Aliasing effects (rewrite)** | `src/inference/aliasing_effects.rs` |
| **Mutation effects (rewrite)** | `src/inference/infer_mutation_aliasing_effects.rs` |
| **Mutation ranges (rewrite)** | `src/inference/infer_mutation_aliasing_ranges.rs` |
| **Function analysis (rewrite)** | `src/inference/analyse_functions.rs` |
| Scope grouping | `src/reactive_scopes/infer_reactive_scope_variables.rs` |
| Scope dependencies | `src/reactive_scopes/propagate_dependencies.rs` |
| Scope pruning + rename | `src/reactive_scopes/prune_scopes.rs` |
| Frozen mutation validation | `src/validation/validate_no_mutation_after_freeze.rs` **(remove after Phase 4c)** |
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
