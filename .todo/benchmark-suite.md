# Shareable Benchmark Suite

> A benchmark suite comparing OXC React Compiler vs Babel on real-world React components,
> with deep correctness analysis, differential snapshots, and CI regression tracking.

---

## Gap 1: Real-world fixture extraction pipeline

**Upstream:** N/A (OXC-specific tooling)
**Current state:** No benchmark fixtures exist. The original plan proposed hand-rolled micro-components, which are too small and synthetic to stress the compiler meaningfully.

**What's needed:**

- Create `benchmarks/fixtures/` directory with components extracted from real OSS repositories, pinned at specific commits for reproducibility:
  - **cal.com** (calcom/cal.com) -- scheduling app, heavy hook usage, complex form state
  - **excalidraw** (excalidraw/excalidraw) -- canvas app, useEffect-heavy, custom hooks
  - **shadcn/ui** (shadcn-ui/ui) -- component library, clean patterns, good baseline
- Extraction script (`benchmarks/scripts/extract-fixtures.sh`) that:
  1. Clones each repo at the pinned commit SHA into a temp directory
  2. Copies selected component files into `benchmarks/fixtures/{repo-name}/`
  3. Strips non-essential imports (replace with stubs) so files compile standalone
  4. Records provenance metadata in `benchmarks/fixtures/manifest.json` (repo, commit SHA, file path, extraction date)
- Categorize every fixture by size tier based on LOC and hook count:
  - **XS** (< 50 LOC, 0-1 hooks) -- baseline sanity checks
  - **S** (< 150 LOC, 1-3 hooks) -- typical leaf components
  - **M** (< 400 LOC, 3-6 hooks) -- mid-complexity pages/forms
  - **L** (1000+ LOC, 6+ hooks) -- dashboards, data tables, complex views
- Include at least 2-3 components that the OXC compiler currently fails on or produces divergent output for (document these in `manifest.json` with `"known_divergent": true`)
- Minimum 15 fixtures total, at least 3 per size tier
- Each fixture must be a single file that both Babel and OXC can attempt to compile (may import React types but no project-specific imports)

**Depends on:** None

---

## Gap 2: Benchmark harness v2 (speed, memory, separated overhead)

**Upstream:** N/A
**Current state:** No benchmark infrastructure exists. The NAPI binding (`napi/react-compiler/src/lib.rs`) exposes `transform_react_file()` but does not separate marshalling time from Rust compilation time.

**What's needed:**

### 2a: Rust-side timer exposed via NAPI

- Add a new NAPI function `transform_react_file_timed()` in `napi/react-compiler/src/lib.rs` that returns an extended result:
  ```
  TransformTimedResult {
    code: String,
    transformed: bool,
    source_map: Option<String>,
    rust_compile_ns: i64,   // Rust-side std::time::Instant measurement (nanoseconds)
  }
  ```
- The Rust-side timer wraps only the `compile_program()` call, excluding NAPI argument marshalling and result serialization
- This lets the benchmark harness report both total wall-clock (including NAPI overhead) and pure Rust compilation time

### 2b: Node.js benchmark harness

- Create `benchmarks/bench.mjs` -- the main benchmark entry point
- For each fixture, run both compilers:
  - **OXC:** via the NAPI `transform_react_file_timed()` binding
  - **Babel:** via `@babel/core` `transformSync` with `babel-plugin-react-compiler`
- Measurement methodology:
  - **Warmup:** Run 20 iterations per fixture, discard all results (JIT warmup for Babel, cold-start for NAPI)
  - **Measured runs:** 100 iterations (configurable via `--iterations N`)
  - Report: min, p50, p95, p99, max for both wall-clock and Rust-only time
  - **Batch mode:** `--batch` flag compiles all fixtures in sequence per iteration (simulates bundler compilation of an entire project), reports aggregate time
- **Memory tracking:**
  - Track peak RSS using `process.memoryUsage().rss` before/after each compilation batch
  - For Rust-side memory, expose peak allocator bytes via a NAPI helper if feasible (stretch goal; RSS is the minimum)
- Output formats:
  - `--format markdown` -- table for GitHub PR comments (default)
  - `--format json` -- machine-readable for CI consumption, written to `benchmarks/results.json`
- CLI: `node benchmarks/bench.mjs [--iterations N] [--batch] [--format markdown|json] [--filter pattern]`

**Depends on:** Gap 1 (fixtures must exist)

---

## Gap 3: Deep correctness analysis

**Upstream:** N/A
**Current state:** The original plan only proposed token-level grep for `_c(N)` and `$[N]` patterns. This is too shallow to detect structural memoization differences or semantic regressions.

**What's needed:**

### 3a: Structural AST diffing

- Parse both OXC and Babel outputs into ASTs (using `@babel/parser` or `acorn`)
- Compare at the structural level, not token level:
  - **Memoization block parity:** count and match `useMemoCache(N)` calls -- same N value, same position in function body
  - **Cache slot parity:** for each `$[i]` access, verify the slot index and the cached expression are structurally equivalent
  - **Scope boundary alignment:** verify that memoized regions wrap the same set of statements/expressions
  - **Dependency array equivalence:** if the compiler emits conditional checks (`if ($[0] !== dep`), verify the same dependencies are checked
- Output a structured diff report per fixture:
  ```
  { fixture: "cal.com/BookingForm.tsx",
    memoBlockCount: { oxc: 3, babel: 3 },
    slotCount: { oxc: 8, babel: 8 },
    structuralMatch: true,
    divergences: [] }
  ```

### 3b: Semantic equivalence via headless render

- For each fixture that exports a renderable component:
  - Render with `@testing-library/react` + `jsdom` under identical props/state sequences
  - Drive a sequence of interactions (click handlers, state updates) defined per fixture in `benchmarks/fixtures/{name}.interactions.mjs`
  - Assert that HTML output is identical between OXC-compiled and Babel-compiled versions at each step
  - This catches cases where structural differences lead to actual behavioral divergence
- Not all fixtures will be renderable (some may have unresolvable imports); skip gracefully and note in the report

### 3c: Divergence classification

- Every detected difference between OXC and Babel output must be classified into one of:
  - **Conservative miss:** OXC does not memoize something Babel does (safe but suboptimal)
  - **Over-memoization:** OXC memoizes something Babel does not (potentially unsafe if dependencies are wrong)
  - **Semantic difference:** the compiled output would produce different runtime behavior (this is a bug)
  - **Cosmetic:** whitespace, variable naming, import ordering differences with no semantic impact
- Classification is automated where possible (structural diff can detect conservative miss vs over-memoization) and flagged for manual review otherwise
- The correctness report aggregates counts by classification type

**Depends on:** Gap 1 (fixtures), Gap 2 (harness runs both compilers)

---

## Gap 4: Differential snapshot tests

**Upstream:** N/A
**Current state:** The conformance test infrastructure (`tests/conformance/`) has a pattern for generating `.expected` files from upstream Babel, but no committed snapshots exist for benchmark fixtures.

**What's needed:**

- `benchmarks/snapshots/` directory with committed expected outputs from both compilers:
  - `benchmarks/snapshots/{fixture-name}.babel.js` -- Babel's output
  - `benchmarks/snapshots/{fixture-name}.oxc.js` -- OXC's output
  - `benchmarks/snapshots/{fixture-name}.diff.json` -- structural diff report from Gap 3a
- Update workflow: `npm run bench:update-snapshots`
  - Regenerates all snapshot files from current compiler outputs
  - These changes appear in `git diff` and must be human-reviewed before merging
  - This creates an audit trail: every PR that changes compiler output will show snapshot diffs
- Snapshot comparison in CI:
  - `npm run bench:check-snapshots` verifies that committed snapshots match current output
  - If snapshots diverge, CI fails with a clear message: "Compiler output changed. Run `npm run bench:update-snapshots` and review the diff."
- The snapshots serve as a regression safety net independent of the correctness score -- even if the score stays the same, a snapshot change signals that compiler behavior shifted

**Depends on:** Gap 1 (fixtures), Gap 2 (harness to generate outputs)

---

## Gap 5: CI integration

**Upstream:** N/A
**Current state:** No CI benchmark step exists.

**What's needed:**

### 5a: Dedicated runner and baseline management

- GitHub Actions workflow `.github/workflows/benchmark.yml`
- Runner requirements:
  - Use a pinned self-hosted runner or a dedicated GitHub-hosted runner size (e.g., `ubuntu-latest-8-cores`) -- shared default runners have too much variance for a 10% regression threshold
  - If self-hosted runners are not available initially, use `ubuntu-latest` but increase the regression threshold to 20% and add a "noisy" warning label
- **Baseline storage:** committed `benchmarks/baseline.json` file updated on every merge to `main`
  - Contains per-fixture timing (p50, p95) and correctness metrics from the last main build
  - PR benchmark compares against this baseline, not against the previous commit
  - Baseline update is a separate CI job triggered only on main merges: `npm run bench --format json > benchmarks/baseline.json`

### 5b: Per-fixture and per-divergence-type failure tracking

- CI benchmark output includes a per-fixture table:
  ```
  | Fixture             | Size | OXC p50 | Babel p50 | Speedup | Correctness | Divergences         |
  |---------------------|------|---------|-----------|---------|-------------|---------------------|
  | cal.com/BookingForm | M    | 2.1ms   | 18.3ms    | 8.7x    | PASS        | 0                   |
  | excalidraw/Canvas   | L    | 5.4ms   | 42.1ms    | 7.8x    | WARN        | 1 conservative miss |
  ```
- Failure conditions (CI blocks the PR):
  - Any fixture with a **semantic difference** (classification from Gap 3c)
  - Any fixture where OXC p50 regressed > threshold vs baseline
  - Snapshot mismatch (from Gap 4) without explicit snapshot update
- Warning conditions (CI passes but posts a comment):
  - New conservative misses (OXC memoizes less than Babel)
  - New over-memoization instances
  - Memory usage increase > 25% vs baseline
- Post results as a PR comment using `github-script` or `peter-evans/create-or-update-comment`

### 5c: Snapshot verification in CI

- Run `npm run bench:check-snapshots` as a separate CI step
- Fail if snapshots are stale (compiler output changed but snapshots were not updated)

**Depends on:** Gap 2 (harness), Gap 3 (correctness analysis), Gap 4 (snapshots)

---

## Gap 6: README and correctness score documentation

**Upstream:** N/A
**Current state:** No benchmark documentation exists.

**What's needed:**

- `benchmarks/README.md` covering:

### Usage

- Prerequisites: Node.js >= 18, Rust toolchain (for building NAPI from source), npm dependencies
- Quick start: `cd benchmarks && npm install && npm run bench`
- Full suite: `npm run bench -- --batch --format json`
- Update snapshots: `npm run bench:update-snapshots`
- Check snapshots: `npm run bench:check-snapshots`
- Filter by fixture: `npm run bench -- --filter "cal.com"`

### How to add new fixtures

- Extract from an OSS repo, pin the commit SHA
- Add to `benchmarks/fixtures/manifest.json`
- Assign size tier and optionally add an interactions file
- Run `npm run bench:update-snapshots` and commit the new snapshots

### What the correctness score means

- Document the four divergence classifications (conservative miss, over-memoization, semantic difference, cosmetic)
- Explain which are acceptable and which are bugs:
  - **Cosmetic:** always acceptable, filtered out of reports
  - **Conservative miss:** acceptable during development, tracked for parity roadmap
  - **Over-memoization:** warning-level, must be investigated to confirm no semantic impact
  - **Semantic difference:** always a bug, blocks CI
- Provide a "known acceptable divergences" section listing any intentional differences from Babel output with rationale
- Link to the structural AST diff methodology (what is compared, what is ignored)

### How to interpret performance numbers

- Explain wall-clock vs Rust-only time and what NAPI marshalling overhead means
- Explain warmup methodology and why p50/p95 are the primary metrics
- Explain batch mode vs single-file mode

**Depends on:** Gap 1-5 (documents the full system)
