# oxc-react-compiler — Remaining Work

> **Conformance: 567/1717 (33.0%)** | Known failures: 1150 | 0 panics | 0 unexpected divergences
> Last updated: 2026-04-05 (Phase 187 — PruneNonEscapingScopes force-memoize fix + DeclareLocal merge, +7 net)

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
- ❌ **"Switching to `use_mutable_range=true` would fix it"** — Tested: **-57 regression** (560→503) even after isMutable fix. Over-splitting dominates (+1 category: 66→145). Mutable ranges from `infer_mutation_aliasing_ranges` are still too narrow.
- ❌ **"Alternative grouping algorithms can replace the effective_range workaround"** — Tested 8+ variants (use-based grouping, selective trivial-range extension with thresholds 1-50, split grouping/scoping, fixed +1/+2 extensions). Best results achieve 560 but are functionally equivalent to `use_mutable_range=false`. The over-merging problem cannot be solved by tweaking the grouping algorithm — it requires matching upstream instruction IDs or implementing PruneNonEscapingScopes. See `.analysis/scope-grouping-algorithms.md`.

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

- [x] **A3.0:** Line-by-line comparison of `InferReactiveScopeVariables.ts` with our code — isMutable operand filter fix (+1, 559->560), 8 divergences documented in `.analysis/scope-variables-comparison.md` ✅
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

### Group I: O(n^2+) Performance Regression in Mutation/Aliasing Passes (HIGH PRIORITY)

**Affects: Real-world adoption — OXC is SLOWER than Babel on medium/large files**
**Status: Root causes identified, needs profiling confirmation and targeted fixes**

O(n^2+) scaling in `infer_mutation_aliasing_effects` (Pass 16) and `infer_mutation_aliasing_ranges` (Pass 20) causes OXC to run at 0.2x-0.7x Babel speed on medium/large files. Batch throughput is 0.5x Babel. Regression introduced in Phases 113-130.

#### I1: Pass 16 — `infer_mutation_aliasing_effects` (abstract interpretation fixpoint)

**Files:** `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_effects.rs`
**Upstream:** `src/Inference/InferMutationAliasingEffects.ts`

Root causes:
- Full `InferenceState` clone on every block visit (Vec + FxHashMap of FxHashSets) — 10,000+ clones for large functions
- Cascading re-queuing: single phi convergence triggers entire function re-traversal
- Full instruction re-processing per iteration (including expensive signature computation)
- Hard-coded iteration limit of 100 with no convergence acceleration
- No structural sharing, no widening operator, no worklist priority ordering

Sub-tasks:
- [x] **I1.1:** Profile Pass 16 on large fixtures (canvas-sidebar, booking-list, data-table) to confirm hotspots ✅
- [x] **I1.2:** In-place state merging, sorted block processing, reused buffers, lightweight phi ✅ (21-35% improvement)
- [x] **I1.3:** Worklist priority ordering (sorted by block index for forward convergence) ✅
- [x] **I1.4:** Verify zero test regressions after each optimization ✅ (560/1717 maintained)

#### I2: Pass 20 — `infer_mutation_aliasing_ranges` (graph BFS range propagation)

**Files:** `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs`
**Upstream:** `src/Inference/InferMutationAliasingRanges.ts`

Root causes:
- Independent BFS per mutation instead of batched graph reachability
- Linear edge filtering O(E) per node visit
- Two redundant full-HIR traversals (`annotate_place_effects` + `annotate_last_use`) that could be merged
- Instruction effects computed in Pass 16 then recomputed from scratch in Pass 20

Sub-tasks:
- [x] **I2.1:** Profile Pass 20 on large fixtures to confirm hotspots ✅
- [x] **I2.2a:** Reusable BFS buffers across mutations (clear instead of realloc) ✅
- [x] **I2.2b:** Pre-sized hash maps based on instruction count ✅
- [x] **I2.2c:** Visitor-based operand ID collection (avoid per-instruction Vec alloc) ✅
- [ ] **I2.2:** Batch mutation BFS — single reverse-reachability pass instead of per-mutation
- [ ] **I2.3:** Merge `annotate_place_effects` + `annotate_last_use` into single HIR traversal
- [ ] **I2.4:** Cache and reuse instruction effects between Pass 16 and Pass 20
- [x] **I2.5:** Verify zero test regressions after each optimization ✅ (560/1717 maintained)

#### Constraint

NO test regressions allowed. Each optimization must be verified against the full conformance suite before merging.

---

### Group J: Rust Data Structure Optimizations (HIGH PRIORITY)

**Affects: Compile-time performance across ALL passes — fundamental porting debt**
**Status: Investigation complete, implementation needed**

The TypeScript to Rust port preserved JS reference semantics as Rust clones. In JS, passing an object is a pointer copy. In Rust, each `.clone()` is a deep copy with heap allocations. These issues compound across all 65 passes.

#### J1: Replace `HashMap<IdentifierId, T>` with `Vec<T>` indexed by ID

17+ validation passes create `FxHashMap<IdentifierId, String>` for name lookups. Since IdentifierId is a sequential u32, a `Vec<Option<String>>` indexed by the raw u32 would be O(1) with no hashing, better cache locality, and lower memory overhead.

**Files:** All files in `src/validation/*.rs`, `src/inference/*.rs`

- [x] **J1.1:** Create `IdVec<Id, T>` and `IdSet<Id>` types in `types.rs` ✅
- [~] **J1.2:** Replace HashMap<IdentifierId, T> with IdVec in validation passes (10 of 17+ files done)
- [ ] **J1.3:** Replace HashMap<BlockId, T> with IdVec in inference passes
- [x] **J1.4:** Verify zero test regressions ✅ (560/1717 maintained)

#### J2: Replace owned `Place` in `AliasingEffect` with `IdentifierId`

The `AliasingEffect` enum stores full `Place` structs (88 bytes each, heap-allocated) in every variant. A CallExpression with N args generates N+3 effects, each cloning 2-3 Places. Storing `IdentifierId` (4 bytes, Copy) and resolving on demand would eliminate thousands of clones per function.

**Files:** `src/hir/types.rs` (AliasingEffect enum), `src/inference/infer_mutation_aliasing_effects.rs`

- [ ] **J2.1:** Audit all AliasingEffect consumers to determine minimal data needed
- [ ] **J2.2:** Replace Place with IdentifierId in AliasingEffect variants
- [ ] **J2.3:** Add resolution helper to look up Place from IdentifierId when needed
- [ ] **J2.4:** Verify zero test regressions

#### J3: Replace `String` with `oxc_span::Atom` for property names and identifiers

`Identifier.name`, `PropertyLoad.property`, `MethodCall.property`, `ObjectPropertyKey`, `JsxAttributeName` — all use owned `String` (24 bytes + heap alloc). The OXC ecosystem provides `Atom` (8 bytes, interned). Switching shrinks Identifier 72 to 56 bytes, Place 88 to 72 bytes, eliminates heap allocations for property names.

**Files:** `src/hir/types.rs`, `src/hir/build.rs` (60+ `.to_string()` calls)

- [ ] **J3.1:** Replace `Identifier.name: Option<String>` with `Option<Atom>`
- [ ] **J3.2:** Replace String property fields (PropertyLoad, MethodCall, etc.) with Atom
- [ ] **J3.3:** Update build.rs to use Atom instead of `.to_string()`
- [ ] **J3.4:** Verify zero test regressions

#### J4: Use `Rc<ReactiveScope>` instead of cloning per-identifier

In `infer_reactive_scope_variables.rs`, every identifier belonging to a scope gets `Some(Box::new(scope.clone()))`. If 50 identifiers share one scope, that's 50 deep clones of a struct containing Vec<Place>, Vec<ScopeId>, etc. Should use Rc for shared ownership.

**Files:** `src/reactive_scopes/infer_reactive_scope_variables.rs`, `src/hir/types.rs`

- [ ] **J4.1:** Change `Identifier.scope` from `Option<Box<ReactiveScope>>` to `Option<Rc<ReactiveScope>>`
- [ ] **J4.2:** Update scope writeback to use Rc::clone instead of deep clone
- [ ] **J4.3:** Verify zero test regressions

#### J5: Reduce Place cloning in HIR builder (45+ clone sites)

The HIR builder clones Place for every instruction operand. Place contains Identifier with Option<String> and Option<Box<ReactiveScope>>. In TypeScript these are cheap reference copies.

**Files:** `src/hir/build.rs`

- [ ] **J5.1:** Audit Place clone sites in build.rs — determine which can be eliminated
- [ ] **J5.2:** Restructure instruction building to avoid intermediate clones where possible
- [ ] **J5.3:** Verify zero test regressions

---

### Group K: Rust Pipeline Optimizations (MEDIUM PRIORITY)

**Affects: Compile-time performance — redundant work across passes**
**Status: Investigation complete, implementation needed**

#### K1: Consolidate id-to-name map rebuilding

Three consecutive passes (`infer_types`, `infer_reactive_places`, `infer_reactive_scope_variables`) each rebuild `FxHashMap<IdentifierId, String>` by scanning all instructions. Should compute once after SSA and thread through.

**Files:** `src/inference/infer_types.rs`, `src/inference/infer_reactive_places.rs`, `src/reactive_scopes/infer_reactive_scope_variables.rs`

- [ ] **K1.1:** Build id_to_name map once in an early pass
- [ ] **K1.2:** Thread it through subsequent passes as a parameter
- [ ] **K1.3:** Verify zero test regressions

#### K2: Replace naive O(N^2) fixpoint in scope membership propagation

`infer_reactive_scope_variables.rs` Phase 4 re-scans ALL instructions on each iteration. Should use a worklist of affected instructions.

**Files:** `src/reactive_scopes/infer_reactive_scope_variables.rs`

- [ ] **K2.1:** Implement worklist-based propagation instead of full re-scan
- [ ] **K2.2:** Verify zero test regressions

#### K3: Add SmallVec for common small collections

`CallExpression.args`, `JsxExpression.children`, `FinishMemoize.deps`, `Switch.cases`, `Sequence.blocks` — all use `Vec` but typically hold 1-4 elements. `SmallVec<[T; 4]>` eliminates heap allocation for the common case.

**Files:** `src/hir/types.rs`

- [ ] **K3.1:** Add SmallVec dependency
- [ ] **K3.2:** Replace Vec with SmallVec for identified fields
- [ ] **K3.3:** Verify zero test regressions and size assertions still pass

#### K4: Mark `Type` enum as `Copy`

The `Type` enum contains no heap data but is only `Clone`. Should be `Copy` to avoid explicit clone calls in type inference.

**Files:** `src/hir/types.rs`

- [ ] **K4.1:** Add `Copy` derive to `Type` and `PrimitiveType` enums
- [ ] **K4.2:** Remove unnecessary `.clone()` calls on Type values
- [ ] **K4.3:** Verify zero test regressions

#### K5: Eliminate `.collect::<Vec<_>>().join()` in codegen

At least 4 locations in codegen.rs collect into a temporary Vec just to call `.join()`. Should iterate directly.

**Files:** `src/reactive_scopes/codegen.rs`

- [ ] **K5.1:** Replace collect-then-join with direct iteration (lines ~2638, 2659, 3551)
- [ ] **K5.2:** Verify zero test regressions

#### K6: Worklist-based DCE instead of 8-iteration fixpoint

`pipeline.rs` runs constant propagation + DCE up to 8 times with full HIR scans. Should track affected instructions.

**Files:** `src/entrypoint/pipeline.rs`

- [ ] **K6.1:** Implement worklist tracking for DCE/constant propagation
- [ ] **K6.2:** Verify zero test regressions

---

## Recommended Priority Order

1. **A1 (scope output over-declaration)** — Most isolated sub-problem, likely quickest win
2. **I (O(n^2+) perf regression in Pass 16 + Pass 20)** — Blocks real-world adoption
3. **J (Rust data structure optimizations)** — Fundamental porting debt, compounds across all passes
4. **A2 (missing reactive dependencies)** — High impact, may fix ~200 fixtures
5. **E6 (do-while codegen)** — Independent of scope inference
6. **A3 (range computation accuracy)** — Hardest, requires line-by-line upstream comparison
7. **K (Rust pipeline optimizations)** — Medium priority, incremental improvements
8. **B (preserve-memo)** — Blocked until A1+A2 are resolved
9. **Everything else** — Diminishing returns

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
