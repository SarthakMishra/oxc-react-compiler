# Performance Benchmarks

> **Priority**: LOW (performance and developer experience -- not correctness-blocking)
> **Impact**: Headline "how much faster" metric, regression detection, bundle size validation

## Problem Statement

We have a basic benchmark suite (16 fixtures, per-fixture timing, structural divergence analysis, render comparison) but lack head-to-head speed comparison against Babel, memory profiling, bundle size impact measurement, runtime performance validation, and CI regression tracking. Without these, we cannot quantify or defend the performance story.

## Current State

Existing infrastructure in `benchmarks/`:
- **16 fixtures** across 4 size tiers (XS/S/M/L, 8-255 LOC, 1-9 hooks)
- **bench.mjs** -- per-fixture timing (wall + Rust-internal via `rustCompileNs`), p50/p95, memory snapshot
- **babel-compile.mjs** -- structural divergence (cache slots, scope blocks, dependency checks)
- **render-compare.mjs** -- semantic correctness via ReactDOMServer HTML comparison
- **analyze-correctness.mjs** -- memoization pattern heuristic analysis
- **vite-cache-bench.mjs** -- cache hit/miss speedup measurement
- **NAPI `transformReactFileTimed`** -- returns `rustCompileNs` (excludes NAPI marshalling)

## Implementation Plan

### Gap 1: Per-File Speed Comparison (OXC vs Babel)

**Upstream:** N/A (benchmark infrastructure)
**Current state:** `bench.mjs` measures OXC timing only. No Babel timing or side-by-side comparison.
**What's needed:**
- Add `@babel/core` and `babel-plugin-react-compiler` as dev dependencies in `benchmarks/`
- Add `--compare` flag to `bench.mjs` that runs Babel's `transformSync()` with the react-compiler plugin on each fixture
- Interleave OXC and Babel iterations to reduce thermal/frequency bias
- Report speedup ratio (`babel_p50 / oxc_p50`) per fixture and aggregate
- Add `--format json` and `--format markdown` output modes for machine consumption and README embedding
- Methodology: 20 warmup iterations (discarded), 100 measured iterations, report min/p50/p95/p99/max/mean/stddev
**Depends on:** None

### Gap 2: Batch Throughput Comparison

**Upstream:** N/A (benchmark infrastructure)
**Current state:** No batch/project-level throughput measurement exists.
**What's needed:**
- Add `--batch-compare` mode to `bench.mjs` that compiles all 16 fixtures in sequence (simulates Vite transform hook, which is single-threaded per file)
- 5 warmup passes, 20 measured passes
- Report: total wall time, files/sec, bytes/sec for OXC vs Babel
**Depends on:** Gap 1 (shares the Babel integration)

### Gap 3: Bundle Size Impact

**Upstream:** N/A (benchmark infrastructure)
**Current state:** No measurement of how compiled output affects bundle size.
**What's needed:**
- Create `scripts/bundle-size-bench.mjs` measuring per-fixture: original source size, OXC compiled size, Babel compiled size
- Minify all three variants with esbuild (consistent minifier)
- Gzip all three (zlib.gzipSync, level 9)
- Report: raw/minified/gzipped sizes and deltas vs original, per-fixture and aggregate
- Validates that memoization cache overhead does not offset re-render savings
**Depends on:** Gap 1 (needs Babel compilation for comparison)

### Gap 4: Runtime Re-Render Count Validation

**Upstream:** N/A (benchmark infrastructure)
**Current state:** `render-compare.mjs` checks HTML equivalence on initial render but does not measure re-render behavior.
**What's needed:**
- Create `scripts/rerender-count-bench.mjs` that for each fixture: mounts component, applies 5 prop updates, counts renders via wrapper component
- Compare render counts: original vs OXC-compiled vs Babel-compiled
- Fewer renders = memoization is working correctly
- This is the most important end-to-end validation that the compiler is actually doing its job
**Depends on:** Gap 1 (needs Babel compilation for comparison)

### Gap 5: Consolidated Report and CI Integration

**Upstream:** N/A (benchmark infrastructure)
**Current state:** No single command produces a complete benchmark report. No CI regression detection.
**What's needed:**
- Create `benchmarks/run-all.mjs` orchestrator that runs speed, bundle size, and re-render benchmarks
- Markdown output suitable for README or PR descriptions
- JSON output for machine consumption and trend tracking
- Create `scripts/check-regression.mjs` that compares against a stored baseline (`benchmarks/baseline.json`) and fails if any fixture regresses by >20%
- Add benchmark CI job (non-blocking initially, `continue-on-error: true`)
**Depends on:** Gaps 1-4

### Gap 6: Cold Start and Memory Profiling

**Upstream:** N/A (benchmark infrastructure)
**Current state:** No cold start timing or memory profiling.
**What's needed:**
- Create `scripts/cold-start-bench.mjs` measuring `require()` time for OXC NAPI binary vs Babel + plugin dependency tree, plus first-transform latency
- Add `--memory-profile` flag to `bench.mjs` recording `process.memoryUsage()` (rss, heapUsed, heapTotal, external) before/after each fixture compilation
- Force GC between fixtures (`--expose-gc` + `global.gc()`) for accuracy
- Note: OXC allocates in Rust heap (not tracked by `heapUsed`), so `rss` and `external` are the relevant metrics
**Depends on:** Gap 1 (shares the Babel integration)

### Gap 7: Vite Dev Server and HMR Benchmarks

**Upstream:** N/A (benchmark infrastructure)
**Current state:** No DX-focused benchmarks exist (startup time, HMR latency).
**What's needed:**
- Create a minimal Vite project in `benchmarks/vite-testbed/` with all 16 fixtures as routes/components
- Three Vite configs: no compiler, OXC plugin, Babel plugin
- Create `scripts/vite-startup-bench.mjs` measuring `vite dev` startup to "ready" log (use hyperfine or internal timing)
- Create `scripts/hmr-latency-bench.mjs` measuring file-save to HMR-update-received latency via Vite's WebSocket protocol
- Target: OXC should add <5ms to HMR latency for XS/S fixtures
**Depends on:** Gap 1 (shares Babel integration and fixture infrastructure)

### Gap 8: Per-Pass Timing and Criterion Benchmarks

**Upstream:** N/A (internal profiling)
**Current state:** No per-pass timing instrumentation. No Rust-native benchmarks.
**What's needed:**
- Add `compile-timing` feature flag to `oxc_react_compiler` with a `timed_pass!` macro wrapping each pass in `pipeline.rs`
- Add `criterion` benchmarks in `crates/oxc_react_compiler/benches/` with one benchmark per size tier (XS/S/M/L)
- Optionally integrate `iai-callgrind` for deterministic instruction-count stability in CI
- Optionally integrate CodSpeed for Rust-side CI regression detection (follows OXC's approach)
**Depends on:** None (Rust-side only, independent of JS benchmarks)

## Measurement Strategy

After each gap, run the relevant benchmark and verify output is reasonable:
```bash
# After Gap 1:
node benchmarks/bench.mjs --compare --format markdown

# After Gap 3:
node benchmarks/scripts/bundle-size-bench.mjs

# After Gap 5:
node benchmarks/run-all.mjs --compare --format markdown
```

## Risks and Notes

- **Babel version drift**: Pin `babel-plugin-react-compiler` to a specific version so benchmarks are reproducible. Update periodically and re-baseline.
- **Thermal throttling**: Interleave OXC/Babel iterations and use `--warmup` to mitigate. For CI, use fixed-frequency CPU governors if available.
- **NAPI marshalling overhead**: Always report both `rustCompileNs` (pure Rust) and wall time (including NAPI marshalling) to avoid misleading speedup claims.
- **Fixture representativeness**: Current 16 fixtures cover common patterns but miss hooks-heavy (10+), context consumers, render props, HOC chains, Suspense/lazy, and large forms. Consider expanding fixtures independently.
- **Re-render benchmarks require JSDOM**: The re-render count benchmark needs `react`, `react-dom`, and a test renderer. These are already dev dependencies in the E2E test setup.
