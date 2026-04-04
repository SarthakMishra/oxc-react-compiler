# oxc-react-compiler Backlog

> Last updated: 2026-04-03
> Conformance: **550/1717 (32.0%)** (known-failures.txt has 1167 non-comment entries). Render: **92% (23/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Latest session gains: tN dep resolution BREAKTHROUGH (approach #10). Skipped unnamed SSA temporaries in propagate_scope_dependencies_hir Phase 2 else-branch. Check 2 bails: 154 -> 0 (ELIMINATED). Check 1 bails: 0 -> 194 (correct). Slot accuracy: +3 (235 -> 238 MATCH). Conformance unchanged at 550/1717. 94 preserve-memo false bails now ALL from Check 1 (scope not completed), not Check 2 (dep mismatch). Blocker shifted from "tN dep resolution" to "scope inference for preserve-memo fixtures".
> Known-failures: 1167. False-positive bails: ~168 (83 preserve-memo now ALL Check 1, 14 frozen-mutation, 9 reassign, 7 silent, 7 ref-access, 7 context-variable, 7 setState-in-effect, 5 MethodCall codegen, rest misc).
> WE-COMPILE-THEY-DON'T: ~88 (69 scope-surplus with no upstream error, ~9 "Found 1 error" bail-outs remaining, 10 Flow parse errors). Down from 94 after Session 2 fixes.
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
| Scope dep resolution (unnamed skip) | 94 preserve-memo false bails (Check 1) | +20-40 | **PARTIALLY RESOLVED** — Approach #10 (skip unnamed SSA temps in Phase 2 else-branch) ELIMINATED Check 2 bails (154 -> 0). Remaining 94 preserve-memo bails are ALL Check 1 (scope not completed). Blocker is now SCOPE INFERENCE: our compiler creates/keeps scopes that upstream prunes. See [Deferred/Blocked](#scope-dep-resolution-ssa-temp---named-variable-mapping----partially-resolved). |
| Scope inference fixes (slots-DIFFER) | ~572 | +50-100 | HIGH risk — cascading regression, scope MERGING is bottleneck |
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
- [ ] **Stage 2i: Preserve-memo false-positive bails (94 fixtures, all Check 1)** — PARTIALLY UNBLOCKED. tN dep resolution solved (approach #10, Check 2 eliminated). Remaining 94 bails are Check 1 (scope not completed) — our scope inference keeps scopes that upstream prunes. Fix is now in SCOPE INFERENCE, not dep resolution. See [bail-out-investigation.md](bail-out-investigation.md).
- [ ] **Stage 2h: Replan** — Categorize remaining bail-outs after all other fixes.
- [ ] **Stage 2g residual** — setState-in-render (4), setState-in-effect (2), hooks (3), exhaustive-deps (1), silent (7), other (~10).

---

### Stage 3: Scope Inference -- INVESTIGATED, BLOCKED on mutable range accuracy

**Pool:** ~572 slot-differ fixtures (deficit + surplus). Single largest category.
**Root cause:** Scope MERGING is the bottleneck (not scope creation). `last_use_map` in `infer_mutation_aliasing_ranges.rs` extends ranges wider than upstream. Cannot remove without receiver mutation effects + reverse scope propagation.

**Investigation complete (Stages 3a, 3a2):** Full categorization done. 134 zero-slot surplus fixtures confirmed as scope CREATION problem. 3 pruning approaches failed (-44, unresolved refs, confirmed not-pruning-problem).

**6 approaches tried for scope inference, ALL net-negative:** heuristic removal (-5), operand liveness blanket (-24), operand liveness targeted (-17), 0-slot codegen (-51), non-reactive dep pruning (-107), per-function reactive guard (-44).

**Prerequisites for progress:**
1. Fix mutable range accuracy: receiver mutation effects for MethodCall, reverse scope propagation
2. Audit `dep.reactive` flag assignment against upstream semantics
3. Understand scope merging algorithm in detail before attempting changes

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
| Check 1 "value was memoized" | 94 bails | IMPLEMENTED but fires on 94 fixtures. Root cause: scope inference keeps scopes upstream prunes. Fix is in scope inference. |
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

**20 of 29 done.** Groups A (+10, try-catch value blocks), B (+3, optional chains), C (+4, computed keys), F-partial (+2, PruneHoistedContexts), H (+2, hook-destructure-before-use), E (+2, mutable function mutations) = 23 COMPLETE. ~9 remaining.

| Group | Fixtures | Status |
|-------|----------|--------|
| A: Try-catch value blocks | 10 | COMPLETE (+10) |
| B: Optional terminal | 3 | COMPLETE (+3) |
| C: Computed keys | 4 | COMPLETE (+4) |
| D: Frozen mutation / value modification | 3 | 1 BLOCKED (useFreeze aliasing), 2 remaining |
| E: Post-render mutation | 2 | COMPLETE (+2) |
| F: Context/hoisting | 4 | 2 COMPLETE (PruneHoistedContexts +2), 1 BLOCKED (nested HIR LoadContext), 1 remaining (unnamed temporary) |
| G: Preserve-memo | 2 | BLOCKED by scope dep resolution |
| H: Hoisting access-before-declare | 2 | COMPLETE (+2) |

**Remaining tractable (low risk):** Group D simple (2 fixtures), Group F unnamed-temporary (1 fixture). **Blocked:** Group D-aliasing (1), Group F-context (1), Group G (2).

---

### Stage 5: "Both No Memo" -- DCE/CP CEILING REACHED

**Total gain: +7 fixtures (Stages 5a+5b).** DCE, phi-node CP, and dead branch elimination all implemented. Remaining ~90 fixtures blocked by 0-slot codegen (scope inference creates scopes where upstream doesn't). Further DCE/CP has diminishing returns.

- [ ] Binary operator folding, string concatenation (LOW priority, minimal gain expected)

---

## Active Work

- [~] **Preserve-memo Check 1 scope inference** — [bail-out-investigation.md](bail-out-investigation.md)#preserve-memo-check-1-scope-inference -- tN dep resolution SOLVED (approach #10, unnamed skip). 94 preserve-memo bails remain, ALL Check 1 (scope not completed). Root cause: our scope inference creates/keeps scopes where upstream prunes them. Next step: investigate which scopes upstream prunes and why ours don't.

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

#### Remaining Problem: Check 1 Scope Inference

94 preserve-memo fixtures bail on Check 1 ("value was not memoized" / scope not completed). Root cause: our scope inference creates reactive scopes for memo values where upstream either (a) prunes the scope, (b) merges it differently, or (c) never creates it. The fix is in scope inference, not dep resolution.

**Potential approaches:**
1. Investigate which specific scopes upstream prunes that we keep — compare `InferReactiveScopeVariables.ts` scope creation logic
2. Check if `PruneNonReactiveDependencies` or `PruneUnusedScopes` upstream prunes scopes that our equivalents don't
3. May overlap with Stage 3 scope inference work (mutable range accuracy, scope merging)

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

1. **`effective_range` is load-bearing.** 6 attempts to switch to `mutable_range` all regressed. File: `infer_reactive_scope_variables.rs`.
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

1. **Scope dep resolution** — PARTIALLY RESOLVED. Approach #10 (skip unnamed SSA temps in Phase 2 else-branch) eliminated Check 2 dep-mismatch bails entirely (154 -> 0). The remaining 94 preserve-memo false bails are ALL Check 1 (scope not completed). The blocker has shifted to SCOPE INFERENCE: our compiler creates/keeps scopes where upstream prunes them. This overlaps with Stage 3 scope inference work. See [blocker details](#scope-dep-resolution-ssa-temp---named-variable-mapping----partially-resolved).

2. **Scope inference accuracy** — `last_use_map` in `infer_mutation_aliasing_ranges.rs` produces wider ranges than upstream, causing over-merging and surplus scopes. This is the root cause behind ~572 slot-differ, ~90 both-no-memo (0-slot codegen), and ~69 we-compile-they-don't surplus fixtures. 6+ approaches have been tried and all regressed. The prerequisites are: (a) receiver mutation effects for MethodCall, (b) reverse scope propagation, (c) `dep.reactive` flag audit.

Every remaining conformance gain of significant size (>10 fixtures) depends on one or both of these. The only exception is the ~9 remaining "Found 1 error" bail-outs in Stage 4f, which are safe incremental gains.

**Key invariants discovered through failure:**
- `effective_range` cannot be replaced with `mutable_range` (6 attempts, all regressed)
- `collect_all_scope_declarations` is load-bearing for render (96%->24% without it)
- Pre-validation DCE must preserve StoreLocal/DeclareLocal for validators
- Skip/filter approaches to preserve-memo bails that filter by NAME pattern are fundamentally flawed (approaches 2-6, 9; worst -56). However, filtering by ABSENCE of name (approach #10, unnamed SSA temps) WORKS — key distinction is structural (pre-naming) vs pattern (post-naming). Check 2 is now eliminated; remaining problem is Check 1 (scope inference).
- 0-slot codegen via IR reconstruction is architecturally wrong (-52 twice); must use source-text editing
- Pruning cannot fix scope merging problems; the fix must be in scope creation/merging itself
