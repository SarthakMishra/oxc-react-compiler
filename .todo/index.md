# oxc-react-compiler — Remaining Work

> **Conformance: 559/1717 (32.6%)** | Known failures: 1158 | 0 panics | 0 unexpected divergences
> Last updated: 2026-04-05 (Phase 178)

---

## Failure Breakdown (1160 divergences)

| Category | Count | % | Description |
|----------|------:|---|-------------|
| Both compile, slots DIFFER | 565 | 49% | We compile, they compile, but different scope/slot counts |
| Both compile, slots MATCH | 227 | 20% | Same slots but codegen token diffs (naming, structure) |
| We bail, they compile | 199 | 17% | False-positive bail-outs |
| Both no memo (format diff) | 98 | 8% | Neither side memoizes, format differs |
| We compile, they don't | 71 | 6% | We compile, upstream bails with error |

### Slot Diff Distribution (565 slot-DIFFER fixtures)

| Diff | Count | Meaning |
|------|------:|---------|
| -1 | 146 | We under-split (over-merge) — too few scopes |
| -2 | 122 | |
| -3 | 59 | |
| -4 to -23 | 92 | |
| +1 | 54 | We over-split — too many scopes |
| +2 to +4 | 54 | |

**The dominant problem is under-splitting (over-merging): 419 fixtures have negative diff vs 108 positive.**

---

## Root Cause Analysis (Verified April 2026)

### What was DISPROVEN:

- ❌ **"Apply effects are skipped"** — Apply effects ARE fully resolved in Pass 16 (`infer_mutation_aliasing_effects.rs` lines 1035-1135). The `AliasingEffect::Apply { .. } => {}` skip in Pass 17 is a correct no-op matching upstream's invariant.
- ❌ **"Switching to `use_mutable_range=true` would fix it"** — Tested: **-40 regression** (557→517). Despite producing correct slot counts for individually tested fixtures, the net effect is strongly negative. The `effective_range` workaround is more load-bearing than previously understood.

### What IS verified:

1. **The `effective_range = max(mutable_range.end, last_use + 1)` workaround creates a coupled system.** It compensates for narrow `mutable_range` values by extending ranges. This over-extends some ranges (causing over-merging → too few scopes) while correctly extending others (preventing over-splitting). Both directions of error exist simultaneously.

2. **Three distinct root causes for slot differences:**

   | Root Cause | Direction | ~Count | Example |
   |-----------|-----------|--------|---------|
   | Missing reactive dependencies | both | ~200+ | Sentinel scope where upstream uses reactive dep check |
   | Over-declaring scope outputs | +N | ~50+ | Caching internal variables (`b`, `c` in while loop) |
   | Wrong scope boundaries from effective_range | -N | ~300+ | Over-merged scopes due to artificially widened ranges |

3. **Apply resolution quality matters.** While Apply IS resolved, our resolution may produce different concrete effects than upstream's. The conservative fallback (MutateTransitiveConditionally + O(n²) cross-arg Capture) is load-bearing for 10 fixtures. The resolution quality affects downstream range computation.

---

## Grouped Remaining Tasks

### Group A: Scope Inference Accuracy (CRITICAL PATH)

**Affects: ~565 slot-DIFFER + 94 preserve-memo bails**
**Status: Deep investigation complete, multiple sub-problems identified**

The scope inference problem is NOT a single bug. It's a coupled system where the `effective_range` workaround creates compensating errors. Three sub-problems must be addressed:

#### A1: Scope Output Over-Declaration (~50+ fixtures, +N surplus)

Variables used only INSIDE a scope body are incorrectly declared as scope outputs, adding extra cache slots. Example: in `alias-while.js`, `b` and `c` are internal to the while loop scope but get cached.

**Root cause:** `propagate_scope_dependencies_hir` or the scope declaration logic doesn't distinguish "used inside scope" from "used after scope".

**Files:** `propagate_dependencies.rs`, `infer_reactive_scope_variables.rs`

- [x] **A1.1:** Investigate scope output declaration logic ✅
- [x] **A1.2:** Compare with upstream — found operand_consumers scope-ID check is flawed ✅
- [x] **A1.3:** Fix: replaced with `last_use >= scope.range.end` check ✅ (+1 conformance, -18 in -1 deficit, -15 in +2 surplus)

#### A2: Missing Reactive Dependencies (~200+ fixtures, wrong dep detection)

We use sentinel scopes (one-time computation) where upstream uses reactive scopes (re-compute when deps change). This means we DON'T re-compute when reactive values change.

Example: `allocating-logical-expression-instruction-scope.ts` — we use sentinel scope, upstream depends on `data` from `useFragment()`.

**Root cause:** `propagate_scope_dependencies_hir` doesn't correctly identify reactive external dependencies, especially hook return values.

**Files:** `propagate_dependencies.rs`

- [x] **A2.1:** Investigated — Phase 2 only processes scoped instructions, missing loop test blocks ✅
- [x] **A2.2:** FIXED: Phase 2b collects deps from while/do-while test blocks using `collect_read_operand_places` (not `_for_deps` which skips LoadLocal). +1 conformance (558→559), 0 regressions ✅
- [ ] **A2.3:** Extend to for-loop conditions (currently excluded due to regressions) and hook return values

#### A3: Scope Boundary Accuracy (~300+ fixtures, effective_range over-merging)

The `effective_range = max(mutable_range.end, last_use + 1)` workaround in `infer_reactive_scope_variables.rs` causes over-merging. Variables with artificially extended ranges overlap and get unioned into the same scope.

**This cannot be fixed by just switching to `use_mutable_range=true`** (tested: -40 regression). The raw `mutable_range` values are too narrow for many fixtures.

**The gap is in range computation, not effect resolution.** Our BFS range propagation in `infer_mutation_aliasing_ranges.rs` produces narrower ranges than upstream for reasons not yet fully understood. Possible causes:
- Different graph edge construction
- Different BFS traversal order
- Missing implicit edges from Create/Capture chains

**Files:** `infer_mutation_aliasing_ranges.rs`, `infer_reactive_scope_variables.rs`

- [ ] **A3.1:** Line-by-line comparison of `InferMutationAliasingRanges.ts` with our `infer_mutation_aliasing_ranges.rs`
- [ ] **A3.2:** Identify specific range differences for 3-5 -1 fixtures
- [ ] **A3.3:** Fix range computation to produce wider ranges where needed
- [ ] **A3.4:** Re-test `use_mutable_range=true` after A3.3

#### ⚠️ What NOT to do (proven by 10+ failed attempts):
- Do NOT make blanket model changes (cascading regressions)
- Do NOT switch `use_mutable_range` without fixing range computation first
- Do NOT filter deps by name pattern (post-naming). Only structural filters work.
- Do NOT re-enable scope propagation to FinishMemoize.decl until scope accuracy improves (-52 regression)
- Do NOT attempt individual scope inference fixes without understanding the coupled system

---

### Group B: Preserve-Memo False Bails (94 fixtures)

**Bail error: "Existing memoization could not be preserved"**
**Status: BLOCKED by Group A**

Infrastructure is READY. Scope propagation to `FinishMemoize.decl` reduces bails 94→14 (-80) but causes -52 conformance regression from scope surplus. Needs accurate scope inference first.

**Dependency:** A1 + A2 → re-enable scope propagation → +80 fixtures

---

### Group C: Frozen-Mutation False Bails (14 fixtures)

**Status: BLOCKED by aliasing pass semantics**

Our aliasing pass over-propagates freeze through capture chains. `mutate()` upgrades to MutateFrozen based on transitive freeze. Needs "container holds frozen" vs "container IS frozen" distinction.

---

### Group D: Context Variable Bails (16 fixtures)

**Status: BLOCKED by HIR builder context variable support**

Nested function expressions use LoadLocal/StoreLocal instead of LoadContext/StoreContext. Affects 16 fixtures across reassignment and render-time mutation validation.

---

### Group E: Loop Codegen (MOSTLY FIXED — Phase 177)

**Status: Core issue fixed. 3 remaining sub-issues.**

- [x] **E4:** For-loop update expression preserved — DCE no longer removes PostfixUpdate/PrefixUpdate ✅
- [x] **E5:** Self-assignment stripping works globally ✅
- [ ] **E6:** Do-while with continue falls back to `while(true)`

---

### Group F: Miscellaneous Bails (36 fixtures)

| Error | Count | Difficulty |
|-------|------:|-----------|
| setState in useEffect | 7 | Medium (transitive conditional analysis) |
| Cannot access refs in render | 9 | Medium (ref type exclusion) |
| MethodCall codegen internal error | 5 | Hard (nested method call codegen) |
| Cannot call setState during render | 4 | Hard (conditional lambda tracking) |
| Hooks as normal values | 3 | Easy (PropertyLoad local object check) — but tested, causes regression |
| Other (1-2 each) | 8 | Various |

---

### Group G: "We Compile, They Don't" (71 fixtures)

58 of 71 are BLOCKED by Group A (scope surplus causes us to compile where upstream bails). Fixing scope accuracy would naturally fix most.

---

### Group H: "Both No Memo" Format Diffs (98 fixtures)

Dominated by DCE/constant-propagation gaps and variable naming differences. Low priority — doesn't affect memoization correctness.

---

## Recommended Priority Order

1. **A1 (scope output over-declaration)** — Most isolated sub-problem, likely quickest win
2. **A2 (missing reactive dependencies)** — High impact, may fix ~200 fixtures
3. **E4 (for-loop SSA/DCE)** — Independent of scope inference
4. **A3 (range computation accuracy)** — Hardest, requires line-by-line upstream comparison
5. **B (preserve-memo)** — Blocked until A1+A2 are resolved
6. **Everything else** — Diminishing returns

---

## Key File Reference

| Purpose | Path (relative to `crates/oxc_react_compiler/`) |
|---------|------|
| Pipeline orchestration | `src/entrypoint/pipeline.rs` |
| HIR types | `src/hir/types.rs` |
| HIR builder | `src/hir/build.rs` |
| Code generation | `src/reactive_scopes/codegen.rs` |
| Mutation effects (abstract interp) | `src/inference/infer_mutation_aliasing_effects.rs` |
| Mutation ranges (graph BFS) | `src/inference/infer_mutation_aliasing_ranges.rs` |
| Scope grouping (union-find) | `src/reactive_scopes/infer_reactive_scope_variables.rs` |
| Scope dependencies | `src/reactive_scopes/propagate_dependencies.rs` |
| Scope pruning + promotion | `src/reactive_scopes/prune_scopes.rs` |
| Scope merging | `src/reactive_scopes/merge_scopes.rs` |
| Environment config | `src/hir/environment.rs` |
| Conformance test runner | `tests/conformance_tests.rs` |
| Known failures | `tests/conformance/known-failures.txt` |
