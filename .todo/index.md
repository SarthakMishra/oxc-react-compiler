# oxc-react-compiler Backlog

> Last updated: 2026-04-03
> Conformance: **550/1717 (32.0%)** (known-failures.txt has 1167 non-comment entries). Render: **92% (23/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Latest session gains: +1 from Check 1 scope completion tracking in validate_preserved_manual_memoization.rs (549->550). tN dep resolution thoroughly investigated (4 additional approaches, all net-negative).
> Known-failures: 1167. False-positive bails: ~168 (83 preserve-memo, 14 frozen-mutation, 9 reassign, 7 silent, 7 ref-access, 7 context-variable, 7 setState-in-effect, 5 MethodCall codegen, rest misc).
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
| Scope dep resolution (port ReactiveScopeDependency) | 51 preserve-memo false bails + 28 validateInferredDep | +20-40 | **TOP PRIORITY** — 8 skip/filter approaches failed. Must port upstream `ReactiveScopeDependency` type with full access paths. Significant refactor. |
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
- [ ] **Stage 2i: Preserve-memo false-positive bails (55 fixtures)** — DEFINITIVELY BLOCKED. 8 approaches tried across 3 sessions, all net-negative (worst: -56). ALL skip/filter approaches are fundamentally flawed. Only fix: port upstream `ReactiveScopeDependency` type with full access paths. See blocker reports in [bail-out-investigation.md](bail-out-investigation.md).
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

**4 of 32 passing.** Check 1 (scope completion tracking) implemented (+1). validateInferredDep (Check 2) ported correctly. 28 remaining BLOCKED by scope dep resolution (SSA temp IdentifierIds don't resolve to named variables). See Deferred/Blocked section.

| Sub-type | Count | Status |
|----------|-------|--------|
| Check 1 "value was memoized" | 17 | IMPLEMENTED. +1 conformance from scope completion tracking. Remaining gains blocked by scope dep resolution (Check 2 fires first on most fixtures). |
| Check 2 validateInferredDep | 26 | 4 done, 28 BLOCKED by scope dep resolution |
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

- [~] **tN dep resolution (Part B of combined fix)** — [bail-out-investigation.md](bail-out-investigation.md)#combined-check-1--tn-dep-fix -- Check 1 DONE (+1), Part B BLOCKED: 4 additional approaches all net-negative. See blocker report.

---

## Deferred / Blocked Work

### Scope Dep Resolution (SSA temp -> named variable mapping) -- DEFINITIVELY BLOCKED

**Affects:** Stage 2i (51 false-positive bails), Stage 4b validateInferredDep (28 fixtures), B2 variable name preservation.
**Problem:** After SSA, scope dependency IdentifierIds point to temporaries, not original named variables. `propagate_dependencies.rs` does not preserve the original dependency path.

**Key finding (2026-04-03):** ALL 76 preserve-memo false bails are Check 2 (validateInferredDep), caused by synthetic tN-named deps. 55 of those are load-bearing (error fixtures that bail "by accident" via tN mismatch). The root cause is computation-result temps (CallExpression, MethodCall, BinaryExpression, Destructure outputs) that cannot be traced to named variables through the temp map.

**8 total approaches tried, ALL net-negative:**
1. Build temp resolution map before inline_load_local_temps: no effect (0)
2. Skip unnamed deps in propagate_scope_dependencies_hir: -15
3. Skip "tN" names in resolve_scope_dep validation: -31
4. Skip "tN" names in validateInferredDep comparison: -56
5. tN dep skip in propagate_dependencies.rs (this session): -55
6. Synthetic tN name skip in resolve_scope_dep (this session): -55
7. MethodCall check removal (this session): -4
8. Receiver-only MethodCall check (this session): -4

**Check 1 (scope completion tracking) implemented** but neutral (+1 only). It does NOT provide the alternative bail path that was hoped for — error fixtures still need Check 2's tN mismatch to bail correctly.

**Only viable path forward:** Port upstream's richer `ReactiveScopeDependency` type which includes the full property access path (not just IdentifierId). This is a significant refactor of `propagate_dependencies.rs` and its consumers.

**Previous resolution options (all failed or superseded):**
1. ~~Skip/filter approaches~~ -- ALL 4 variants tried, all net-negative, fundamentally flawed
2. ~~Check 1 as alternative bail path~~ -- Implemented but insufficient, error fixtures still need Check 2
3. Enhance `propagate_dependencies.rs` to carry original dependency path -- NOT YET ATTEMPTED, most promising
4. Build post-SSA reverse mapping pass -- NOT YET ATTEMPTED
5. Port upstream's richer `ReactiveScopeDependency` type -- NOT YET ATTEMPTED, likely the correct long-term fix

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

1. **Scope dep resolution** — SSA temporaries obscure original variable identities. This blocks preserve-memo validation (51 false bails + 29 error fixtures = 80 fixtures affected), B2 variable naming, and any feature needing source-level dep names. The fix is in `propagate_dependencies.rs` — it must preserve original property access paths through SSA.

2. **Scope inference accuracy** — `last_use_map` in `infer_mutation_aliasing_ranges.rs` produces wider ranges than upstream, causing over-merging and surplus scopes. This is the root cause behind ~572 slot-differ, ~90 both-no-memo (0-slot codegen), and ~69 we-compile-they-don't surplus fixtures. 6+ approaches have been tried and all regressed. The prerequisites are: (a) receiver mutation effects for MethodCall, (b) reverse scope propagation, (c) `dep.reactive` flag audit.

Every remaining conformance gain of significant size (>10 fixtures) depends on one or both of these. The only exception is the ~9 remaining "Found 1 error" bail-outs in Stage 4f, which are safe incremental gains.

**Key invariants discovered through failure:**
- `effective_range` cannot be replaced with `mutable_range` (6 attempts, all regressed)
- `collect_all_scope_declarations` is load-bearing for render (96%->24% without it)
- Pre-validation DCE must preserve StoreLocal/DeclareLocal for validators
- ALL skip/filter approaches to preserve-memo bails are fundamentally flawed (8 attempts across 3 sessions, worst -56)
- 0-slot codegen via IR reconstruction is architecturally wrong (-52 twice); must use source-text editing
- Pruning cannot fix scope merging problems; the fix must be in scope creation/merging itself
