# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.
> Items are ordered by dependency: nothing should be blocked by an item below it.

Last updated: 2026-03-12

---

## Priority 1 -- Core memoization

- [x] Fix scope boundary alignment so scopes wrap computation instructions, not discriminant markers -- [memoization-codegen.md](memoization-codegen.md)#gap-1-debug-memoization-pipeline----reactivescopeblock-generation

## Priority 3 -- Test coverage (upstream parity)

- [x] Config parsing and option construction unit tests -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-1-config-parsing-and-option-construction-tests
- [x] Error diagnostic fixture tests (~20 fixtures covering 17 DiagnosticKind variants) -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-2-error-diagnostic-fixture-tests
- [x] Post-codegen output validation (parse check + use-before-define) -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-3-post-codegen-output-validation
- [x] E2E dual-mode rendering tests (compiled vs uncompiled DOM comparison) -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-4-e2e-dual-mode-rendering-tests
- [x] Sprout-equivalent runtime evaluation (semantic correctness verification) -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-5-sprout-equivalent-runtime-evaluation

## Priority 4 -- Tooling (parallel, not blocking)

- [x] Real-world fixture extraction pipeline (OSS repos, pinned commits, LOC categorization) -- [benchmark-suite.md](benchmark-suite.md)#gap-1-real-world-fixture-extraction-pipeline
- [x] Benchmark harness v2 (Rust-side timer via NAPI, warmup, batch mode, memory) -- [benchmark-suite.md](benchmark-suite.md)#gap-2-benchmark-harness-v2-speed-memory-separated-overhead
- [x] Deep correctness analysis (AST structural diff, semantic equivalence, divergence classification) -- [benchmark-suite.md](benchmark-suite.md)#gap-3-deep-correctness-analysis
- [x] Differential snapshot tests (committed expected outputs, update workflow) -- [benchmark-suite.md](benchmark-suite.md)#gap-4-differential-snapshot-tests
- [x] CI integration (dedicated runner, baseline.json, per-fixture failure tracking) -- [benchmark-suite.md](benchmark-suite.md)#gap-5-ci-integration
- [x] README and correctness score documentation -- [benchmark-suite.md](benchmark-suite.md)#gap-6-readme-and-correctness-score-documentation

---

## Active Work

_(Nothing active)_

---

## Blocked

_(Nothing blocked)_

---

## Completed Work (Archive)

All previously planned workstreams have been completed:

- **Codegen Correctness** -- Fixed `promote_used_temporaries` to walk all places (operands, terminals, scope deps/decls), fixed `place_name()` fallback to use `t{id}` instead of `_t{id}`, JSX naming fixed as a consequence
- **Memoization Pipeline** -- Fixed scope assignment in `infer_reactive_scope_variables`, reactive param marking in `infer_reactive_places`, range extension in `infer_mutation_aliasing_ranges`, dependency/declaration population in `propagate_scope_dependencies_hir`, instruction dedup in `build_reactive_function`, added E2E memoization snapshot test. Fixed scope boundary alignment: name-based reactivity propagation in `infer_reactive_places` (resolves HIR builder fresh-ID-per-Place issue), hook detection via LoadGlobal binding name resolution, scope terminal self-referential fallthrough fix, destructure pattern target propagation
- **Tier 2 Lint Rules** -- Full Rules of Hooks with CFG analysis, immutability checking, manual memoization preservation, exhaustive memo deps, exhaustive effect deps, structured DiagnosticKind filtering
- **Source Maps** -- Source map generation from codegen, NAPI passthrough, Vite plugin wiring, whole-file source map composition
- **Upstream Conformance** -- Fixture download, upstream oracle runner, differential comparison harness, output normalization
- **Documentation** -- Vite plugin usage guide, lint rules docs, configuration reference, known limitations
- **Clippy Cleanup** -- 258 mechanical fixes, crate-level allows for style lints, zero warnings across workspace
- **Test Coverage** -- Config parsing unit tests, error diagnostic fixture tests with insta snapshots, post-codegen output validation (parse check), E2E dual-mode rendering tests (Vitest + esbuild JSX transform + vm sandboxed eval + ReactDOMServer), sprout-equivalent runtime evaluation with function eval + mutation tracking
- **Benchmark Suite** -- Synthetic fixture extraction with manifest.json, benchmark harness v2 with `transformReactFileTimed()` NAPI function (Rust-side `std::time::Instant`), warmup/batch/filter/json output, differential snapshots with update/check workflows, deep correctness analysis (memoization pattern extraction, sentinel/dependency check counting, divergence classification), CI workflow with PR benchmark comments, README documentation
