# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.
> Items are ordered by dependency: nothing should be blocked by an item below it.

Last updated: 2026-03-11

---

## Priority 1 -- Correctness (output is broken)

- [ ] Fix `promote_used_temporaries` to rename ALL identifier references, not just lvalues -- [codegen-correctness.md](codegen-correctness.md)#gap-1-variable-reference-naming-mismatch-_tn-vs-tn
- [ ] Fix `place_name()` fallback to use `t{id}` instead of `_t{id}` -- [codegen-correctness.md](codegen-correctness.md)#gap-3-place_name-fallback-uses-underscore-prefix

## Priority 2 -- Core memoization (compiler produces no memoization)

- [ ] Debug memoization pipeline: why no `ReactiveScopeBlock` nodes in output -- [memoization-codegen.md](memoization-codegen.md)#gap-1-verify-_cn-cache-allocation-is-emitted
- [ ] Verify `propagate_scope_dependencies_hir` populates scope deps/decls -- [memoization-codegen.md](memoization-codegen.md)#gap-2-verify-n-memoization-slot-readswrites-are-emitted
- [ ] Add end-to-end memoization snapshot test -- [memoization-codegen.md](memoization-codegen.md)#gap-3-end-to-end-memoization-test

## Priority 3 -- Test coverage (upstream parity)

- [ ] Config parsing and option construction unit tests -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-1-config-parsing-and-option-construction-tests
- [ ] Error diagnostic fixture tests (~20 fixtures covering 17 DiagnosticKind variants) -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-2-error-diagnostic-fixture-tests
- [ ] Post-codegen output validation (parse check + use-before-define) -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-3-post-codegen-output-validation
- [ ] E2E dual-mode rendering tests (compiled vs uncompiled DOM comparison) -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-4-e2e-dual-mode-rendering-tests
- [ ] Sprout-equivalent runtime evaluation (semantic correctness verification) -- [test-coverage-gaps.md](test-coverage-gaps.md)#gap-5-sprout-equivalent-runtime-evaluation

## Priority 4 -- Tooling (parallel, not blocking)

- [ ] Real-world fixture extraction pipeline (OSS repos, pinned commits, LOC categorization) -- [benchmark-suite.md](benchmark-suite.md)#gap-1-real-world-fixture-extraction-pipeline
- [ ] Benchmark harness v2 (Rust-side timer via NAPI, warmup, batch mode, memory) -- [benchmark-suite.md](benchmark-suite.md)#gap-2-benchmark-harness-v2-speed-memory-separated-overhead
- [ ] Deep correctness analysis (AST structural diff, semantic equivalence, divergence classification) -- [benchmark-suite.md](benchmark-suite.md)#gap-3-deep-correctness-analysis
- [ ] Differential snapshot tests (committed expected outputs, update workflow) -- [benchmark-suite.md](benchmark-suite.md)#gap-4-differential-snapshot-tests
- [ ] CI integration (dedicated runner, baseline.json, per-fixture failure tracking) -- [benchmark-suite.md](benchmark-suite.md)#gap-5-ci-integration
- [ ] README and correctness score documentation -- [benchmark-suite.md](benchmark-suite.md)#gap-6-readme-and-correctness-score-documentation

---

## Active Work

_(Nothing in progress)_

---

## Blocked

_(Nothing blocked)_

---

## Completed Work (Archive)

All previously planned workstreams have been completed:

- **Tier 2 Lint Rules** -- Full Rules of Hooks with CFG analysis, immutability checking, manual memoization preservation, exhaustive memo deps, exhaustive effect deps, structured DiagnosticKind filtering
- **Source Maps** -- Source map generation from codegen, NAPI passthrough, Vite plugin wiring, whole-file source map composition
- **Upstream Conformance** -- Fixture download, upstream oracle runner, differential comparison harness, output normalization
- **Documentation** -- Vite plugin usage guide, lint rules docs, configuration reference, known limitations
- **Clippy Cleanup** -- 258 mechanical fixes, crate-level allows for style lints, zero warnings across workspace
