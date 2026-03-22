# oxc-react-compiler Backlog

> Last updated: 2026-03-22 (post Phase 113, Port Phase 2 session 1 — core architecture rewrite)
> Conformance: **456/1717 (26.6%)**. Render: **96% (24/25)**. E2E: **95-100%**. Tests: all pass, 0 panics.
> Re-baselined against upstream main on 2026-03-21. Fixture count unchanged (1717) but many files updated. 298 upstream error fixtures. 1 new divergence (allow-modify-global-in-callback-jsx.js).

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

**2e. Built-in Function Signatures: NOT YET**
- String-based AliasingSignatureConfig types added in Phase 1
- No built-in signatures populated yet — this requires updating ObjectShape/Globals equivalent

**Conformance unchanged at 456/1717, 0 panics, +2 newly passing fixtures.**

**Remaining work for Phase 2:**
- Port `computeEffectsForSignature()` (AliasingSignature substitution)
- Port `buildSignatureFromFunctionExpression()` (local FE resolution)
- Port try/catch handler binding logic for MaybeThrow terminals
- Improve `computeSignatureForInstruction` to better match upstream (DeclareContext hoisting, LoadLocal as Assign, etc.)
- Port `computeEffectsForLegacySignature` refinements (mutableOnlyIfOperandsAreMutable, impure handling)

**This replaces our current abstract interpreter** which is the root cause of Gap 11 (~404 fixtures) and indirectly blocks Gap 5a (58 fixtures) and Gap 7 (175 fixtures).

---

### Port Phase 3: New InferMutationAliasingRanges (~737 lines)

**Effort:** 2-3 sessions
**Risk:** HIGH — directly computes mutable ranges that feed scope inference
**Files:** `src/inference/infer_mutation_aliasing_ranges.rs` (rewrite)
**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/Inference/InferMutationAliasingRanges.ts`

**3a. `AliasingState` Graph:**
- Ordered data flow graph: nodes = values, edges = typed (assign, alias, capture, createFrom, maybeAlias)
- Edges and mutations are **time-indexed** (ordered by instruction ID)

**3b. Mutable Range Computation:**
- For each mutation effect from Phase 2, walk reachable nodes in the graph at the mutation's time index
- Extend `identifier.mutableRange` for all reached values
- Handle phi nodes by connecting phi operands to phi places

**3c. Per-Place Effect Annotation:**
- After range computation, set `Place.effect` (Read, Mutate, Capture, Store) for backward compatibility with downstream passes (reactive scope inference)

**3d. Function Effect Inference:**
- For FE bodies, compute externally-visible effects (mutations of params/context/returns)
- Store as `fn.aliasingEffects` for use by caller analysis

**Critical interaction with `effective_range`:** Our current `effective_range = max(mutable_range.end, last_use + 1)` approximation in `infer_reactive_scope_variables.rs` was needed because our old mutable ranges were too narrow. With the new model producing correct ranges, we should be able to use `mutable_range` directly — **which would fix the 4x failed attempt to switch**.

---

### Port Phase 4: Update AnalyseFunctions & Pipeline

**Effort:** 1-2 sessions
**Risk:** MEDIUM
**Files:** `src/inference/analyse_functions.rs`, `src/entrypoint/pipeline.rs`

**4a. Update `AnalyseFunctions`:**
Upstream's `AnalyseFunctions` now recursively calls a full sub-pipeline for each nested FE:
1. `InferMutationAliasingEffects` (new Phase 2)
2. `DeadCodeElimination`
3. `InferMutationAliasingRanges` (new Phase 3)
4. `RewriteInstructionKinds`
5. `InferReactiveScopeVariables`

This is different from our current approach where `AnalyseFunctions` only does basic function analysis.

**4b. Update Pipeline Pass Ordering:**
Current pipeline needs reordering to match upstream:
```
AnalyseFunctions (with recursive sub-pipeline)
InferMutationAliasingEffects (top-level)
[SSR optimization]
DeadCodeElimination
PruneMaybeThrows
InferMutationAliasingRanges (top-level)
[validations]
InferReactivePlaces
...
```

**4c. Remove `validate_no_mutation_after_freeze.rs`:**
Frozen mutation validation is now integrated into `InferMutationAliasingEffects` (emits `MutateFrozen` effects) and `InferMutationAliasingRanges` (records errors). Our standalone validator becomes redundant.

**4d. Try switching to `mutable_range` instead of `effective_range`:**
With correct mutable ranges from the new inference, attempt removing the `effective_range` approximation. This has failed 4 times with the old model but should work with the new one.

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

### `effective_range` vs `mutable_range` — WILL CHANGE AFTER PORT
File: `src/reactive_scopes/infer_reactive_scope_variables.rs`

Currently uses `effective_range = max(mutable_range.end, last_use + 1)` because old mutable ranges are too narrow. 4 prior attempts to switch to `mutable_range` failed (96%→36% render). After Phase 3 (new ranges), retry switching to `mutable_range` — the new model should produce correct ranges.

### `collect_all_scope_declarations` is load-bearing
File: `src/reactive_scopes/codegen.rs`

Pre-declares ALL scope output variables at function level. Removing it causes render to drop 96%→24%.

### Block iteration order ≠ source order for loops
The HIR blocks are stored in creation order, but for-loop constructs create blocks out of source order. The frozen mutation validator uses `frozen_at` instruction ID tracking. After Phase 4c (removing the standalone validator), this is handled by the new inference.

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
