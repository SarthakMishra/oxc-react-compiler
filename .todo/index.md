# oxc-react-compiler — Remaining Work

> **Conformance: 555/1717 (32.3%)** | Known failures: 1162 | 0 panics | 0 unexpected divergences
> Last updated: 2026-04-05

---

## Failure Breakdown (1162 divergences)

| Category | Count | % | Description |
|----------|------:|---|-------------|
| Both compile, slots DIFFER | 565 | 49% | We compile, they compile, but different scope/slot counts |
| Both compile, slots MATCH | 229 | 20% | Same slots but codegen token diffs (naming, structure) |
| We bail, they compile | 199 | 17% | False-positive bail-outs |
| Both no memo (format diff) | 98 | 8% | Neither side memoizes, format differs |
| We compile, they don't | 71 | 6% | We compile, upstream bails with error |

---

## Grouped Remaining Tasks (by root cause)

### Group A: Scope Inference — Mutable Range Accuracy (CRITICAL PATH)

**Affects: ~565 slot-DIFFER + 94 preserve-memo + 98 both-no-memo + 71 we-compile-they-don't**
**Potential: +100-200 fixtures**
**Status: Investigation complete, implementation needed**

The single largest blocker. Our scope inference creates too many or too few scopes compared to upstream because mutable ranges are slightly inaccurate. The `effective_range = max(mutable_range.end, last_use + 1)` workaround compensates for narrow ranges but causes over-merging.

#### What we know (from 10 approaches + 2 investigations):

1. **Pipeline is structurally correct** — effects pass (Pass 16) → ranges pass (Pass 20) → scope inference all match upstream architecture. Apply effects ARE pre-resolved.
2. **Individual scope inference fixes cascade** — tested `mutableRange.start > 0` guard (-2 regression), PropertyStore allocating (+1/-1 neutral), spread Destructure (regression). Each fix disturbs compensating errors.
3. **Correct fix order confirmed:** (1) Fix mutable range accuracy → (2) Switch to `use_mutable_range=true` → (3) Apply scope inference fixes
4. **Conservative unknown-call fallback is load-bearing** — MutateTransitiveConditionally on all operands + O(n²) cross-arg Capture produces wide ranges needed for 10 fixtures. Cannot relax without introducing equal deficit.

#### Three sub-categories of +1 surplus (52 fixtures, Phase 176):

| Sub-category | ~Count | Root Cause | Status |
|-------------|--------|-----------|--------|
| Loop codegen bugs | ~12 | do-while flattened, for-of body dropped | **MOSTLY FIXED** (Phase 177) — remaining: SSA/DCE, self-assign, do-while+continue |
| Sentinel dep scopes | ~10 | Deps on never-changing sentinel values | **FIXED** (`prune_scopes_with_sentinel_only_deps`) |
| Redundant declarations | ~30 | Aliases (y=x) stored as separate slots | Open |

#### Next steps:

- [ ] **A1: Fixture-driven mutable range debugging** — Pick 3-5 simple +1 surplus fixtures (NOT loop-related), add debug logging to `infer_mutation_aliasing_ranges.rs`, compare ranges with upstream, identify specific divergence patterns. Key question: is divergence from (a) effect resolution, (b) cross-arg Capture, (c) receiver mutation, or (d) something else?
- [ ] **A2: Fix identified range divergences** — Targeted fixes based on A1 findings
- [ ] **A3: Switch to `use_mutable_range=true`** — Currently -40 regression. Retry after A2 narrows the gap.
- [ ] **A4: Apply scope inference fixes** — `mutableRange.start > 0` guard, remove `any_reactive && any_mutable` pre-filter, PropertyStore/ComputedStore allocating, spread Destructure allocating
- [ ] **A5: Redundant declaration deduplication** — Aliases stored as separate slots (~30 fixtures)

**Key files:** `infer_mutation_aliasing_ranges.rs`, `infer_mutation_aliasing_effects.rs`, `infer_reactive_scope_variables.rs`
**Upstream:** `InferMutationAliasingRanges.ts`, `InferMutationAliasingEffects.ts`, `InferReactiveScopeVariables.ts`

#### ⚠️ What NOT to do (learned from 10 failed approaches):
- Do NOT make blanket model changes (relaxing mutation model, removing heuristics). They shift the balance without fixing root cause.
- Do NOT filter deps by name pattern (post-naming). Only structural filters (pre-naming, `name == None`) are safe.
- Do NOT re-enable scope propagation to FinishMemoize.decl until scope accuracy improves (-52 regression).
- Do NOT attempt individual scope inference fixes without fixing mutable ranges first (cascading regressions).

---

### Group B: Preserve-Memo False Bails (94 fixtures)

**Bail error: "Existing memoization could not be preserved"**
**Status: BLOCKED by Group A**

All 94 bails are Check 1 ("value was not memoized" / scope not completed). Check 2 (dep mismatch) was ELIMINATED by approach #10 (skip unnamed SSA temps).

**Infrastructure is READY:** Scope propagation to `FinishMemoize.decl` tested — reduces bails 94→14 (-80!) but causes -52 conformance regression from scope surplus. The code works; it just needs accurate scope inference underneath.

**Dependency chain:** Group A scope accuracy → re-enable scope propagation → correct Check 1 → up to +80 fixtures

**History (10 approaches to dep resolution):**

| # | Approach | Result | Lesson |
|---|----------|--------|--------|
| 1-9 | Various skip/filter strategies | -4 to -56 | Post-naming filters catch both false and true positives |
| **10** | **Skip unnamed SSA temps (name==None)** | **+3 slot, 0 conf** | **Pre-naming structural filter works. Check 2 eliminated.** |

---

### Group C: Frozen-Mutation False Bails (14 fixtures)

**Bail error: "This value cannot be modified"**
**Status: BLOCKED by aliasing pass**

Our aliasing pass over-propagates freeze status through capture chains. When a mutable container `y` captures frozen data via `y.x = x`, `y` becomes MaybeFrozen. Then `mutate(y)` triggers MutateFrozen — a false positive.

**3 approaches tried, all failed:**
1. IIFE detection improvement (no effect on false positives)
2. IIFE skip in Check 1 (wrong source of false positive)
3. Cross-check MutateFrozen vs frozen_ids (-2 regression, lost true positives)

**Root cause:** `infer_mutation_aliasing_effects.rs` `mutate()` upgrades Mutate→MutateFrozen based on transitive freeze status. Fix must distinguish "container holds frozen data" from "container IS frozen" — either in the aliasing pass or the validator.

**Do NOT attempt until:** Aliasing pass freeze propagation semantics are better understood via line-by-line comparison with upstream `InferMutationAliasingEffects.ts`.

---

### Group D: Context Variable / Reassignment Bails (16 fixtures)

**Bail errors: "Cannot reassign variables" (9) + "Local variable reassigned" (7)**

#### D1: Cannot reassign outside component (9 fixtures)
**BLOCKED.** Requires DeclareContext/StoreContext HIR lowering for context variables. Previous attempt: -4 net regression.

#### D2: Local variable reassignment in render (7 fixtures)
These are context variables reassigned inside closures. Our validation fires when upstream doesn't because upstream models context variable semantics more precisely. Related to nested HIR builders not emitting LoadContext/StoreContext.

**Both sub-groups share the same root cause:** Our HIR builder's `build_arrow` doesn't call `setup_context_variables`, so nested functions use LoadLocal/StoreLocal instead of LoadContext/StoreContext. This prevents proper context variable modeling.

---

### Group E: Loop Codegen Bugs (~12 fixtures)

**Status: MOSTLY FIXED (Phase 177) — root cause found and resolved**
**Independent of scope inference**

**Root cause found:** `build_scope_block_only` silently dropped all loop terminals. Fixed in Phase 177.

**What was fixed:**
- While and DoWhile now emit proper HIR terminals instead of Goto+Branch
- Codegen emits proper `while(cond)`, `do{}while(cond)`, `for(init;cond;update)` syntax
- For-of and for-in loops now appear in output when inside reactive scopes

**Remaining issues (not codegen bugs — deeper pipeline issues):**
- [ ] **E4: For-loop update expression lost to DCE/SSA** — `i++` PostfixUpdate gets eliminated because the new SSA version appears unused (phi elimination issue). This is an SSA/DCE bug, not codegen.
- [ ] **E5: Self-assignments after loops** — Scope declarations re-assign to themselves (`ret = ret;`, `x = x;`). Codegen artifact from scope variable handling.
- [ ] **E6: Do-while with continue** — Falls back to `while(true)` instead of emitting proper condition

**Conformance:** Still 555/1717 (32.3%) — loop fixtures have other differences preventing exact match, but loops now actually appear in output.

**Affected fixtures (sample):** do-while-simple, do-while-continue, for-of-simple, for-of-continue, for-of-mutate, for-in-statement-break, for-in-statement-continue, alias-while, reactive-control-dependency-*-while

---

### Group F: Silent / Miscellaneous Bails (9 + 27 fixtures)

#### F1: Silent bails — no error message (9 fixtures)
| Fixture | Root Cause |
|---------|-----------|
| `babel-existing-react-runtime-import.js` | Import merging needed |
| `infer-functions-component-with-ref-arg.js` | Infer mode: function with ref arg not detected |
| `unused-object-element-with-rest.js` | 0 scopes survive pipeline |
| `invalid-jsx-in-catch-in-outer-try-with-catch.js` | try-catch HIR lowering |
| `invalid-jsx-in-try-with-catch.js` | try-catch HIR lowering |
| `valid-set-state-in-useEffect-from-ref.js` | setState-in-effect validation |
| `valid-setState-in-effect-from-ref-arithmetic.js` | setState-in-effect validation |
| `capturing-reference-changes-type.js` | Unknown |
| `gating/infer-function-expression-React-memo-gating.js` | Gating not detected |

#### F2: Other bail categories (27 fixtures combined)
| Error | Count | Notes |
|-------|------:|-------|
| setState in useEffect | 7 | Need to distinguish direct vs indirect setState, ref-sourced values |
| Cannot access refs in render | 7 | False positives: ref-typed values not properly excluded |
| MethodCall codegen internal error | 5 | `MethodCall::property must be unquoted` — codegen limitation for computed methods |
| Cannot call setState during render | 4 | False positives on conditional lambda setState |
| Cannot modify locals after render | 4 | Overlap with Group D reassignment issues |
| Hooks as normal values | 3 | PropertyLoad callee-name vector not checked |
| Exhaustive deps | 3 | Missing/extra dep detection gaps |
| Other (1-2 each) | ~10 | DefaultParam, NestedDestructuring, PruneHoistedContexts, etc. |

---

### Group G: "We Compile, They Don't" (71 fixtures)

Upstream bails with an error, but we compile through. Fixing = adding missing validation.

| Sub-group | Count | Status |
|-----------|------:|--------|
| No upstream error header (scope surplus) | 58 | BLOCKED by Group A — our surplus scopes produce output where upstream doesn't |
| "Found 1 error" bails | 5 | 1 blocked (unnamed-temp invariant), 2 blocked (infrastructure), 2 blocked (new-mutability model) |
| Flow parse errors | 2 | `.flow.js` files need Flow parser support |
| Ref mutation patterns | 4 | Various ref-in-hook patterns |
| Other | 2 | Misc |

---

### Group H: "Both No Memo" Format Diffs (98 fixtures)

Neither side memoizes, but our output format differs. Dominated by:
- **0-slot codegen** — We create scopes where upstream doesn't → produce `_c(N)` cache vs no cache. BLOCKED by Group A scope surplus.
- **DCE/CP gaps** — Some dead code not eliminated. DCE/CP ceiling mostly reached (+7 from Stages 5a/5b).
- **Variable naming** — Different temp names, scope variable naming

---

## Completed Work (condensed)

**Total gain: +105 fixtures across all stages (450→555)**

| Stage | Gain | Summary |
|-------|-----:|---------|
| 1: Codegen structure | +26 | Temp renumbering, scope placement, gating directives |
| 2: Bail-out fixes | +21 | File-level bails, `_exp` handling, error sweep, infer mode |
| 3: Scope inference | +4 | is_mutable hook exclusion (+3), sentinel dep pruning (+1 slot match) |
| 4: Validation gaps | +47 | Preserve-memo, todo errors, frozen mutations, context vars |
| 5: DCE/CP | +7 | Dead code elimination, constant propagation |

---

## Critical Architecture Notes

1. **`effective_range` is load-bearing.** 6 attempts to switch to `mutable_range` all regressed. The workaround compensates for narrow mutable ranges.
2. **Conservative unknown-call fallback is load-bearing for 10 fixtures.** Any relaxation causes identical -10 slot shift.
3. **Nested HIR builders don't emit LoadContext.** Context variables appear as LoadLocal in nested functions. Affects Groups D, validation accuracy.
4. **Pre-validation DCE must not remove StoreLocal/DeclareLocal.** Validators at Pass 21-32 depend on them.
5. **Block iteration order != source order for loops.** Affects freeze ordering checks (Check 6 needs `frozen_at` verification).
6. **Scope propagation to FinishMemoize.decl causes -52 if scope accuracy is wrong.** Do NOT re-enable until Group A is resolved.
7. **Post-naming dep filters are ALWAYS dangerous.** Only pre-naming structural filters (name==None) are safe. 9 failed approaches prove this.
8. **Individual scope inference fixes cascade.** Each fix disturbs compensating errors in the effective_range workaround. Must fix mutable range accuracy FIRST.
9. **+1 surplus has 3 distinct categories** (loop codegen ~12 MOSTLY FIXED, sentinel deps ~10 FIXED, redundant decls ~30). Not all are scope inference.

---

## Key File Reference

| Purpose | Path (relative to `crates/oxc_react_compiler/`) |
|---------|------|
| Pipeline orchestration | `src/entrypoint/pipeline.rs` |
| HIR types | `src/hir/types.rs` |
| HIR builder | `src/hir/build.rs` |
| Code generation | `src/reactive_scopes/codegen.rs` |
| Aliasing effects | `src/inference/aliasing_effects.rs` |
| Mutation effects (abstract interp) | `src/inference/infer_mutation_aliasing_effects.rs` |
| Mutation ranges (graph BFS) | `src/inference/infer_mutation_aliasing_ranges.rs` |
| Scope grouping (union-find) | `src/reactive_scopes/infer_reactive_scope_variables.rs` |
| Scope dependencies | `src/reactive_scopes/propagate_dependencies.rs` |
| Scope pruning + promotion | `src/reactive_scopes/prune_scopes.rs` |
| Scope merging | `src/reactive_scopes/merge_scopes.rs` |
| Frozen mutation validation | `src/validation/validate_no_mutation_after_freeze.rs` |
| Preserve-memo validation | `src/validation/validate_preserved_manual_memoization.rs` |
| Conformance test runner | `tests/conformance_tests.rs` |
| Known failures | `tests/conformance/known-failures.txt` |
