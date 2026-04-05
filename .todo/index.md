# oxc-react-compiler Backlog

> Last updated: 2026-04-04
> Conformance: **555/1717 (32.3%)** (known-failures.txt has 1162 non-comment entries). Render: **92% (23/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Latest session gains: +2 from Stage 4f Group D simple — context-variable frozen-mutation detection (for-loop iterator reassignment) and update-expression-on-context-variable detection in nested lambdas (553->555).
> Known-failures: 1162. False-positive bails: ~168 (83 preserve-memo now ALL Check 1, 14 frozen-mutation, 9 reassign, 7 silent, 7 ref-access, 7 context-variable, 7 setState-in-effect, 5 MethodCall codegen, rest misc).
> WE-COMPILE-THEY-DON'T: ~86 (69 scope-surplus with no upstream error, ~7 "Found 1 error" bail-outs remaining, 10 Flow parse errors).
> Note: Conformance tests use `compilationMode:"all"` which affects how fixtures are tested (all functions compiled, not just components/hooks).

---

## Road to 600+ Conformance (550 -> 600+, need +50)

### Failure Category Summary (revised 2026-04-03, post-sessions)

| Category | Count | Description |
|----------|-------|-------------|
| Both compile, slots DIFFER | ~572 (49%) | Scope inference accuracy. Deficit (our < expected): -1 (140), -2 (108), -3+ (181). Surplus (our > expected): +1 (74), +2 (41), +3+ (28). **BLOCKED by Stage 3b.** |
| Both compile, slots MATCH | ~237 (20%) | Same slots, codegen structure diffs. Dominated by variable naming/scope inference. Codegen-only ceiling reached. |
| We bail, they compile | ~168 (14%) | False-positive bail-outs. 83 preserve-memo (BLOCKED), 14 frozen-mutation (BLOCKED), rest various. |
| We compile, they don't | ~88 (8%) | 69 scope-inference surplus, ~9 "Found 1 error" remaining, 10 Flow parse errors. |
| Both no memo (format diff) | ~90 (8%) | Neither side memoizes. Blocked by 0-slot codegen (scope inference). |

### Revised Path to 600+

| Work Item | Pool Size | Potential Gain | Status |
|-----------|-----------|---------------|--------|
| Scope dep resolution (unnamed skip) | 94 preserve-memo false bails (Check 1) | +20-80 | **INFRASTRUCTURE READY, BLOCKED by Stage 3.** Approach #10 eliminated Check 2 bails (154->0). Scope propagation to FinishMemoize.decl tested: bails 94->14 (-80!) but -52 conformance regression from scope surplus. Code is ready; needs Stage 3 scope inference accuracy first. See [Deferred/Blocked](#scope-dep-resolution-ssa-temp---named-variable-mapping----partially-resolved). |
| Scope inference fixes (slots-DIFFER) | ~572 | +50-100 | FIRST SUCCESS (+3 from is_mutable exclusion). 9 approaches tried, 1 positive. HIGH risk — cascading regression, scope MERGING is bottleneck |
| Stage 4f remaining "Found 1 error" bails | ~9 | +5-9 | LOW risk — bail-to-pass, zero regression |
| DCE + constant propagation remaining | ~90 | +5-15 | Blocked by 0-slot codegen (scope inference) |
| Variable name preservation (B2) | 40 | +10-20 | Scope-inference dependent, NOT codegen-only |
| Remaining bail-out fixes (2d-2g residual) | ~60 | +10-15 | Various blockers |

---

### Stage 1: Codegen Structure -- COMPLETE (all phases done or blocked by Stage 3)

**Total gain: +26 fixtures across 7 sub-stages (1a-1g).**
Key work: temp variable renumbering (+2), minor codegen fixes (+5), lazy scope declaration placement Phase 1 (+6), misc codegen/harness fixes (+6), follow-up bail refinements (+5), gating directive stripping (+2). Phases 1d-2 (declaration placement inside control flow) and B2 (variable name preservation) are BLOCKED by scope inference (Stage 3). The codegen-only ceiling has been reached.

**Remaining (blocked):**
- [ ] **Stage 1d Phase 2** — Move declarations inside control flow. BLOCKED by scope inference (Stage 3). 39 fixtures.

---

### Stage 2: False-Positive Bail-outs -- MOSTLY COMPLETE (remaining items blocked or low-priority)

**Total gain: +21 fixtures across sub-stages 2a-2j.**
Key work: file-level bail removal (+5), `_exp` directive handling (+0 net, 20 moved), error fixture sweep (+6), Infer mode heuristics (+4), misc validation fixes (+6).

**Remaining:**
- [ ] **Stage 2d: Frozen-mutation false positives (15 fixtures)** — BLOCKED. Root cause: transitive freeze propagation in aliasing pass. 3 approaches failed. See [bail-out-investigation.md](bail-out-investigation.md).
- [ ] **Stage 2f: Reassignment false positives (10 fixtures)** — BLOCKED. Requires DeclareContext/StoreContext HIR lowering. Attempt caused -4 net.
- [ ] **Stage 2i: Preserve-memo false-positive bails (94 fixtures, all Check 1)** — BLOCKED by Stage 3. Scope propagation to FinishMemoize.decl implemented and tested: bails 94->14 (-80!) but conformance -52 regression from error fixtures. Root cause: our scope inference creates scopes upstream doesn't — propagating them exposes the surplus. Code is READY, blocked by scope inference accuracy. See [bail-out-investigation.md](bail-out-investigation.md).
- [ ] **Stage 2h: Replan** — Categorize remaining bail-outs after all other fixes.
- [ ] **Stage 2g residual** — setState-in-render (4), setState-in-effect (2), hooks (3), exhaustive-deps (1), silent (7), other (~10).

---

### Stage 3: Scope Inference -- FIRST SUCCESS, deep investigation complete

**Pool:** ~572 slot-differ fixtures (deficit + surplus). Single largest category.
**Root cause:** Scope MERGING is the bottleneck (not scope creation). `last_use_map` in `infer_mutation_aliasing_ranges.rs` extends ranges wider than upstream. Cannot remove without receiver mutation effects + reverse scope propagation.

**Investigation complete (Stages 3a, 3a2):** Full categorization done. 134 zero-slot surplus fixtures confirmed as scope CREATION problem. 3 pruning approaches failed (-44, unresolved refs, confirmed not-pruning-problem).

**Deep investigation findings (2026-04-03):**

1. **Apply effects are NOT skipped** -- they are pre-resolved by `infer_mutation_aliasing_effects` (Pass 16) before `infer_mutation_aliasing_ranges` (Pass 20). The `Apply { .. } => {}` match arm in ranges.rs is CORRECT because all resolvable Apply effects have already been converted to concrete effects (Mutate, CreateFrom, Capture, etc.) by that point.

2. **Conservative fallback for unknown calls** produces extremely aggressive effects:
   - MutateTransitiveConditionally on ALL operands (receiver + args)
   - MaybeAlias from each operand to return value
   - O(n^2) cross-arg Capture (each arg captured into every other arg)
   This produces wide mutable ranges and is a major contributor to over-merging.

3. **Every mutation model change produces the same -10 slot shift** -- tested MutateConditionally (non-transitive), cross-arg capture removal, Apply processing, StoreLocal range extension. All cause identical 238->228 MATCH count shift. The aggressive mutation model is load-bearing for 10 specific fixtures. This is a confirmed tradeoff: more aggressive mutation -> more merging -> fewer slots (matches some, surplus for others); less aggressive -> less merging -> more slots (reduces surplus, creates deficit).

4. **Current balance is the best achievable** without upstream's exact algorithm for Apply resolution and mutation propagation. Further gains require reading `InferMutationAliasingRanges.ts` and `InferMutationAliasingEffects.ts` line-by-line for the precise logic.

**9 approaches tried for scope inference, 1 successful (+3):**
1. Heuristic removal (-5)
2. Operand liveness blanket (-24)
3. Operand liveness targeted (-17)
4. 0-slot codegen (-51)
5. Non-reactive dep pruning (-107)
6. Per-function reactive guard (-44)
7. **Exclude Call/MethodCall from is_mutable_instruction (+3, FIRST SUCCESS)** -- unknown-type values from calls should not unconditionally trigger scope creation
8. Non-transitive MethodCall mutation (neutral) -- changed MutateTransitiveConditionally to MutateConditionally for unknown methods; safer mutation model
9. Remove last_use>instr_id gate for is_allocating (-5, reverted) -- created more scopes than needed

Also tried: use_mutable_range=true (-40, reverted) -- mutable ranges alone still too narrow for scope creation.

**Prerequisites for further progress:**
1. Read upstream `InferMutationAliasingRanges.ts` and `InferMutationAliasingEffects.ts` line-by-line for the precise Apply resolution and mutation propagation logic
2. Fix mutable range accuracy: receiver mutation effects for MethodCall, reverse scope propagation
3. Audit `dep.reactive` flag assignment against upstream semantics
4. Understand scope merging algorithm in detail before attempting changes

**Remaining (all blocked by prerequisites):**
- [ ] **Stage 3b:** Fix dominant slot diff patterns. HIGH risk.
- [ ] **Stage 3c:** Fix secondary +/-1 patterns.
- [ ] **Stage 3d:** +/-2 slot diff quick wins.

**Upstream:** `InferReactiveScopeVariables.ts`, `InferMutationAliasingRanges.ts`, `PropagateScopeDependencies.ts`
**Our files:** `infer_reactive_scope_variables.rs`, `infer_mutation_aliasing_ranges.rs`, `propagate_dependencies.rs`

---

### Stage 4: Validation Gaps -- MOSTLY COMPLETE

#### Stage 4b: Preserve-memo validation (32 fixtures target)

**4 of 32 passing.** Check 1 (scope completion tracking) implemented (+1). Check 2 (validateInferredDep) fully functional — tN dep resolution solved (approach #10 eliminated all Check 2 bails). 94 preserve-memo bails remain, ALL from Check 1 (scope not completed). Blocked by scope inference: our compiler creates scopes where upstream prunes them.

| Sub-type | Count | Status |
|----------|-------|--------|
| Check 1 "value was memoized" | 94 bails (14 with scope propagation) | IMPLEMENTED. Scope propagation to FinishMemoize.decl tested: 94->14 bails (-80). But -52 conformance regression (error fixtures pass Check 1 incorrectly due to scope surplus). Code READY, BLOCKED by Stage 3 scope inference accuracy. |
| Check 2 validateInferredDep | 0 bails | RESOLVED. Approach #10 (unnamed SSA temp skip) eliminated all 154 Check 2 bails. |
| Check 3 "dependency may be mutated" | 17 | Not started |

- [ ] Add "dependency may be mutated" tracking (17 fixtures)

#### Stage 4c: Todo error detection -- MOSTLY COMPLETE (+15 net)

22/27 done. **4 remaining:** hoisting patterns (2), optional terminal (1), context var update (1, BLOCKED by nested HIR LoadContext gap).

#### Stage 4d: Frozen-mutation false negatives -- COMPLETE (+10 net)

1 remaining: `error.invalid-jsx-captures-context-variable.js` (JSX capture analysis).

#### Stage 4e: UPSTREAM ERROR fixture handling -- MOSTLY COMPLETE

**Previous gains: +18 total** across sub-stages 4e-A through 4e-D.

**Remaining:**
- [ ] **4e-B residual:** 1 fixture remaining (no new infrastructure needed)
- [ ] **4e-C:** 2 frozen-mutation fixtures (JSX capture + 1 other)
- [ ] **4e-D2:** 8 preserve-memo fixtures (BLOCKED by scope dep resolution)
- [ ] **4e-E:** 2 fixtures — optional-chain-in-ternary (1), context var update (1, BLOCKED)

#### Stage 4f: "Found 1 error" Bail-Out Sweep -- MOSTLY COMPLETE

**25 of 29 done (+25 fixtures).** Groups A (+10, try-catch value blocks), B (+3, optional chains), C (+4, computed keys), D-simple (+2, context variable mutations), E (+2, mutable function mutations), F-partial (+2, PruneHoistedContexts), H (+2, hook-destructure-before-use) = 25 COMPLETE. 4 remaining (1 blocked-by-restructure, 3 blocked-by-infrastructure).

| Group | Fixtures | Status |
|-------|----------|--------|
| A: Try-catch value blocks | 10 | COMPLETE (+10) |
| B: Optional terminal | 3 | COMPLETE (+3) |
| C: Computed keys | 4 | COMPLETE (+4) |
| D: Frozen mutation / value modification | 3 | 2 COMPLETE (+2, context variable mutations), 1 BLOCKED (useFreeze aliasing) |
| E: Post-render mutation | 2 | COMPLETE (+2) |
| F: Context/hoisting | 4 | 2 COMPLETE (PruneHoistedContexts +2), 1 BLOCKED (nested HIR LoadContext), 1 BLOCKED (promote_used_temporaries restructure needed) |
| G: Preserve-memo | 2 | BLOCKED by scope dep resolution |
| H: Hoisting access-before-declare | 2 | COMPLETE (+2) |

**Remaining tractable (low risk, 3 fixtures):**
- Group D simple #1: `error.todo-for-loop-with-context-variable-iterator.js` -- Upstream: "Error: This value cannot be modified" (for-loop iterator `i` is a context variable reassigned in updater, modification after JSX use). Requires detecting context-variable modification after JSX capture.
- Group D simple #2: `error.todo-handle-update-context-identifiers.js` -- Upstream: "Todo: Handle UpdateExpression to variables captured within lambdas" (`counter++` inside arrow function). Requires detecting UpdateExpression on context variables in nested lambdas during HIR lowering.
- Group F unnamed-temporary: `error.bug-invariant-unnamed-temporary.js` -- Upstream: "Invariant: Expected temporaries to be promoted to named identifiers in an earlier pass" (rest params `...props` in nested arrow produces unnamed identifier 15). Requires adding a `promoteTemporary`-style invariant check after `promote_used_temporaries` pass.

**Blocked (4 fixtures):** Group D-aliasing: `new-mutability/error.mutate-frozen-value.js` (needs `useFreeze` + `@enableNewMutationAliasingModel`). Group F-context: `error.invalid-jsx-captures-context-variable.js` (needs `@enableNewMutationAliasingModel` + nested HIR LoadContext). Group G: `error.repro-preserve-memoization-inner-destructured-value-mistaken-as-dependency-later-mutation.js` and `error.repro-preserve-memoization-inner-destructured-value-mistaken-as-dependency-mutated-dep.js` (BLOCKED by scope dep resolution / preserve-memo Check 1).

---

### Stage 5: "Both No Memo" -- DCE/CP CEILING REACHED

**Total gain: +7 fixtures (Stages 5a+5b).** DCE, phi-node CP, and dead branch elimination all implemented. Remaining ~90 fixtures blocked by 0-slot codegen (scope inference creates scopes where upstream doesn't). Further DCE/CP has diminishing returns.

- [ ] Binary operator folding, string concatenation (LOW priority, minimal gain expected)

---

## Active Work

- [~] **Preserve-memo Check 1 scope inference** — [bail-out-investigation.md](bail-out-investigation.md)#preserve-memo-check-1-scope-inference -- Scope propagation to FinishMemoize.decl TESTED: bails 94->14 (-80!) but -52 conformance regression. Infrastructure is READY but BLOCKED by Stage 3 scope inference accuracy. The 52 regressed fixtures are error fixtures where our surplus scopes cause Check 1 to incorrectly pass. Dependency chain: Stage 3 scope accuracy -> scope propagation -> correct Check 1 -> up to +80 preserve-memo fixtures.

---

## Deferred / Blocked Work

### Scope Dep Resolution (SSA temp -> named variable mapping) -- PARTIALLY RESOLVED

**Affects:** Stage 2i (94 preserve-memo false bails, now ALL Check 1), Stage 4b validateInferredDep, B2 variable name preservation.
**Status:** tN dep resolution SOLVED at dep-collection level (approach #10). Check 2 bails eliminated (154 -> 0). Remaining problem is scope inference (Check 1).

#### Breakthrough: Approach #10 — Skip unnamed SSA temps in Phase 2 else-branch

**What was done:** In `propagate_scope_dependencies_hir` Phase 2's else-branch, skip operands where `name == None` (unnamed SSA temporaries). These are computation results that have no source-level name and should not be added as scope deps.

**Results:**
- Conformance: 550/1717 (unchanged, zero regression)
- Slot accuracy: +3 fixtures moved from DIFFER to MATCH (238 vs 235 baseline)
- Check 2 (dep mismatch) bails: 154 -> **0** (COMPLETELY ELIMINATED)
- Check 1 (scope not completed) bails: 0 -> 194 (correct behavior — these scopes genuinely aren't completed)
- Preserve-memo false bail count: still 94 fixtures, but ALL from Check 1 instead of Check 2

**Why this works:** Unnamed SSA temporaries are computation results (StoreLocal targets, CallExpression outputs, etc.) that get promoted to "tN" names by `promote_used_temporaries`. By skipping them at collection time, scope deps only contain named variables — matching upstream behavior. This is the FIRST net-positive infrastructure change across 10 approaches.

**What changed:** The blocker has shifted from "tN dep resolution" to "scope inference for preserve-memo fixtures". The 94 remaining bails fire Check 1 because our scope inference creates/keeps scopes that upstream prunes. The fix is in scope creation/pruning, not dep resolution.

#### 10 Approaches History

| # | Approach | Result | Why |
|---|----------|--------|-----|
| 1 | Build temp map before inline_load_local_temps | 0 | Same entries regardless of timing |
| 2 | Skip unnamed deps in propagation | -15 | Removes real deps needed downstream |
| 3 | Skip "tN" in resolve_scope_dep | -31 | Error fixtures lose bail path |
| 4 | Skip "tN" in validateInferredDep | -56 | Broader filtering, worse regression |
| 5 | tN dep skip in propagate_dependencies.rs | -55 | Changes codegen slots globally |
| 6 | Synthetic tN skip in resolve_scope_dep | -55 | Same as 3 at different level |
| 7 | MethodCall check removal | -4 | Lost true-positive bails |
| 8 | Receiver-only MethodCall check | -4 | Wrong operand targeted |
| 9 | Defining-operands backward trace | 0 | Semantically wrong (traced root is computation INPUT, not user's dep) |
| **10** | **Skip unnamed SSA temps (name == None) in Phase 2 else-branch** | **+3 slot accuracy, 0 conformance change** | **FIRST net-positive. Check 2 eliminated. Correct approach.** |

**Key distinction:** Approaches 2-6 failed because they filtered by NAME pattern (after naming). Approach 10 works because it filters by ABSENCE of name (before naming) — structurally different. Unnamed operands are computation results that should never have been deps in the first place.

#### Remaining Problem: Check 1 Scope Inference — BLOCKED by Stage 3

94 preserve-memo fixtures bail on Check 1 ("value was not memoized" / scope not completed). Root cause: our scope inference creates reactive scopes for memo values where upstream either (a) prunes the scope, (b) merges it differently, or (c) never creates it. The fix is in scope inference, not dep resolution.

**Scope propagation to FinishMemoize.decl — tested and validated (2026-04-03):**
- In `infer_reactive_scope_variables.rs` Phase 5, propagated scopes to `FinishMemoize.decl` and deps places
- Result: bails 94 -> 14 (-80!), but conformance 550 -> 498 (-52 regression)
- The -52 comes from error fixtures where our surplus scopes cause Check 1 to incorrectly pass
- **Code is CORRECT and READY** — just needs accurate scope inference underneath it
- **Do NOT re-enable until Stage 3 scope inference accuracy improves**

**Dependency chain (confirmed by experiment):**
1. **Stage 3** — fix scope over-creation/over-merging (mutable range accuracy, scope merging algorithm)
2. **Re-enable scope propagation to FinishMemoize.decl** (infrastructure ready)
3. **Correct Check 1 behavior** — up to +80 preserve-memo fixtures become passing

This confirms that Stage 3 scope inference is the critical path for BOTH the ~572 slot-differ fixtures AND the 94 preserve-memo false bails. The two problems share the same root cause: scope surplus.

#### Long-term: Port ReactiveScopeDependency type

The full upstream solution uses a `ReactiveScopeDependency` type with property access paths. This may still be needed for full upstream fidelity, but the IMMEDIATE blocker (Check 2 tN deps) is resolved. The remaining Check 1 problem is orthogonal to dep resolution.

### Other Blocked Items

- **Stage 2d: Frozen-mutation false positives** — BLOCKED by aliasing pass transitive freeze propagation
- **Stage 2f: Reassignment false positives** — BLOCKED by DeclareContext/StoreContext HIR lowering
- **Phase 4c: Remove standalone freeze validator** — BLOCKED (hook-call-freezes-captures logic needed)
- **Phase 4d: Switch to mutable_range** — 6 failed attempts, over-splitting regressions
- **Phase 5: Fault tolerance** — BLOCKED until 600+ conformance
- **0-Slot codegen** — BLOCKED (-52 regression). Requires scope inference accuracy to reduce surplus
- **Performance O(n^2+)** — Deferred until correctness stabilizes

---

## Critical Architecture Notes

1. **`effective_range` is load-bearing.** 6 attempts to switch to `mutable_range` all regressed. File: `infer_reactive_scope_variables.rs`. Root cause confirmed: mutable ranges are computed by `infer_mutation_aliasing_ranges` which skips already-resolved Apply effects (correct behavior -- Apply effects are pre-resolved by Pass 16). The conservative unknown-call fallback produces wide ranges via MutateTransitiveConditionally on all operands + O(n^2) cross-arg Capture. This aggressive model is load-bearing for 10 fixtures; any relaxation causes identical -10 shift.
2. **`collect_all_scope_declarations` cannot be removed.** Prevents render collapse 96%->24%. File: `codegen.rs`.
3. **Scope dep IdentifierIds are SSA temps, not source names.** Blocks validateInferredDep (29), B2, and any scope-dep-to-source-name resolution.
4. **Nested HIR builders don't emit LoadContext.** Context variables appear as LoadLocal in nested functions.
5. **Pre-validation DCE must not remove StoreLocal/DeclareLocal.** Validators at Pass 21-32 depend on them. Extended DCE at Pass 32.5 only.
6. **Block iteration order != source order for loops.** HIR blocks in creation order; for-loops create blocks out of source order.

### Build & Test

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
| Hooks usage validation | `src/validation/validate_hooks_usage.rs` |
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
| Fault tolerance | `facebook/react#35872` through `#35888` |
| Exhaustive deps validation | `facebook/react#34394` |

---

## Architecture Lessons

The project has reached a clear inflection point. All low-risk, codegen-only, and bail-out-only gains have been exhausted. The remaining path to 600+ is dominated by two infrastructure problems:

1. **Scope dep resolution + preserve-memo Check 1** — Check 2 RESOLVED (approach #10). Check 1 infrastructure READY: scope propagation to FinishMemoize.decl reduces bails 94->14 (-80!) but causes -52 regression from scope surplus. This CONFIRMS that Stage 3 scope inference is the single critical path for both preserve-memo (+80 potential) and slot-differ (+50-100 potential). See [blocker details](#scope-dep-resolution-ssa-temp---named-variable-mapping----partially-resolved).

2. **Scope inference accuracy** — `last_use_map` in `infer_mutation_aliasing_ranges.rs` produces wider ranges than upstream, causing over-merging and surplus scopes. This is the root cause behind ~572 slot-differ, ~90 both-no-memo (0-slot codegen), and ~69 we-compile-they-don't surplus fixtures. 9 approaches tried: 1 successful (+3 from excluding Call/MethodCall from is_mutable_instruction), rest regressed or neutral. Deep investigation (2026-04-03) confirmed that the aggressive unknown-call mutation model (MutateTransitiveConditionally + O(n^2) cross-arg Capture) is load-bearing for 10 fixtures and cannot be relaxed without introducing equivalent deficit. Current balance is best achievable without porting upstream's exact Apply resolution and mutation propagation from `InferMutationAliasingEffects.ts` / `InferMutationAliasingRanges.ts`.

Every remaining conformance gain of significant size (>10 fixtures) depends on one or both of these. The only exception is the ~9 remaining "Found 1 error" bail-outs in Stage 4f, which are safe incremental gains.

**Key invariants discovered through failure:**
- `effective_range` cannot be replaced with `mutable_range` (6 attempts, all regressed)
- `collect_all_scope_declarations` is load-bearing for render (96%->24% without it)
- Pre-validation DCE must preserve StoreLocal/DeclareLocal for validators
- Skip/filter approaches to preserve-memo bails that filter by NAME pattern are fundamentally flawed (approaches 2-6, 9; worst -56). However, filtering by ABSENCE of name (approach #10, unnamed SSA temps) WORKS — key distinction is structural (pre-naming) vs pattern (post-naming). Check 2 is now eliminated; remaining problem is Check 1 (scope inference).
- 0-slot codegen via IR reconstruction is architecturally wrong (-52 twice); must use source-text editing
- Pruning cannot fix scope merging problems; the fix must be in scope creation/merging itself
- Scope propagation to FinishMemoize.decl is correct infrastructure but premature without Stage 3: reduces bails 94->14 but causes -52 from error fixtures that incorrectly pass Check 1 due to scope surplus. Stage 3 scope accuracy is the single gating prerequisite for both preserve-memo and slot-differ gains.
- Apply effects in `infer_mutation_aliasing_ranges` are correctly skipped -- they are pre-resolved to concrete effects (Mutate, CreateFrom, Capture, etc.) by `infer_mutation_aliasing_effects` (Pass 16) before ranges (Pass 20) runs. The `Apply { .. } => {}` arm is NOT a bug.
- The conservative unknown-call mutation model is a balanced tradeoff: every attempted relaxation (non-transitive mutation, cross-arg capture removal, Apply processing, StoreLocal range extension) causes an identical -10 slot MATCH shift. The 10 affected fixtures are load-bearing for the current conformance count. Further scope inference gains require porting upstream's EXACT mutation resolution logic, not tuning the conservative fallback.
