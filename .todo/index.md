# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.
> Items are ordered by dependency: nothing should be blocked by an item below it.

Last updated: 2026-03-12

---

## Priority 3 -- Test coverage (upstream parity)

- [ ] Error diagnostic fixture tests — expand from 4/17 to all DiagnosticKind variants -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-2-error-diagnostic-fixture-tests
- [ ] Post-codegen output validation — add oxc_semantic use-before-define checking and integration into other test runners -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-3-post-codegen-output-validation

## Priority 4 -- Tooling (parallel, not blocking)

- [ ] Real-world fixture extraction — run extraction against OSS repos, reach 15+ fixtures across all tiers -- [benchmark-suite.md](benchmark-suite.md)#gap-1-real-world-fixture-extraction-pipeline
- [ ] Deep correctness analysis — add Babel AST diffing, headless render comparison, full divergence classification -- [benchmark-suite.md](benchmark-suite.md)#gap-3-deep-correctness-analysis
- [ ] Differential snapshot tests — add Babel comparison snapshots and diff.json reports -- [benchmark-suite.md](benchmark-suite.md)#gap-4-differential-snapshot-tests
- [ ] README correctness score documentation — document divergence classifications and scoring methodology -- [benchmark-suite.md](benchmark-suite.md)#gap-6-readme-and-correctness-score-documentation

---

## Active Work

_(Nothing active)_

---

## Blocked

_(Nothing blocked)_

---

## Completed Work (Archive)

- **P1: Scope boundary alignment** -- Name-based reactivity propagation in `infer_reactive_places`, hook detection via LoadGlobal binding name resolution, scope terminal self-referential fallthrough fix
- **Codegen Correctness** -- Fixed `promote_used_temporaries` to walk all places (operands, terminals, scope deps/decls), fixed `place_name()` fallback to use `t{id}` instead of `_t{id}`, JSX naming fixed as a consequence
- **Memoization Pipeline** -- Fixed scope assignment in `infer_reactive_scope_variables`, reactive param marking in `infer_reactive_places`, range extension in `infer_mutation_aliasing_ranges`, dependency/declaration population in `propagate_scope_dependencies_hir`, instruction dedup in `build_reactive_function`, added E2E memoization snapshot test, destructure pattern target propagation
- **Tier 2 Lint Rules** -- Full Rules of Hooks with CFG analysis, immutability checking, manual memoization preservation, exhaustive memo deps, exhaustive effect deps, structured DiagnosticKind filtering
- **Source Maps** -- Source map generation from codegen, NAPI passthrough, Vite plugin wiring, whole-file source map composition
- **Upstream Conformance** -- Fixture download, upstream oracle runner, differential comparison harness, output normalization
- **Documentation** -- Vite plugin usage guide, lint rules docs, configuration reference, known limitations
- **Clippy Cleanup** -- 258 mechanical fixes, crate-level allows for style lints, zero warnings across workspace
- **P3 Gap 1: Config parsing tests** -- 10 unit tests in `options.rs` covering all enum variants, `from_map()`, `GatingConfig`, `SourceFilter`
- **P3 Gap 4: E2E dual-mode tests** -- Vitest + esbuild JSX transform + vm eval + ReactDOMServer, 31 tests across 5 files with self-healing `it.fails` for known codegen issues
- **P3 Gap 5: Sprout runtime eval** -- Function evaluator with shared runtime utilities, mutation tracking, sequential render consistency, 11 tests
- **P4 Gap 2: Benchmark harness v2** -- `transformReactFileTimed()` NAPI function with Rust-side `std::time::Instant`, `bench.mjs` with warmup/batch/filter/json output
- **P4 Gap 5: CI integration** -- `.github/workflows/benchmark.yml` with full pipeline and PR benchmark comments
