# Memoization Structure Differences

> **Priority**: P1 (largest category -- 904 fixtures, 64% of all failures)
> **Impact**: 904 divergences where both compilers produce `_c()` but structure differs
> **Tractability**: LOW per-item, HIGH aggregate -- items are interdependent; no single fix moves the needle alone

## Problem Statement

When both our compiler and Babel memoize a function, our output differs structurally. The 904 fixtures break down into:

| Sub-category | Count | Root cause |
|-------------|-------|------------|
| Over-scoped (too many cache slots) | ~400 | Globals/stable values treated as reactive deps |
| ~~Sentinel pattern never emitted~~ | ~~280~~ | ~~RESOLVED -- sentinel scopes now emitted~~ |
| Under-scoped (too few cache slots) | ~90 | Missing scopes for some expressions |
| Same slots, wrong deps | ~37 | Dependency tracking diverges (property-path resolution now active) |
| Other structural | ~94 | Temp variable naming, code ordering |
| Sentinel regressions (temporary) | +35 | Scopes correct, deps/slots still wrong |

The structural issues compound: a fixture may have wrong temp variables AND wrong slot counts AND missing sentinel scopes. Fixing one in isolation typically gains zero fixtures because the remaining issues still cause a mismatch.

## Files to Modify

### Temp Variable Inlining
- **`crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`** -- add inline pass or modify codegen to collapse SSA chains
- Potentially new file: **`crates/oxc_react_compiler/src/optimization/inline_temporaries.rs`** -- post-RF pass to inline trivial SSA chains

### JSX Preservation
- **`crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`** -- lines 325-348, modify JSX codegen to emit JSX syntax instead of `_jsx()` calls

### Scope/Dependency Analysis
- **`crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`** -- review merge heuristics vs upstream
- **`crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`** -- review prune decisions vs upstream
- **`crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`** -- reactive place inference
- **`crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs`** -- dependency tracking

## Implementation Plan

### Gap 1: Temp Variable Inlining Pass [IN PROGRESS]

**Upstream:** Babel's codegen never sees raw SSA temps -- its IR-to-code translation directly inlines simple expressions. The relevant upstream logic is spread across `CodegenReactiveFunction.ts` and `PrintReactiveFunction.ts`.
**Current state (updated 2026-03-13):** Recursive cross-scope temp use-counting has been implemented directly in `codegen.rs`. The codegen now walks nested `ReactiveTerminal::Scope` blocks when counting temp uses, so temps referenced only inside child scopes are correctly identified as single-use and inlined. All hash collections in codegen were migrated to `FxHashMap`/`FxHashSet` for performance. Conformance remains at 304/1717 -- this is a foundational fix that unblocks other P1 items rather than moving fixtures on its own.

**What remains:**
- The inlining logic itself is functional and correct for cross-scope cases
- Fixture gains will materialize when combined with other P1 fixes (JSX preservation, sentinel scopes, over-scoped deps) -- the interdependency noted in the risk section is the key blocker
- May need additional refinement now that JSX preservation (Gap 2) has landed, as JSX nodes create additional temp chains that are now exercised

**Implementation file:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
**Fixture gain estimate:** ~150-200 (compound effect with other P1 gaps; 0 in isolation)
**Depends on:** None (but gains depend on Gap 2 + Gap 5 + Gap 6)

### Gap 2: JSX Syntax Preservation in Codegen ✅

~~**Upstream:** `CodegenReactiveFunction.ts` emits JSX syntax directly (`<div>`, `<Component>`, `<>{...}</>`)~~
~~**Current state:** `codegen.rs` lines 325-348 emit `_jsx("div", { ... })` and `_jsxs(_Fragment, { children: [...] })` function call syntax~~

**Completed**: JSX syntax preservation fully implemented in `codegen.rs`. The `InstructionValue::JsxExpression` arm now emits proper JSX syntax (`<div>`, `<Component>`, `<>...</>`) instead of `_jsx()`/`_jsxs()` function calls. Self-closing vs open/close tags, spread props, string/expression children, and fragment shorthand all handled. The `react/jsx-runtime` import is removed from generated output; only `_c` from `react/compiler-runtime` remains. 23 snapshot files updated. Conformance unchanged at 304/1717 due to JSX normalization in the test harness. Implementation file: `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`.

### Gap 3: Cache Slot Count Alignment [IN PROGRESS]

**Upstream:** Babel counts cache slots based on reactive scope outputs + dependencies
**Current state (updated 2026-03-14):** Two major fixes applied:

1. **Sentinel slot count fix (2026-03-14, +7 fixtures):** `count_cache_slots` in `codegen.rs` now correctly handles sentinel scopes (0 deps) by counting `max(declarations, 1)` instead of `1 + declarations`. Sentinel scopes reuse the sentinel check slot as the first declaration slot, matching upstream's `getScopeCount`. Reactive scopes correctly count `deps + declarations` as separate slots.

2. **Declaration cache storage (2026-03-14):** `codegen_scope` now stores declarations into cache slots after deps for reactive scopes, and starting from `slot_start` for sentinel scopes. The else-branch reload uses the correct `decl_reload_start` offset. Previously, declarations were stored using a global `cache_slot` counter that could drift from the slot range used in the if-branch.

3. **Transitive dependency resolution (2026-03-14):** Phase 3 + Phase 3.5 in `propagate_dependencies.rs` correctly resolves derived variable deps to their root reactive inputs. Phase 3 tracks StoreLocal/StoreContext targets as scope declarations. Phase 3.5 runs a fixpoint substitution loop replacing transitive deps with root deps.

4. **Gap 11 resolved:** `prune_scopes.rs` no longer incorrectly prunes scopes whose declarations are consumed by other scopes.

Conformance: 342 -> 349/1717 (+7 from sentinel slot fix)

**What remains:**
- Remaining slot count divergences likely stem from scope merging differences (Sub-task 4f) and missing scopes for some expression types
- Edge cases where scope declaration sets differ from upstream (e.g., when a scope should declare a value but doesn't, or declares extra values)
**Fixture gain estimate:** Compound effect with other gaps
**Depends on:** Sub-task 4f (DeclarationId alignment, for shadowed variable edge cases)

### Gap 4: Scope Merging Architecture Rewrite

**Status:** IN PROGRESS (Sub-tasks 4a, 4b, 4c, 4d, 4e completed; 4f remaining)

This gap supersedes the previous "Scope Merging/Splitting Heuristic Review" and Gap 10
("Overlap Merge Regression"). Deep research into the upstream algorithm revealed that the
current implementation is fundamentally wrong in two places:

1. **`merge_overlapping_reactive_scopes_hir` (Pass 42)** uses a flat-range sort-and-merge,
   but upstream uses an active-scope-stack algorithm with cross-scope mutation tracking.
2. **`merge_reactive_scopes_that_invalidate_together` (post-conversion)** only merges
   consecutive scopes with identical deps, but upstream also handles output-to-input scope
   chaining and nested scope flattening.

Both must be rewritten to match upstream. The work is broken into 6 sub-tasks ordered
by dependency.

**Completed sub-items (from previous Gap 4 work):**
- Name-based dep comparison (`DepKey`) in `merge_reactive_scopes_that_invalidate_together`
- Double-merge prevention via `merged_indices`
- Dependency union and declaration merge on scope merge
- Non-reactive propagation through Destructure and CallExpression

**Reverted attempts (context for new plan):**
- Flat-range overlap merge: merged semantically separate scopes
- DSU rewrite: produced invalid JS (const scoping across blocks)
- setState non-reactive heuristic: false positives (resolved in Gap 9 via hook call exclusion)

#### Sub-task 4a: Active-scope-stack overlap detection (Pass 42) ✅

~~**Upstream:** `src/HIR/MergeOverlappingReactiveScopesHIR.ts` (note: in `src/HIR/`, NOT `src/ReactiveScopes/`)~~
~~**Current state:** Flat sort-and-merge in `merge_overlapping_reactive_scopes_hir()`~~

**Completed**: Full rewrite of `merge_overlapping_reactive_scopes_hir()` with active-scope-stack algorithm matching upstream `MergeOverlappingReactiveScopesHIR.ts`. Implementation includes:
- `DisjointSet<ScopeId>` (union-find with path compression and union-by-rank)
- 3-phase algorithm: (1) collect scope start/end maps and place-to-scope map, (2) walk instructions in ID order with active-scope stack to detect overlaps and cross-scope mutations, (3) rewrite scope annotations using merged representative scopes
- Cross-scope mutation tracking: when an instruction mutates an identifier belonging to a scope that is not at the top of the active stack, that scope and everything above it are merged
- Scopes with identical ranges are auto-merged when pushed onto the stack
- Ending scopes that are not at stack-top trigger merges with all scopes above them
- 1 regression: `error.invalid-prop-mutation-indirect.js` -- indirect prop mutation no longer detected after scope merging changes scope boundaries (added to known-failures.txt)
- Conformance: 343 -> 342/1717 (-1)
- Upstream file: `src/HIR/MergeOverlappingReactiveScopesHIR.ts`
- Implementation file: `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`

#### Sub-task 4b: Output-to-input scope chaining in invalidate-together ✅

~~**Upstream:** `MergeReactiveScopesThatInvalidateTogether.ts`~~
~~**Current state:** `merge_scopes_in_block` only merges consecutive scopes with identical dependency sets.~~

**Completed**: Output-to-input scope chaining implemented in `merge_scopes_in_block`, wiring together the safety-check infrastructure from Sub-task 4d and the eligibility predicate from Sub-task 4e. The merge logic now handles the "transitive invalidation" pattern: when scope A produces declarations that are dependencies of scope B, and A's outputs are always-invalidating types (Object, Array, Function, JSX), the scopes are merged -- provided only simple/pure instructions exist in the gap between them (validated by `IntermediateAccumulator` and `are_lvalues_last_used_by_scope`), and A passes the `scope_is_eligible_for_merging` check. This matches upstream's `MergeReactiveScopesThatInvalidateTogether.ts` algorithm.
- Upstream file: `src/ReactiveScopes/MergeReactiveScopesThatInvalidateTogether.ts`
- Implementation file: `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`

#### Sub-task 4c: Nested scope flattening ✅

~~**Upstream:** `MergeReactiveScopesThatInvalidateTogether.ts`~~
~~**Current state:** Not implemented~~

**Completed**: `flatten_nested_identical_scopes()` function implemented in `merge_scopes.rs`, matching upstream's nested-scope flattening in `MergeReactiveScopesThatInvalidateTogether.ts`. The algorithm detects when a `ReactiveScopeBlock`'s body consists of a single inner `ReactiveScopeBlock` with identical dependencies (compared via `dep_key_set` equality). When found, the inner scope is absorbed: its instructions, declarations, and merged IDs are transferred to the outer scope, and the inner scope wrapper is discarded. The function uses a `loop { ... if !changed { break; } }` pattern to handle multi-level nesting (e.g., outer/middle/inner all with identical deps) in successive passes. Wired as "Pass 1.5" in `merge_scopes_in_block`, after recursive descent into child terminals (Pass 1) and before the merge-plan walk (Pass 2), matching upstream ordering. Conformance unchanged (342/1717) -- this is a structural prerequisite for compound gains with Sub-tasks 4b and 4f.
- Upstream file: `src/ReactiveScopes/MergeReactiveScopesThatInvalidateTogether.ts`
- Implementation file: `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`

#### Sub-task 4d: Safety checks for intermediate instructions ✅

~~**Upstream:** `MergeReactiveScopesThatInvalidateTogether.ts`~~
~~**Current state:** Not implemented -- the current merge has no safety checks on instructions between scopes~~

**Completed**: Full safety-check infrastructure added to `merge_scopes.rs`. Implementation includes:
- `LastUsageMap` (`FxHashMap<IdentifierId, u32>`) built by `build_last_usage_map` / `collect_last_usage_in_block` / `collect_last_usage_in_terminal` -- a whole-function pre-pass mirroring upstream's `FindLastUsageVisitor` that records the maximum instruction ID at which each identifier is read
- `visit_instruction_read_places` -- exhaustive match over all `InstructionValue` variants to collect read operands (no catch-all arm, so new variants cause compile-time errors)
- `is_simple_instruction` predicate -- replicates upstream's allowlist: BinaryExpression, ComputedLoad, JSXText, LoadGlobal, LoadLocal, Primitive, PropertyLoad, TemplateLiteral, UnaryExpression
- `is_const_store_local` -- handles the StoreLocal(Const) special case allowed by upstream
- `IntermediateAccumulator` struct -- tracks lvalues written and LoadLocal aliases in the gap between two scope candidates
- `accumulate_intermediate_instruction` -- absorbs simple instructions into the accumulator
- `are_lvalues_last_used_by_scope` -- consults the `LastUsageMap` to verify no lvalue written in the gap is read after the merged scope boundary (the key safety invariant)
- `LastUsageMap` threaded through `merge_scopes_in_block` / `merge_scopes_in_terminal`
- This is purely additive infrastructure; the merge decision logic that calls these helpers is wired in Sub-task 4b. Conformance unchanged (342/1717).
- Upstream file: `src/ReactiveScopes/MergeReactiveScopesThatInvalidateTogether.ts`
- Implementation file: `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`

#### Sub-task 4e: `scopeIsEligibleForMerging` predicate ✅

~~**Upstream:** `MergeReactiveScopesThatInvalidateTogether.ts`~~
~~**Current state:** Not implemented~~

**Completed**: `scope_is_eligible_for_merging()` function implemented in `merge_scopes.rs`, matching upstream `scopeIsEligibleForMerging` from `MergeReactiveScopesThatInvalidateTogether.ts`. The predicate checks two conditions: (1) at least one scope declaration has an "always-invalidating" type (Object, Array, Function, JSX -- types that always create new references), and (2) the scope contains no reassignments to identifiers declared in a different scope. Currently marked `#[expect(dead_code)]` as the merge decision logic that calls it is wired in Sub-task 4b. Conformance unchanged (342/1717).
- Upstream file: `src/ReactiveScopes/MergeReactiveScopesThatInvalidateTogether.ts`
- Implementation file: `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`

#### Sub-task 4f: DeclarationId alignment for dependency comparison

**Upstream:** Uses `DeclarationId` (stable across SSA renaming) for dependency identity
**Current state:** Name-based `DepKey = (Option<String>, Vec<DependencyPathEntry>)` workaround
**What's needed:**

The name-based workaround works for most cases but fails when:
- Two different variables have the same name in nested scopes (shadowing)
- A variable is destructured and re-bound (same name, different declaration)
- The "intermediate lvalue last-used before boundary" check (Sub-task 4d) needs stable identity

Options:
1. **Add DeclarationId to Identifier** -- a stable ID assigned during declaration that persists
   through SSA renaming. This is the upstream approach and the most correct solution.
2. **Enhance DepKey with scope/block context** -- add the declaring block ID or scope depth to
   disambiguate same-named variables. Less invasive but less robust.

Option 1 is recommended but has high impact (touches HIR types, SSA pass, and all consumers).
Option 2 is a pragmatic interim step.

**Depends on:** None (improves correctness of all other sub-tasks)
**Risk:** HIGH -- changes to Identifier type ripple through the entire compiler
**Implementation files:** `crates/oxc_react_compiler/src/hir/types.rs`, `crates/oxc_react_compiler/src/hir/build.rs`, `crates/oxc_react_compiler/src/ssa/enter_ssa.rs`

### Gap 5: Sentinel Scope Emission ✅

~~**Upstream:** Babel creates reactive scopes for allocating expressions (JSX elements, object/array literals) even when they have no reactive dependencies. These scopes use the sentinel pattern (`Symbol.for("react.memo_cache_sentinel")`) instead of dependency checking.~~
~~**Current state:** `infer_reactive_scope_variables.rs` only creates scopes for reactive identifiers.~~

**Completed**: Sentinel scope emission is now active. `infer_reactive_scope_variables.rs` creates reactive scopes for allocating expressions (JSX elements, object/array literals) even when they have no reactive dependencies. `prune_scopes.rs` was updated to preserve these scopes. `codegen.rs` emits the sentinel pattern (`Symbol.for("react.memo_cache_sentinel")`) for scopes with zero reactive dependencies. Net conformance impact: -32 (35 regressions added to known-failures.txt, 3 newly passing). The regressions are expected -- the scopes are structurally correct but other P1 issues (over-scoped deps in Gap 6, slot counts in Gap 3) cause the overall output to still diverge. Implementation files: `infer_reactive_scope_variables.rs`, `prune_scopes.rs`, `codegen.rs`.

### Gap 6: Over-Scoped Dependencies ✅

~~**Upstream:** Babel correctly identifies global values (e.g., `Math.max`, `console.log`), stable hook returns (e.g., `setState` from `useState`), and other non-reactive values, and excludes them from dependency tracking.~~
~~**Current state:** We treat some globals and stable values as reactive, causing them to appear as dependencies in scopes. This results in more cache slots than needed (~400 fixtures).~~

**Completed**: Globals, stable hook returns (SetState, Ref), and property accesses of globals are no longer treated as reactive dependencies. Three files modified: `infer_types.rs` (type inference for stable hook returns), `infer_reactive_places.rs` (globals and stable values excluded from reactive marking), `propagate_dependencies.rs` (global property accesses filtered from dependency propagation). Conformance unchanged at 272/1717 -- gains expected to compound with remaining P1 fixes (Gap 3 slot counts, Gap 4 scope heuristics).

### Gap 7: Property-Path Dependency Resolution ✅

~~**Upstream:** `PropagateScopeDependencies.ts` uses `collectTemporaries()` to follow LoadLocal → PropertyLoad → ComputedLoad chains, resolving each SSA temporary to its root named variable + property path. Dependencies are emitted as e.g. `props.x` rather than just `props`.~~
~~**Current state:** `propagate_dependencies.rs` emitted dependencies using the raw SSA temp identifier, losing property path information.~~

**Completed**: Full property-path dependency resolution implemented in `propagate_dependencies.rs`. A `temp_map: FxHashMap<IdentifierId, TemporaryInfo>` is built in Phase 1.5, tracing LoadLocal/LoadContext → PropertyLoad chains to resolve SSA temps to `(root_identifier, property_path)`. The `collect_read_operand_places_for_deps` function uses this map to emit `ReactiveScopeDependency` with proper `DependencyPathEntry` paths. `codegen.rs` gained `dependency_display_name()` to render deps with dot-separated property paths (including optional chaining `?.`). Sentinel scope codegen was also fixed to store the first declaration value into the sentinel slot (previously sentinel scopes had no cache-store, causing re-computation every render). `DependencyPathEntry` gained `PartialEq, Eq` derives for deduplication. Conformance: 315 → 318/1717 (+3). Implementation files: `propagate_dependencies.rs`, `codegen.rs`, `types.rs`.

### Gap 8: Sentinel Scope Codegen Correctness ✅

~~**Upstream:** Sentinel scopes (zero reactive deps) store the first declaration value into cache slot 0 after computation, so subsequent renders skip re-computation via the sentinel check.~~
~~**Current state:** Sentinel scopes emitted the sentinel check but never stored anything into the cache slot, causing re-computation every render.~~

**Completed**: Fixed in `codegen.rs`. When `deps.is_empty()` (sentinel scope), the codegen now stores the first declaration value (`$[slot_start] = declName`) after the if-block body. This matches upstream behavior where sentinel scopes mark themselves as "computed" by writing a value to the sentinel slot. Part of the Gap 7 changeset.

### Gap 9: setState False Positive in Non-Reactive Propagation ✅

~~**Upstream:** N/A (this is a divergence-specific issue in our non-reactive propagation logic)~~
~~**Current state:** An attempt was made to treat `setState` calls as non-reactive (since setState itself is a stable function). This was reverted because it caused false positives.~~

**Completed**: Resolved by the hook call exclusion in the CallExpression non-reactivity rule. Instead of broadly treating setState calls as non-reactive, the fix narrows the rule: CallExpression results are only marked non-reactive when the callee is NOT a hook (name does not match `use[A-Z]`). This correctly handles `require('shared-runtime')` (non-reactive import, non-hook) while keeping `useState(0)`, `useContext(ctx)`, etc. as reactive (their return values are reactive state/context even though the hook function itself is a non-reactive import). The `id_to_name` map traces callee IDs back to their original variable names via LoadLocal/LoadGlobal instructions.
- Implementation file: `crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs`
- Part of the free variable detection changeset (349 -> 354/1717)

### Gap 10: Overlap Merge Regression -- SUPERSEDED by Gap 4

~~**Upstream:** `MergeOverlappingReactiveScopes.ts`~~
~~**Current state:** Two attempts have been made and reverted.~~

**Superseded**: This gap has been absorbed into the comprehensive Gap 4 rewrite plan.
Sub-task 4a covers the active-scope-stack overlap detection algorithm that replaces
both the flat-range merge and the reverted DSU attempt. The research revealed that
the const-scoping problem from the DSU attempt does not apply when the algorithm
runs at Pass 42 (before `build_reactive_scope_terminals_hir`), because scopes are
still just annotations on identifiers at that point, not block-structure modifications.
See Gap 4 Sub-task 4a for the full implementation plan.

### Gap 11: Derived Computation Codegen Outside Scope Guards ✅

~~**Upstream:** `CodegenReactiveFunction.ts` emits all scope declarations *inside* the scope guard's if-block.~~
~~**Current state:** Codegen emitted derived variable declarations outside the scope guard, defeating memoization for intermediate values.~~

**Completed**: Root cause was in `prune_scopes.rs`, not codegen. The `collect_used_outside_scopes` function had an `in_scope` flag that treated uses inside ANY scope block as "not escaping", causing derived computations (like `const doubled = value * 2`) to be pruned from their scope and emitted at the function body level. Fix: renamed to `collect_used_ids` and removed the `in_scope` gate entirely, matching upstream's `PruneNonEscapingScopes.ts` which collects all references without scope-awareness filtering. A variable declared in scope S1 and used inside scope S2 IS escaping S1 -- both scopes are independent cache boundaries.
- Upstream file: `src/ReactiveScopes/PruneNonEscapingScopes.ts`
- Implementation file: `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`

## Measurement Strategy

After each gap, run conformance and measure:
```bash
cargo test conformance -- --nocapture 2>&1 | tail -5
```

Expected progression (gaps are interdependent, so gains compound):
- Gap 1 (temp inlining) ✅ + Gap 2 (JSX) ✅ + Gap 5 (sentinel) ✅: structural foundation complete, 35 temporary regressions
- Gap 6 (over-scoped deps) ✅: globals/stable values excluded from deps
- Gap 7 (property-path deps) ✅ + Gap 8 (sentinel codegen) ✅: deps now emit `props.x` not just `props`, sentinel scopes store values correctly (+3 fixtures)
- After Sub-task 4a (active-scope-stack overlap) ✅: correct scope boundaries in HIR (-1 regression from indirect prop mutation)
- After Sub-tasks 4b-4e (invalidate-together rewrite) ✅: correct scope merging in ReactiveFunction (all complete)
- After transitive dep resolution ✅: Phase 3 StoreLocal declaration tracking + Phase 3.5 fixpoint substitution (slot counts closer, `component-with-derived` 5→4 slots)
- After Gap 11 (derived computation codegen) ✅: `collect_used_ids` fix in `prune_scopes.rs` -- declarations now kept inside scope guards
- After Gap 3 sentinel slot fix ✅: sentinel scopes use `max(decls, 1)` slots, reactive scopes use `deps + decls` (+7 fixtures, 342 -> 349)
- After free variable detection + hook call exclusion ✅: non-reactive free variables (module imports) excluded from deps, hook calls excluded from CallExpression non-reactivity (+5 fixtures, 349 -> 354)
- After Gap 3 remaining work: edge-case slot divergences from scope declaration set differences
- Sub-task 4f (DeclarationId): correctness improvement, may unlock edge-case fixtures
- Total potential from this category: ~400-600 new passes

## Risks and Notes

- **Interdependency is the key risk**: Previous experience shows that fixing one structural issue in isolation gains zero fixtures because the remaining issues still cause mismatches. Temp inlining (Gap 1), JSX preservation (Gap 2), sentinel scope emission (Gap 5), over-scoped deps (Gap 6), property-path deps (Gap 7), sentinel codegen (Gap 8), transitive dep resolution, Gap 11 (derived computation codegen), Gap 3 sentinel slot counting, Gap 9 (setState false-positive, resolved via hook call exclusion), and free variable detection are all complete. Scope merge sub-tasks 4a through 4e are done. The remaining blockers are: Gap 3 residual edge cases (scope declaration set differences), Sub-task 4f (DeclarationId alignment for shadowed variable correctness).
- **Scope merging is a 2-pass problem**: The overlap detection (Pass 42, Sub-task 4a) runs on the HIR BEFORE block structure is created. The invalidate-together merge (post-conversion, Sub-tasks 4b-4e) runs on the ReactiveFunction tree AFTER conversion. These are separate algorithms operating on different data structures at different pipeline stages. The reverted DSU attempt conflated them.
- **The const-scoping problem is a non-issue for Pass 42** ✅ CONFIRMED: The reverted DSU attempt failed because it was tested after block structure existed. But Pass 42 runs before `build_reactive_scope_terminals_hir` (Pass 43), so scopes are just annotations at that point. The Sub-task 4a rewrite confirmed this -- no const-scoping issues encountered.
- **Temp inlining correctness**: Must verify that inlined expressions maintain the same evaluation order. Only inline pure expressions or expressions where order doesn't matter.
- **JSX edge cases**: Self-closing elements, boolean attributes (`<div disabled />`), computed property names in JSX, namespace attributes (`xml:lang`).
- **Scope merging audit scope**: The merge/prune passes are among the most complex in the compiler. A full audit requires careful line-by-line comparison with upstream TypeScript.
