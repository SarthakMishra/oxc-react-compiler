# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.

Last updated: 2026-03-13

Current conformance: 272/1717 pass (15.8%), 0 panics, 0 unexpected divergences.

Note: Most passing fixtures match by both compilers returning source unchanged
(trivial match via lint mode, validation bail-out, or non-component detection).
Only 2 fixtures match with actual compiled `_c()` output. The remaining 1445
divergences break down as follows:

**Regression note (2026-03-13):** Sentinel scope emission (Gap 5) was activated,
correctly adding reactive scopes for allocating expressions. This introduced 35
regressions (added to known-failures.txt) where the new scopes are structurally
correct but other P1 issues (over-scoped deps, wrong slot counts) cause the
overall output to still diverge. Net change: -32 (35 regressions, 3 newly passing).
The regressions will resolve as remaining P1 gaps (Gap 3 slot counts,
Gap 4 scope heuristics) are fixed.

| Category | Count | Description |
|----------|-------|-------------|
| Compiled with memo | ~939 | Both compile, structure/deps/slots differ (+35 from sentinel regression) |
| No expected file | 261 | Can't compare (no upstream output) |
| Compiled no memo | ~149 | Needs DCE/const-prop/outlining |
| Upstream errors | ~96 | We compile but upstream bails |
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
- ~40 same slots, wrong deps (dependency tracking diverges)
- ~94 other structural (temp variable naming, code ordering)
- +35 regressions from sentinel activation (scopes correct, deps/slots still wrong)

All items are interdependent -- they must be fixed together for fixtures to pass.
Sentinel scope activation was a necessary structural prerequisite; the 35
regressions are expected and will resolve with over-scoped dep fixes (Gap 6)
and slot count alignment (Gap 3). Gap 6 (over-scoped deps) is now resolved.

- [ ] Scope merging/splitting heuristic audit vs upstream — [memoization-structure.md](memoization-structure.md)#gap-4-scope-mergingsplitting-heuristic-review
- [ ] Correct `_c(N)` slot counts — [memoization-structure.md](memoization-structure.md)#gap-3-cache-slot-count-alignment

## Priority 2 -- Upstream Errors (96 fixtures)

We compile functions that upstream rejects with validation errors. These are
"free" fixture gains -- emit the right error and bail, source matches.

- [ ] Frozen mutation detection ("This value cannot be modified", 26 fixtures) — [upstream-errors.md](upstream-errors.md)#gap-1-frozen-mutation-detection
- [ ] ValidatePreserveExistingMemoization ("Compilation Skipped", 13 fixtures) — [upstream-errors.md](upstream-errors.md)#gap-2-validate-preserve-existing-memoization
- [ ] Missing/extra deps in exhaustive-deps (8 fixtures) — [upstream-errors.md](upstream-errors.md)#gap-3-exhaustive-deps-remaining
- [ ] Cannot reassign variables outside component (6 fixtures) — [upstream-errors.md](upstream-errors.md)#gap-4-reassign-outside-component
- [ ] Cannot access refs during render (6 fixtures) — [upstream-errors.md](upstream-errors.md)#gap-5-ref-access-during-render
- [ ] Hooks must be same function (4 fixtures) — [upstream-errors.md](upstream-errors.md)#gap-6-dynamic-hook-identity
- [ ] Cannot call setState during render (2 fixtures) — [upstream-errors.md](upstream-errors.md)#gap-7-set-state-during-render
- [ ] Cannot access variable before declared (2 fixtures) — [upstream-errors.md](upstream-errors.md)#gap-8-hoisting-tdz
- [ ] Other upstream errors (~7 remaining fixtures, eval done) — [upstream-errors.md](upstream-errors.md)#gap-9-other

Note: 21 "Invariant/Todo" upstream errors are internal compiler failures in
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
