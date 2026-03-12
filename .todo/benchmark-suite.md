# Shareable Benchmark Suite

> A benchmark suite comparing OXC React Compiler vs Babel on real-world React components,
> with deep correctness analysis, differential snapshots, and CI regression tracking.

Last updated: 2026-03-12

---

## Gap 1: Real-world fixture extraction pipeline [~]

**Upstream:** N/A (OXC-specific tooling)

**Done so far:**
- `benchmarks/fixtures/` directory with `manifest.json` and 4 synthetic fixtures (1 XS, 2 S, 1 M)
- `benchmarks/scripts/extract-fixtures.sh` extraction script with repo definitions for cal.com, excalidraw, shadcn/ui
- Fixture categorization by size tier (XS/S/M/L) with LOC and hook count

**Remaining:**
- Run extraction script against actual OSS repos and pin to specific commit SHAs
- Reach minimum 15 fixtures total, at least 3 per size tier (currently 0 L-tier)
- Include 2-3 components that produce divergent output (`"known_divergent": true` in manifest)
- Strip non-essential imports so fixtures compile standalone

**Depends on:** None

---

## Gap 2: Benchmark harness v2 (speed, memory, separated overhead) ✅

**Completed.**
- `transform_react_file_timed()` NAPI function with Rust-side `std::time::Instant` measurement in `napi/react-compiler/src/lib.rs`
- `benchmarks/bench.mjs` with warmup, measured iterations, batch mode, filter, json/markdown output, memory tracking
- CLI: `--iterations`, `--warmup`, `--batch`, `--format`, `--filter`, `--update-snapshots`, `--check-snapshots`

---

## Gap 3: Deep correctness analysis [~]

**Upstream:** N/A

**Done so far:**
- `benchmarks/scripts/analyze-correctness.mjs` with regex-based memoization pattern extraction from OXC output
- Extracts: cache size, sentinel checks, dependency checks, cache reads/writes, scope blocks
- Divergence classification: `ok`, `no_memoization`, `conservative_miss`
- JSON and markdown output formats

**Remaining:**
- **3a: Structural AST diffing** — Parse both OXC and Babel outputs into ASTs (requires `babel-plugin-react-compiler` as a dependency), compare memoization blocks structurally (slot count, scope boundaries, dependency arrays)
- **3b: Semantic equivalence via headless render** — Render OXC-compiled and Babel-compiled versions with identical props/state sequences, compare HTML output at each step
- **3c: Full divergence classification** — Automated classification into conservative miss, over-memoization, semantic difference, and cosmetic categories based on actual Babel comparison (current classification is heuristic-only)

**Depends on:** Gap 1 (more fixtures), Babel integration as a dependency

---

## Gap 4: Differential snapshot tests [~]

**Upstream:** N/A

**Done so far:**
- `benchmarks/snapshots/` directory with 4 committed `.oxc.js` snapshot files
- `--update-snapshots` and `--check-snapshots` workflows in `bench.mjs`
- CI runs snapshot checking

**Remaining:**
- Add `.babel.js` snapshots (Babel's output for each fixture) — requires `babel-plugin-react-compiler` dependency
- Add `.diff.json` snapshots (structural diff report from Gap 3a)

**Depends on:** Gap 3a (Babel integration for comparison snapshots)

---

## Gap 5: CI integration ✅

**Completed.**
- `.github/workflows/benchmark.yml` with full pipeline: NAPI build, cargo test, benchmarks, snapshot check, E2E tests
- PR benchmark comments via `actions/github-script` with formatted timing tables
- Baseline update on main merges

---

## Gap 6: README and correctness score documentation [~]

**Upstream:** N/A

**Done so far:**
- `benchmarks/README.md` with quick start, CLI usage, fixture tiers, adding fixtures, output metrics, snapshot verification, CI integration

**Remaining:**
- Document the four divergence classifications (conservative miss, over-memoization, semantic difference, cosmetic) and which are acceptable vs bugs
- Add "known acceptable divergences" section
- Document correctness score methodology and how to interpret it

**Depends on:** Gap 3 (divergence classification must be implemented first)

---

## Summary

| Gap | Name | Status |
|-----|------|--------|
| 1 | Fixture extraction pipeline | [~] Partial — 4 synthetic fixtures, need 15+ real-world |
| 2 | Benchmark harness v2 | ✅ Done |
| 3 | Deep correctness analysis | [~] Partial — regex analysis done, Babel AST diff missing |
| 4 | Differential snapshot tests | [~] Partial — OXC snapshots done, Babel snapshots missing |
| 5 | CI integration | ✅ Done |
| 6 | README documentation | [~] Partial — usage done, correctness score docs missing |
