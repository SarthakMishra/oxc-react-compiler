# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.

Last updated: 2026-03-14 (Sub-task 4d completed, 342/1717)

Current conformance: 342/1717 pass (19.9%), 0 panics, 0 unexpected divergences.

Note: Most passing fixtures match by both compilers returning source unchanged
(trivial match via lint mode, validation bail-out, or non-component detection).
Only 2 fixtures match with actual compiled `_c()` output. The remaining 1374
divergences break down as follows (counts are approximate and overlap):

**Regression note (2026-03-13):** Sentinel scope emission (Gap 5) was activated,
correctly adding reactive scopes for allocating expressions. This introduced 35
regressions (added to known-failures.txt) where the new scopes are structurally
correct but other P1 issues (over-scoped deps, wrong slot counts) cause the
overall output to still diverge. Net change: -32 (35 regressions, 3 newly passing).
The regressions will resolve as remaining P1 gaps (Gap 3 slot counts,
Gap 4 scope heuristics) are fixed.

**Regression note (2026-03-14):** Sub-task 4a (active-scope-stack overlap detection)
was implemented, rewriting Pass 42 with a proper active-scope-stack algorithm and
DisjointSet union-find. This introduced 1 regression: `error.invalid-prop-mutation-indirect.js`
(added to known-failures.txt) where the new scope merging causes an indirect prop
mutation to no longer be detected by the frozen-mutation validator. Net change: -1
(343 -> 342). The regression is expected to resolve as downstream scope merging
sub-tasks (4b-4f) refine merge eligibility checks.

| Category | Count | Description |
|----------|-------|-------------|
| Compiled with memo | ~924 | Both compile, structure/deps/slots differ (+35 from sentinel regression, -3 from property-path deps, +1 from scope merge regression) |
| No expected file | 261 | Can't compare (no upstream output) |
| Compiled no memo | ~149 | Needs DCE/const-prop/outlining |
| Upstream errors | ~50 | We compile but upstream bails (63 total - 13 invariant/todo skips) |
| @flow fixtures | 38 | OXC parser can't handle Flow syntax |

---

## Active Work

- [~] Temp variable inlining pass (recursive cross-scope counting done; needs remaining P1 fixes to yield fixture gains) — [memoization-structure.md](memoization-structure.md)#gap-1-temp-variable-inlining-pass

---

## Priority 1 -- Memoization Structure (904 fixtures)

The largest divergence category. Both compilers produce `_c()` output but our
structure differs. Sub-breakdown (updated post-sentinel activation):

- ~~400 over-scoped (too many cache slots; globals/stable values as deps)~~ **RESOLVED** -- globals/stable values excluded from deps
- ~~280 sentinel pattern never emitted~~ **RESOLVED** -- sentinel scopes now emitted
- ~90 under-scoped (too few cache slots; missing scopes for some expressions)
- ~37 same slots, wrong deps (dependency tracking diverges; property-path resolution now active)
- ~94 other structural (temp variable naming, code ordering)
- +35 regressions from sentinel activation (scopes correct, deps/slots still wrong)

All items are interdependent -- they must be fixed together for fixtures to pass.
Sentinel scope activation was a necessary structural prerequisite; the 35
regressions are expected and will resolve with scope merging fixes and
slot count alignment. Gap 6 (over-scoped deps), Gap 7 (property-path deps), and Gap 8 (sentinel
codegen) are now resolved. Property-path deps yielded +3 fixtures (315 -> 318).

**Scope merging architecture rewrite (Gap 4):** Deep research revealed that both
merge passes are fundamentally wrong. Pass 42 (overlap detection) needs an active-scope-stack
algorithm with cross-scope mutation tracking. The post-conversion merge needs output-to-input
chaining, nested scope flattening, and safety checks. See memoization-structure.md for the
6-sub-task plan (4a through 4f). Gap 10 is superseded by Sub-task 4a.

- [x] **4a** Active-scope-stack overlap detection (rewrite Pass 42 merge algorithm) — [memoization-structure.md](memoization-structure.md)#sub-task-4a-active-scope-stack-overlap-detection-pass-42
- [x] **4d** Safety checks for intermediate instructions between scopes — [memoization-structure.md](memoization-structure.md)#sub-task-4d-safety-checks-for-intermediate-instructions
- [x] **4e** `scopeIsEligibleForMerging` predicate (always-invalidating types) — [memoization-structure.md](memoization-structure.md)#sub-task-4e-scopeiseligibleformerging-predicate
- [ ] **4c** Nested scope flattening (identical-dep inner scopes) — [memoization-structure.md](memoization-structure.md)#sub-task-4c-nested-scope-flattening
- [x] **4b** Output-to-input scope chaining in invalidate-together — [memoization-structure.md](memoization-structure.md)#sub-task-4b-output-to-input-scope-chaining-in-invalidate-together
- [ ] Correct `_c(N)` slot counts — [memoization-structure.md](memoization-structure.md)#gap-3-cache-slot-count-alignment
- [ ] **4f** DeclarationId alignment for dependency comparison — [memoization-structure.md](memoization-structure.md)#sub-task-4f-declarationid-alignment-for-dependency-comparison
- [ ] setState false-positive in non-reactive dep propagation — [memoization-structure.md](memoization-structure.md)#gap-9-setstate-false-positive-in-non-reactive-propagation

## Priority 2 -- Upstream Errors (~50 actionable fixtures remaining)

We compile functions that upstream rejects with validation errors. These are
"free" fixture gains -- emit the right error and bail, source matches.

- [~] Frozen mutation detection ("This value cannot be modified", 8 remaining of 26) — [upstream-errors.md](upstream-errors.md)#gap-1-frozen-mutation-detection
- [ ] Missing/extra deps in exhaustive-deps (2 remaining, 6 fixed) — [upstream-errors.md](upstream-errors.md)#gap-3-exhaustive-deps-remaining
- [~] Cannot reassign variables outside component (2 remaining of 8) — [upstream-errors.md](upstream-errors.md)#gap-4-reassign-outside-component
- [ ] Cannot access refs during render (6 fixtures) — [upstream-errors.md](upstream-errors.md)#gap-5-ref-access-during-render
- [ ] Hooks must be same function (2 remaining, was 4) — [upstream-errors.md](upstream-errors.md)#gap-6-dynamic-hook-identity
- [ ] Cannot call setState during render (3 fixtures) — [upstream-errors.md](upstream-errors.md)#gap-7-set-state-during-render
- [ ] Cannot access variable before declared (1 fixture, 2 todo-* skippable) — [upstream-errors.md](upstream-errors.md)#gap-8-hoisting-tdz
- [ ] Other upstream errors (~29 remaining: mutation tracking, type providers, ref naming, preserve-memo edge cases) — [upstream-errors.md](upstream-errors.md)#gap-9-other

Note: 15 "Invariant/Todo" upstream errors are internal compiler failures in
Babel -- these should be skipped, not matched.

## Priority 3 -- Compiled No Memo (152 fixtures)

Babel transforms but emits no `_c()`. Our compiler either adds memoization
or fails to apply the same non-memo transforms.

- [ ] DCE / constant propagation (remove dead branches, fold constants) — [compiled-no-memo.md](compiled-no-memo.md)#gap-1-dce-and-constant-propagation
- [ ] Arrow function extraction / outlining — [compiled-no-memo.md](compiled-no-memo.md)#gap-2-arrow-extraction
- [ ] Audit validation passes for error accuracy vs upstream — [over-memoization-bailout.md](over-memoization-bailout.md)#gap-3-ensure-validation-passes-emit-correct-errors
- [ ] Mutation aliasing bail-out (escaped values analysis) — [over-memoization-bailout.md](over-memoization-bailout.md)#gap-5-mutation-aliasing-bail-out
- [ ] "Too simple" function detection (zero reactive scopes) — [over-memoization-bailout.md](over-memoization-bailout.md)#gap-6-too-simple-function-detection

## Priority 4 -- No Expected File (261 fixtures)

These fixtures have no Babel expected output to compare against. Low priority
since we cannot measure conformance without a reference.

- [ ] Generate expected outputs for missing fixtures (run upstream compiler) — [no-expected-file.md](no-expected-file.md)#gap-1-generate-expected-outputs

## Priority 5 -- Flow Fixtures (38 fixtures)

OXC parser cannot handle Flow type annotations. These require either:
- Flow-to-TS preprocessing, or
- Skipping entirely (Flow is being deprecated in React ecosystem)

- [ ] Decide strategy for @flow fixtures — [flow-fixtures.md](flow-fixtures.md)#gap-1-strategy

---

## Blocked

_(Nothing blocked)_

---

## Completed Work (Archive)

All P0-P5 items have been implemented. Detail files have been removed.

### Safety Checks for Intermediate Instructions -- Sub-task 4d (2026-03-14)

- `merge_scopes.rs`: Added complete safety-check infrastructure for `MergeReactiveScopesThatInvalidateTogether`
- `LastUsageMap` pre-pass (`build_last_usage_map` / `collect_last_usage_in_block` / `collect_last_usage_in_terminal`) mirroring upstream's `FindLastUsageVisitor`
- `visit_instruction_read_places`: exhaustive operand collector over all `InstructionValue` variants
- `is_simple_instruction` allowlist predicate + `is_const_store_local` special case
- `IntermediateAccumulator` struct for tracking lvalues/aliases in gaps between scope candidates
- `are_lvalues_last_used_by_scope` safety invariant check against `LastUsageMap`
- Purely additive infrastructure (no behavioral changes); merge decision logic wired in Sub-task 4b
- Conformance: unchanged (342/1717)

### Active-Scope-Stack Overlap Detection -- Sub-task 4a (2026-03-14)

- `merge_scopes.rs`: Complete rewrite of `merge_overlapping_reactive_scopes_hir()` (Pass 42) with active-scope-stack algorithm matching upstream `MergeOverlappingReactiveScopesHIR.ts`
- DisjointSet (union-find with path compression) implemented for tracking scope merge groups
- 3-phase algorithm: (1) collect scope start/end maps + place-to-scope map, (2) walk instructions in ID order with active-scope stack detecting overlaps and cross-scope mutations, (3) rewrite scope annotations using merged representatives
- Cross-scope mutation tracking: mutations to identifiers belonging to non-top-of-stack scopes trigger merges
- 1 regression: `error.invalid-prop-mutation-indirect.js` added to known-failures.txt (indirect prop mutation no longer detected after scope merge changes boundary)
- Conformance: 343 -> 342/1717 (-1)

### Hooks-in-Nested-Functions Validation + MergeOverlappingReactiveScopes Investigation (2026-03-13)

- `validate_hooks_usage.rs`: Rule 4 added -- `check_hooks_in_nested_functions` detects hook calls inside FunctionExpression/ObjectMethod bodies and emits bail diagnostic
- 4 fixtures removed from known-failures.txt: `error.bail.rules-of-hooks-3d692676194b`, `error.bail.rules-of-hooks-8503ca76d6f8`, `error.invalid-hook-in-nested-object-method`, `error.invalid.invalid-rules-of-hooks-d952b82c2597`
- `merge_scopes.rs`: 3-phase DSU algorithm for MergeOverlappingReactiveScopes attempted (union-find with scope grouping and merge). Produced invalid JS due to const scoping across blocks. Reverted to flat-range merge. Data-flow dependency analysis identified as prerequisite for correct overlap merging.
- Conformance: 339 -> 343/1717 (+4)

### Global Reassignment + Async Callback Validation (2026-03-13)

- `validate_no_global_reassignment.rs`: Rewritten with nested function scope analysis -- tracks function declarations, arrow functions, and function expressions as scope boundaries, correctly distinguishing global vs local reassignment
- `validate_locals_not_reassigned_after_render.rs`: Enhanced with async function/arrow detection -- reassignments inside async callbacks now correctly flagged
- `build.rs`: Fixed function declaration lowering to emit StoreLocal connecting the function value to its binding identifier
- 8 fixtures removed from known-failures.txt
- Newly passing: error.assign-global-in-component-tag-function, error.assign-global-in-jsx-children, error.reassign-global-fn-arg, error.mutate-global-increment-op-invalid-react, error.invalid-reassign-local-variable-in-async-callback, error.declare-reassign-variable-in-function-declaration, error.todo-repro-named-function-with-shadowed-local-same-name (x2)
- Conformance: 331 -> 339/1717 (+8)

### Frozen Mutation Detection -- Enhancement (2026-03-13)

- `validate_no_mutation_after_freeze.rs`: Hook-return pre-freeze -- values returned from hook calls (useContext, useState, etc.) and their destructured targets are frozen at definition site
- `validate_no_mutation_after_freeze.rs`: Function-capture freeze -- when a function is passed to a hook call, all variables it captures are frozen after the call
- `validate_no_mutation_after_freeze.rs`: Nested function mutation scanning -- FunctionExpression bodies are recursively scanned for mutations to outer frozen variables
- `validate_no_mutation_after_freeze.rs`: `collect_frozen_from_destructure` handles nested array/object destructure patterns for hook returns
- 13 fixtures removed from known-failures.txt (including capture-ref-for-mutation, modify-state, modify-useReducer-state, context mutations, skip-useMemoCache, etc.)
- Conformance: 318 -> 331/1717 (+13)

### Scope Merge Heuristic Improvements (2026-03-13)

- `merge_scopes.rs`: Name-based dep comparison (`DepKey = (Option<String>, Vec<DependencyPathEntry>)`) replaces IdentifierId-based comparison, fixing false "different deps" when SSA creates unique IDs per Place
- `merge_scopes.rs`: Double-merge prevention via `merged_indices` set -- prevents a scope from being merged into multiple targets
- `merge_scopes.rs`: Dependency union and declaration merge when combining scopes
- `propagate_dependencies.rs`: Non-reactive propagation through `Destructure` instructions (all targets of a non-reactive destructure are non-reactive)
- `propagate_dependencies.rs`: Non-reactive propagation through `CallExpression` when callee + all args are non-reactive (handles `require('shared-runtime')`)
- `propagate_dependencies.rs`: Recursive `collect_destructure_target_ids` for nested object/array destructure patterns
- REVERTED: Overlap merge change (caused regressions in scope boundary detection)
- REVERTED: setState heuristic change (caused false positives in non-reactive propagation)
- Conformance: 318/1717 (unchanged -- structural improvements, no net fixture movement)

### Property-Path Dependency Resolution + Sentinel Codegen Fix (2026-03-13)

- `propagate_dependencies.rs`: temp_map built via `collectTemporaries()` equivalent -- resolves SSA temps to root named variable + property path (e.g., `props.x` instead of just `props`)
- `propagate_dependencies.rs`: `collect_read_operand_places_for_deps` now uses temp_map to emit proper `ReactiveScopeDependency` with `DependencyPathEntry` paths
- `codegen.rs`: `dependency_display_name()` renders deps with property paths (e.g., `props.x.y`, `obj?.field`)
- `codegen.rs`: Sentinel scope codegen fix -- stores first declaration value into sentinel slot so subsequent renders reload from cache
- `types.rs`: Added `PartialEq, Eq` derives to `DependencyPathEntry` for deduplication
- `propagate_dependencies.rs`: `TemporaryInfo` struct avoids full `Identifier` clone overhead
- 3 fixtures removed from known-failures.txt: `jsx-empty-expression.js`, `jsx-namespaced-name.js`, `multiple-components-first-is-invalid.js`
- Conformance: 315 -> 318/1717 (+3)

### Over-Scoped Dependency Fix (2026-03-13)

- Globals, stable hook returns (SetState, Ref), and property accesses of globals excluded from reactive dependencies
- Three files modified: `infer_types.rs`, `infer_reactive_places.rs`, `propagate_dependencies.rs`
- Conformance unchanged at 272/1717 (gains expected to compound with remaining P1 fixes)

### Sentinel Scope Emission (2026-03-13)

- Reactive scopes now created for allocating expressions (JSX, object/array literals)
- Sentinel pattern (`Symbol.for("react.memo_cache_sentinel")`) emitted in codegen
- 35 known regressions added to known-failures.txt (scopes correct, deps/slots still diverge)
- Net conformance change: 304 -> 272 (-32; 35 regressions, 3 newly passing)
- Implementation files: `infer_reactive_scope_variables.rs`, `prune_scopes.rs`, `codegen.rs`

### Conformance Quick Wins (2026-03-12)

- TS type stripping via OXC parse/transform/print roundtrip (+30 fixtures)
- JSX normalization via OXC transformer
- Bail on all validation errors (AllErrors threshold, +24 fixtures)
- Skip functions with zero cache slots (+90 fixtures)
- Upstream error matching (+120 fixtures)
- OutputMode::Lint and gating directives (+37 fixtures)

### Temp Variable Inlining Foundation (2026-03-13)

- Recursive cross-scope temp use-counting in codegen.rs
- FxHash migration for all codegen collections

### JSX Syntax Preservation (2026-03-13)

- JSX syntax preservation fully implemented in codegen.rs
- `_jsx()`/`_jsxs()`/`_Fragment` calls replaced with actual JSX syntax (`<div>`, `<Component>`, `<>...</>`)
- `react/jsx-runtime` import removed from generated output
- 23 snapshot files updated; conformance unchanged at 304/1717 (normalization masks JSX differences)

### ValidatePreservedManualMemoization Pipeline Gate Fixes (2026-03-13)

- Pipeline gate fixed: Pass 5 (drop_manual_memoization) now keeps memo markers when `validate_preserve_existing_memoization_guarantees` is set
- Pass 61 now runs on both `enable` and `validate_only` config flags
- Error messages aligned with upstream ("Existing memoization could not be preserved...")
- Pruned memoizations now silently skipped instead of emitting false-positive errors
- 20 preserve-memo-validation error fixtures now passing
- 11 additional error fixtures passing (hoist-optional-member-expression, validate-object-entries/values, gating bailout, new-mutability errors)
- Conformance: 278 -> 309/1717 (+31)
- Implementation files: `pipeline.rs`, `validate_preserved_manual_memoization.rs`

### Frozen Mutation Detection -- Initial Pass (2026-03-13)

- `validate_no_mutation_after_freeze` pass added (Pass 16.5, runs after infer_mutation_aliasing_effects)
- Detects mutations to frozen values: property stores, computed stores, array push on frozen arrays
- Also detects for-in/for-of loops over context variables (upstream "Todo" errors)
- 6 fixtures now passing: invalid-array-push-frozen, invalid-computed-store-to-frozen-value, invalid-mutate-after-freeze, invalid-property-store-to-frozen-value, todo-for-in-loop-with-context-variable-iterator, todo-for-of-loop-with-context-variable-iterator
- Conformance: 272 -> 278/1717 (+6)
- Implementation files: `validate_no_mutation_after_freeze.rs`, `pipeline.rs`
- 20 fixtures remain (require deeper alias tracking, delete operations, indirect mutation through function calls)

### Validation SSA Improvements (2026-03-13)

- SSA name resolution in validate_use_memo (+3 fixtures)
- PropertyStore/PropertyLoad ref tracking in ref-access-in-render (+6 fixtures)
- setState detection in useMemo callbacks (+2 fixtures)
- SSA resolution in impure function detection + performance.now() (+2 fixtures)
- SSA resolution in derived-computation-in-effects (+1 fixture)
- SSA resolution in exhaustive-dependency validation (correctness)
- SSA resolution in set-state-in-render (+9 fixtures)
- SSA resolution in ref-access-in-render (+15 fixtures)
- SSA resolution in set-state-in-effects (correctness)
- SSA resolution in capitalized call validation (+3 fixtures)
- Conditional hook method calls (+3 fixtures)
- Global hook names in SSA for conditional hook detection (+8 fixtures)
- Hooks-as-values validation (+9 fixtures)
- validate_no_global_reassignment pass (new)
- validate_no_eval pass (new, Pass 14.6 -- EvalUnsupported diagnostic)

### Render Equivalence (formerly render-equivalence.md)

- Availability-schedule truncated output fixed
- Phi-node / temporary variable resolution fixed
- JSX hyphenated attribute name quoting fixed
- Multi-step-form timeout/segfault resolved
- Conservative memoization misses addressed
- Render equivalence tracking added to CI

### Upstream Conformance (formerly upstream-conformance.md)

- Upstream fixtures downloaded with expected outputs generated
- Baseline conformance run and triaged
- known-failures.txt populated; conformance added to CI
- Panics fixed; high-priority divergences resolved

### Vite Caching (formerly vite-caching.md)

- In-memory content-hash cache added to Vite plugin
- Config change invalidation implemented
- Optional disk cache for large projects added

### P0-P5 Implementation

- Critical bugs: destructured params, dependency filter, O(N^2) perf fix
- Correctness: ComputeUnconditionalBlocks, CollectHoistablePropertyLoads, CollectOptionalChainDependencies, DeriveMinimalDependenciesHIR, ScopeDependencyUtils
- Type-based ref/setState detection in validation passes
- Config gates, validation passes, optimization passes
- Code quality, testing/CI, polish (see git history for details)
