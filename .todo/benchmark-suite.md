# Shareable Benchmark Suite

> A benchmark suite comparing OXC React Compiler vs Babel on real-world React components,
> with deep correctness analysis, differential snapshots, and CI regression tracking.

Last updated: 2026-03-12

---

## Gap 1: Real-world fixture extraction pipeline ✅

~~**Upstream:** N/A (OXC-specific tooling)~~

**Completed**: Created 12 standalone real-world fixtures inspired by cal.com, excalidraw, and shadcn/ui. Total: 16 fixtures (4 per tier: XS/S/M/L), with 3 known-divergent. Updated `benchmarks/fixtures/manifest.json` with all entries.

---

## Gap 2: Benchmark harness v2 (speed, memory, separated overhead) ✅

**Completed.**
- `transform_react_file_timed()` NAPI function with Rust-side `std::time::Instant` measurement in `napi/react-compiler/src/lib.rs`
- `benchmarks/bench.mjs` with warmup, measured iterations, batch mode, filter, json/markdown output, memory tracking
- CLI: `--iterations`, `--warmup`, `--batch`, `--format`, `--filter`, `--update-snapshots`, `--check-snapshots`

---

## Gap 3: Deep correctness analysis ✅

~~**Upstream:** N/A~~

**Completed (3a + 3c):**
- `benchmarks/scripts/babel-compile.mjs` — Babel compilation via `babel-plugin-react-compiler`, structural AST pattern extraction (cache size, sentinel checks, dependency checks, scope blocks, cache reads/writes), automated divergence classification into `cosmetic`, `conservative_miss`, `over_memoization`, `semantic_difference`
- Markdown and JSON report output with correctness scoring
- Results across 16 fixtures: 14 conservative_miss, 1 over_memoization, 1 semantic_difference, score 0.938
- npm scripts: `babel:update-snapshots`, `babel:diff`, `babel:diff-json`, `correctness`

**Completed (3b):**
- `benchmarks/scripts/render-compare.mjs` — Headless render comparison using ReactDOMServer. Compiles each fixture with both OXC and Babel, renders with identical props sequences, compares HTML output. Per-fixture props map with multiple states (e.g. status-badge tested with all 4 statuses, booking-list with empty and populated data).
- Results: Babel renders match original 100% on renderable fixtures. OXC renders fail due to known codegen bugs (unresolved destructured props/derived values). Render equivalence score: 0.000 — expected given current codegen state.
- npm scripts: `render:compare`, `render:compare-json`, `render:compare-verbose`
- Dependencies: added react, react-dom, esbuild to benchmarks/package.json

---

## Gap 4: Differential snapshot tests ✅

~~**Upstream:** N/A~~

**Completed**: Added `.babel.js` snapshots for all 16 fixtures via `npm run babel:update-snapshots`. Added `.diff.json` structural diff reports for all 16 fixtures. Updated `bench.mjs` docs to reference new commands. Babel dependencies added to `benchmarks/package.json`.

---

## Gap 5: CI integration ✅

**Completed.**
- `.github/workflows/benchmark.yml` with full pipeline: NAPI build, cargo test, benchmarks, snapshot check, E2E tests
- PR benchmark comments via `actions/github-script` with formatted timing tables
- Baseline update on main merges

---

## Gap 6: README and correctness score documentation ✅

~~**Upstream:** N/A~~

**Completed**: Documented divergence classifications (conservative miss, over-memoization, semantic difference, cosmetic), known acceptable divergences, scoring methodology, and aggregate scoring in `benchmarks/README.md`.

---

## Summary

| Gap | Name | Status |
|-----|------|--------|
| 1 | Fixture extraction pipeline | ✅ Done — 16 fixtures (4 per tier), 3 known-divergent |
| 2 | Benchmark harness v2 | ✅ Done |
| 3 | Deep correctness analysis | ✅ Done — AST diff + classification + headless render comparison |
| 4 | Differential snapshot tests | ✅ Done — `.babel.js` + `.diff.json` snapshots for all 16 fixtures |
| 5 | CI integration | ✅ Done |
| 6 | README documentation | ✅ Done — divergence classifications + scoring methodology |
