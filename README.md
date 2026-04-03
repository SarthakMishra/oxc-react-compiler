# oxc-react-compiler

Native [OXC](https://oxc.rs/) port of Meta's [React Compiler](https://github.com/facebook/react/tree/main/compiler/packages/babel-plugin-react-compiler) for the Rolldown/Vite pipeline, plus React 19 compiler-based lint rules for oxlint.

> **Status:** This is an active port — 140+ implementation phases covering HIR construction, SSA, type inference, mutation analysis, reactive scope inference, and codegen. Conformance is at 31.5% (540/1717 upstream fixtures) with 92% render equivalence (23/25 fixtures produce correct HTML). The compiler does not crash on any upstream fixture (0 panics). It is **not** production-ready but is progressing rapidly toward upstream parity.

## Vite Plugin Usage

### Installation

This is a native Rust package — prebuilt binaries are not yet published to npm. Install locally via [yalc](https://github.com/wclr/yalc):

```bash
# Requires: Rust toolchain (1.90+), Node.js, @napi-rs/cli
npm install -g yalc

git clone https://github.com/SarthakMishra/oxc-react-compiler
cd oxc-react-compiler/napi/react-compiler
npm install && npm run build
yalc publish

# Then in your project:
yalc add oxc-react-compiler
```

### Basic Setup

```ts
// vite.config.ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { reactCompiler } from "oxc-react-compiler/vite";

export default defineConfig({
  plugins: [
    // reactCompiler must come BEFORE the React plugin
    reactCompiler(),
    react(),
  ],
});
```

The compiler plugin replaces `babel-plugin-react-compiler` in the Vite pipeline. It runs as a `pre` plugin, transforming components and hooks before the React JSX transform.

### Options

```ts
reactCompiler({
  // How the compiler finds functions to compile (default: 'infer')
  compilationMode: "infer",

  // Output mode (default: 'client')
  outputMode: "client",

  // Enable/disable source maps (default: true in dev, false in build)
  sourceMap: true,
});
```

## Configuration Reference

### `compilationMode`

| Value          | Description                                                                                                       |
| -------------- | ----------------------------------------------------------------------------------------------------------------- |
| `'infer'`      | **(Default)** Compile functions that look like React components or hooks (PascalCase names, `use`-prefixed names) |
| `'all'`        | Compile all top-level functions                                                                                   |
| `'syntax'`     | Only compile functions containing the `"use memo"` directive                                                      |
| `'annotation'` | Same as `syntax` — only compile annotated functions                                                               |

### `outputMode`

| Value      | Description                                                                              |
| ---------- | ---------------------------------------------------------------------------------------- |
| `'client'` | **(Default)** Normal client-side compilation with `useMemoCache`-based memoization       |
| `'ssr'`    | Server-side rendering mode (skips client-specific optimizations)                         |
| `'lint'`   | Lint-only mode — runs analysis passes and collects diagnostics without transforming code |

### `target`

| Value       | Description                                |
| ----------- | ------------------------------------------ |
| `'react19'` | **(Default)** Target React 19 runtime APIs |
| `'react18'` | Target React 18                            |
| `'react17'` | Target React 17                            |

### `sourceMap`

| Value     | Description                                                                  |
| --------- | ---------------------------------------------------------------------------- |
| `true`    | Enable source map generation                                                 |
| `false`   | Disable source maps                                                          |
| _(unset)_ | **(Default)** Auto-detect: enabled in `vite serve`, disabled in `vite build` |

### `include` / `exclude`

Glob patterns to include or exclude specific files from compilation. By default, all `.tsx`, `.jsx`, `.ts`, `.js` files (excluding `node_modules`) are considered.

### `gating`

Feature flag configuration for wrapping compiled output:

```ts
reactCompiler({
  gating: {
    importSource: "my-flags",
    functionName: "isCompilerEnabled",
  },
});
```

### `cacheDir`

Directory for persisting the transform cache across builds. When set, the cache is written to `<cacheDir>/oxc-react-compiler-cache.json` and reloaded on the next build, skipping re-compilation of unchanged files. Leave unset to use only the in-memory cache.

```ts
reactCompiler({
  cacheDir: "node_modules/.cache",
});
```

## Lint Rules

### Tier 1: AST-Level Rules

These rules use static AST analysis (no compilation required):

| Rule                           | Description                                                             |
| ------------------------------ | ----------------------------------------------------------------------- |
| `rules-of-hooks`               | Hooks must be called at the top level, not in conditions/loops          |
| `no-jsx-in-try`                | JSX should not be used inside try blocks — use error boundaries instead |
| `no-ref-access-in-render`      | `ref.current` should not be read during render                          |
| `no-set-state-in-render`       | `setState` should not be called unconditionally during render           |
| `no-set-state-in-effects`      | Synchronous `setState` in effect bodies causes extra re-renders         |
| `use-memo-validation`          | `useMemo`/`useCallback` must have exactly 2 arguments                   |
| `no-capitalized-calls`         | PascalCase names should be JSX components, not called as functions      |
| `purity`                       | Detect impure function calls during render                              |
| `incompatible-library`         | Warn on libraries with known React incompatibilities                    |
| `static-components`            | Detect inline component definitions that cause remounting               |
| `no-deriving-state-in-effects` | Derived state should be computed during render, not in effects          |

### Tier 2: Compiler-Dependent Rules

These rules require the full compiler pipeline (HIR, effect system, reactive scopes). Use the `_with_source` API variants:

| Rule                                | Description                                                      |
| ----------------------------------- | ---------------------------------------------------------------- |
| `check_hooks_tier2`                 | Full Rules of Hooks with CFG analysis                            |
| `check_immutability`                | Detect mutation of frozen/immutable values                       |
| `check_preserve_manual_memoization` | Verify compiler preserves manual `useMemo`/`useCallback`         |
| `check_memo_dependencies`           | Exhaustive dependency checking for `useMemo`/`useCallback`       |
| `check_exhaustive_effect_deps`      | Exhaustive dependency checking for `useEffect`/`useLayoutEffect` |

### Using Lint Rules via NAPI

```ts
import { lintReactFile } from "oxc-react-compiler";

const result = lintReactFile(sourceCode, "component.tsx");
for (const diag of result.diagnostics) {
  console.log(`${diag.message} at ${diag.start}:${diag.end}`);
}
```

## Crates

| Crate                     | Description                                                     |
| ------------------------- | --------------------------------------------------------------- |
| `oxc_react_compiler`      | Core compiler — HIR, 65-pass pipeline, codegen                  |
| `oxc_react_compiler_lint` | Lint rules for oxlint (replaces `eslint-plugin-react-compiler`) |
| `oxc-react-compiler-napi` | NAPI-RS bindings + Vite plugin                                  |

## Development

```bash
# Check
cargo check

# Test (all crates)
cargo test

# Build NAPI bindings
cd napi/react-compiler && npm run build

# Run conformance tests (requires downloading upstream fixtures first)
./tests/conformance/download-upstream.sh
cargo test --test conformance_tests
```

## Architecture

The compiler implements a 65-pass compilation pipeline organized into 10 phases:

| Phase                   | Passes   | Description                                                                                                                          |
| ----------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| 0. Early Validation     | 1        | Reject unsupported patterns (getters, setters, for-await, new.target, `var` declarations)                                            |
| 1. Early Cleanup        | 2–7      | Prune throws, validate context variables, validate useMemo, drop manual memoization, inline IIFEs, merge blocks                      |
| 2. SSA                  | 8–9.6    | Enter SSA form, eliminate redundant phi nodes, prune temporary lvalues, inline LoadLocal temps                                        |
| 3. Optimization & Types | 10–12    | Constant propagation (with binary/unary folding), type inference, instruction kind rewriting                                         |
| 4. Hook Validation      | 13–14.6  | Validate hooks usage, no capitalized calls, no global reassignment, no eval                                                          |
| 5. Mutation/Aliasing    | 14–22    | Props optimization, function analysis, built-in/method signatures, mutation/aliasing effects, freeze validation, SSR opt, DCE, mutable ranges, last-use annotation |
| 6. Validation Battery   | 23–28.7  | Locals reassignment, ref access, setState, impurity, derived computations in effects, blocklisted imports, break targets             |
| 7. Reactivity           | 29–32    | Reactive place inference, exhaustive deps, unconditional blocks, property load hoisting, optional chains, static components           |
| 8. Scope Construction   | 33–46.5  | Reactive scope variables, scope membership propagation, fbt/macro scoping, JSX/function outlining, scope alignment, merging, dependency propagation, minimal deps |
| 9. RF Optimization      | 47–61    | Build reactive function tree, prune/merge/stabilize scopes, rename variables, prune hoisted contexts, validate memoization           |

## Benchmarks & Conformance

### Upstream Conformance

The compiler is tested against Meta's upstream React Compiler conformance suite — the same 1717 test fixtures used by `babel-plugin-react-compiler`. Output is compared structurally after normalizing semantics-irrelevant differences (import paths, variable naming, whitespace, cache variable names).

| Metric                      | Value        |
| --------------------------- | ------------ |
| Total upstream fixtures     | 1717         |
| Passing (exact match)       | 540 (31.5%)  |
| Failing (output divergence) | 1177         |
| Panics / crashes            | 0            |
| Render equivalence          | 92% (23/25)  |

#### Divergence Breakdown (~1222 known failures)

| Category                                 | Count | % of known |
| ---------------------------------------- | ----- | ---------- |
| Both compile, slots DIFFER               | 620   | 50.7%      |
| Both compile, slots MATCH (codegen diff) | 241   | 19.7%      |
| We compile, they don't (validation gaps) | 151   | 12.4%      |
| We bail, they compile                    | 119   | 9.7%       |
| Both no memo (format diff)               | 91    | 7.4%       |

> Note: In Phase 133, expected files were rebaselined with `compilationMode: "all"` (matching the upstream test suite). Phase 138 added Todo error detection for 5 categories of unsupported syntax (+15 fixtures). Phase 139 added frozen-mutation freeze propagation (phi nodes, store chains, property loads, iterators) gaining +9 fixtures. Phase 142 fixed ref-access validation to detect `.current` access after inline_load_local_temps eliminates LoadLocal intermediaries (+1 fixture). Phase 150 implemented validateInferredDep (source dep extraction and comparison) for preserve-memo validation (+3 fixtures). Phase 155 fixed preserve-memo validation by pre-computing HIR temp map before inline_load_locals, correcting Subpath comparison, and removing `is_temp_name` skip that suppressed all dep mismatch detection (+31 fixtures).

#### Bail-out Breakdown (119 fixtures where we bail but upstream compiles)

| Error                                 | Count |
| ------------------------------------- | ----- |
| Frozen-mutation false positives       | 15    |
| Cannot reassign outside component     | 11    |
| Preserve-memo false positives         | 7     |
| Ref-access in render false positives  | 6     |
| setState in effects                   | 7     |
| Local variable reassignment           | 7     |
| Cannot call setState during render    | 4     |
| Preserve-memo validation              | 4     |
| Silent bail-outs (no error)           | 4     |
| Hooks as normal values                | 3     |
| Extra effect dependencies             | 3     |
| Other                                 | 10    |

#### Slot Diff Distribution (688 fixtures where both compile but slot counts differ)

| Diff             | Count | Notes                     |
| ---------------- | ----- | ------------------------- |
| -1 (under-count) | 131   | Scope analysis gaps       |
| +1 (over-count)  | 123   | Extra scopes or deps      |
| +2               | 57    | Extra scopes              |
| -2               | 120   | Under-memoization         |
| other            | 257   |                           |

#### Key Divergence Patterns

Most of the 1222 failures fall into a few root causes:

- **Scope inference / codegen accuracy (620 fixtures)** — The dominant failure category. Both compilers compile the function but produce different slot counts. Improving mutable range propagation, scope merging, and codegen structure is the primary path to higher conformance.
- **Codegen structure (241 fixtures)** — Slot count matches upstream but code within scopes differs (ordering, scope boundaries, variable placement). Declaration placement and variable name preservation are the largest sub-patterns.
- **Missing validations (151 fixtures)** — We compile functions that upstream bails on. Down from 191 after Phase 155 fixed preserve-memo dep mismatch detection (+31 fixtures).
- **False-positive bail-outs (119 fixtures)** — We reject functions that upstream compiles successfully. Increased from 70 to 119 due to preserve-memo validation now correctly detecting dep mismatches for some non-error fixtures too. Many are legitimate preserve-memo errors that will be resolved by improving scope dep resolution.
- **Format-only divergences (91 fixtures)** — Neither side memoizes, but the output differs. Requires dead-code elimination and constant propagation passes.

Conformance runs as a non-blocking CI check — failures are tracked in `tests/conformance/known-failures.txt` and ratcheted as improvements land.

To run conformance tests locally:

```bash
./tests/conformance/download-upstream.sh
cargo test --release upstream_conformance -- --nocapture
```

### Memoization Benchmarks (OXC vs Babel)

The benchmark suite compiles 16 real-world React components through both OXC and the upstream Babel compiler, then structurally compares memoization patterns (cache slot count, scope blocks, dependency checks).

| Fixture               | Size | OXC Slots | Babel Slots | Delta | Status             |
| --------------------- | ---- | --------- | ----------- | ----- | ------------------ |
| simple-counter        | XS   | 4         | 2           | +2    | over-memoization   |
| status-badge          | XS   | 7         | 7           | 0     | cosmetic           |
| theme-toggle          | XS   | 7         | 4           | +3    | over-memoization   |
| avatar-group          | XS   | 14        | 10          | +4    | over-memoization   |
| search-input          | S    | 21        | 17          | +4    | over-memoization   |
| toolbar               | S    | 0         | 19          | -19   | bail-out           |
| todo-list             | S    | 23        | 24          | -1    | conservative miss  |
| form-validation       | S    | 34        | 48          | -14   | conservative miss  |
| time-slot-picker      | M    | 28        | 20          | +8    | over-memoization   |
| color-picker          | M    | 42        | 30          | +12   | over-memoization   |
| data-table            | M    | 37        | 56          | -19   | conservative miss  |
| command-menu          | M    | 52        | 37          | +15   | over-memoization   |
| booking-list          | L    | 51        | 62          | -11   | conservative miss  |
| canvas-sidebar        | L    | 81        | 70          | +11   | over-memoization   |
| availability-schedule | L    | 46        | 31          | +15   | over-memoization   |
| multi-step-form       | L    | 78        | 92          | -14   | conservative miss  |

**Divergence types:**

- **cosmetic** — OXC produces the same number of cache slots as Babel with minor structural differences (1 fixture).
- **conservative miss** — OXC compiles successfully but memoizes fewer values than Babel. Functionally correct but leaves optimization opportunities on the table (5 fixtures).
- **over-memoization** — OXC memoizes more than Babel (9 fixtures). Extra scopes or dependencies being included.
- **bail-out** — OXC bails on compilation while Babel compiles successfully (1 fixture: `toolbar`).

**Correctness Score**: 0.938. Bail-outs have been reduced from 4 to 1 fixture — `command-menu`, `availability-schedule`, and `multi-step-form` now compile successfully. The previously bailing `data-table` now produces 37 slots (vs Babel's 56).

### Compile Performance: OXC vs Babel (p50 latency)

All numbers measured on the 16-fixture benchmark suite (`--release` build, 30 iterations, 5 warmup). Both compilers process the same fixtures with equivalent configuration (JSX automatic runtime, TypeScript support, React compiler plugin).

| Fixture | Size | LOC | OXC p50 | Babel p50 | Speedup |
|---------|------|-----|---------|-----------|---------|
| simple-counter | XS | 8 | 124.0 µs | 8.29 ms | **66.8x** |
| theme-toggle | XS | 16 | 273.8 µs | 7.34 ms | **26.8x** |
| status-badge | XS | 21 | 175.8 µs | 8.24 ms | **46.9x** |
| avatar-group | XS | 23 | 6.95 ms | 11.58 ms | **1.7x** |
| todo-list | S | 35 | 763.3 µs | 27.92 ms | **36.6x** |
| form-validation | S | 50 | 75.42 ms | 29.95 ms | **0.4x** |
| search-input | S | 55 | 5.83 ms | 24.33 ms | **4.2x** |
| toolbar | S | 60 | 119.9 µs | 22.99 ms | **191.8x** |
| time-slot-picker | M | 81 | 25.73 ms | 23.98 ms | **0.9x** |
| data-table | M | 80 | 40.46 ms | 38.47 ms | **1.0x** |
| color-picker | M | 125 | 41.51 ms | 43.78 ms | **1.1x** |
| command-menu | M | 147 | 41.48 ms | 48.36 ms | **1.2x** |
| booking-list | L | 152 | 73.27 ms | 49.65 ms | **0.7x** |
| availability-schedule | L | 255 | 111.76 ms | 70.73 ms | **0.6x** |
| canvas-sidebar | L | 272 | 254.05 ms | 71.89 ms | **0.3x** |
| multi-step-form | L | 284 | 356.81 ms | 79.87 ms | **0.2x** |

**Aggregate**: median **1.1x**, mean 23.8x, range 0.2x–191.8x

> **Performance regression on larger fixtures:** The recent mutation range propagation and abstract interpreter rewrites (Phases 113–130) significantly improved conformance (+93 fixtures) but introduced O(n²+) scaling in the effects/aliasing analysis passes. Small fixtures (XS/S) are 5–67x faster than Babel, but medium/large fixtures now show regressions (0.2–1.2x). The `toolbar` outlier (192x) is a bail-out where OXC exits early.
>
> **Optimization opportunity:** The `infer_mutation_aliasing_effects` worklist-based fixpoint and `infer_mutation_aliasing_ranges` BFS passes are the primary bottlenecks. Profiling and algorithmic optimization of these passes is a high-priority item.

### Batch Project Build (End-to-End Throughput)

Simulates compiling an entire project — all 16 fixtures compiled sequentially as a single batch, measured 30 times.

| Metric | OXC | Babel |
|--------|-----|-------|
| Files compiled | 16 | 16 |
| Total LOC | 1,664 | 1,664 |
| Batch p50 | 1,047.24 ms | 522.79 ms |
| Batch p95 | 1,113.35 ms | 569.62 ms |
| Throughput | 1,589 LOC/s | 3,183 LOC/s |
| **Speedup** | **0.5x** | baseline |

> **Note:** OXC is currently slower than Babel in batch mode due to the O(n²+) scaling in mutation/aliasing analysis passes introduced in Phases 113–130. Small files are faster but large files dominate the batch total. See the per-fixture table above.

### Vite Dev Server Simulation

Simulates Vite's transform pipeline with content-hash caching — cold build (all files, no cache) and warm HMR rebuild (one file changed, rest cached).

| Scenario | OXC p50 | Babel p50 | Speedup |
|----------|---------|-----------|---------|
| Cold build (16 files, no cache) | 1,048.73 ms | 477.98 ms | **0.5x** |
| Warm HMR rebuild (1 file changed) | 359.93 ms | 75.22 ms | **0.2x** |

Changed file: `multi-step-form` (284 LOC, largest fixture)

> The warm HMR regression is because `multi-step-form` (the changed file) is the largest fixture and triggers the full effects pipeline. Smaller files would show faster HMR times.

### SSR Render Performance

Measures ReactDOMServer.renderToString() timing for original (uncompiled), OXC-compiled, and Babel-compiled output. This is a proxy for runtime performance — well-memoized code should render comparably to uncompiled code on initial render.

> **Note:** OXC-compiled output renders correctly for 23 of 25 benchmark fixtures (92% render equivalence). `command-menu` and `canvas-sidebar` have content divergences. Most OXC-compiled fixtures hit runtime errors during SSR benchmarking due to codegen differences (e.g., scope variable ordering, missing declarations).

| Fixture | Size | Original p50 | OXC p50 | Babel p50 |
|---------|------|-------------|---------|-----------|
| simple-counter | XS | 62.0 µs | 59.1 µs | 61.1 µs |
| theme-toggle | XS | 23.1 µs | 23.6 µs | 21.8 µs |
| todo-list | S | 92.0 µs | — | 91.0 µs |
| form-validation | S | 153.5 µs | — | 143.7 µs |
| data-table | M | 162.6 µs | — | 177.0 µs |
| color-picker | M | 13.6 µs | — | 11.0 µs |
| booking-list | L | 213.5 µs | — | 240.5 µs |
| availability-schedule | L | 201.0 µs | — | 206.5 µs |

Two OXC fixtures render successfully in SSR (`simple-counter` at 1.05x improvement, `theme-toggle` at 0.98x vs uncompiled). Babel-compiled output is within 1.04x of uncompiled on average — the memoization cache overhead roughly offsets any render savings on initial render, as expected (memoization benefits show on re-renders with unchanged deps, not measured in SSR).

### Real-World E2E Vite Builds

The e2e benchmark clones real open-source projects that use Vite + React, builds them with `babel-plugin-react-compiler` (baseline), then patches the Vite config to swap in the OXC plugin and rebuilds. All builds run 3 iterations; median is reported.

| Project | Scale | React Files | Babel Build | OXC Build | Speedup |
|---------|-------|-------------|-------------|-----------|---------|
| [ephe](https://github.com/unvalley/ephe) (PWA markdown editor) | small | 19 | 7.92s | 8.05s | **0.98x** |
| [rai-pal](https://github.com/Raicuparta/rai-pal) (Tauri game mod manager) | medium | 42 | 7.41s | 6.01s | **1.23x** |
| [arcomage-hd](https://github.com/arcomage/arcomage-hd) (web card game) | large | 62 | 13.09s | 11.25s | **1.16x** |
| [docmost](https://github.com/docmost/docmost) (collaborative wiki, 10.7K★) | large | 295 | 32.29s | 21.91s | **1.47x** |

#### Bundle Size Comparison

| Project | Babel JS | OXC JS | Delta |
|---------|----------|--------|-------|
| ephe | 2.8 MB | 2.9 MB | +10.9 KB (+0.4%) |
| rai-pal | 634.3 KB | 608.3 KB | -26.0 KB (-4.1%) |
| arcomage-hd | 845.0 KB | 809.9 KB | -35.1 KB (-4.2%) |
| docmost | 10.4 MB | 10.1 MB | -244.0 KB (-2.3%) |

#### OXC Transform Coverage

| Project | React Files | Compiled | Errors | Coverage |
|---------|------------|----------|--------|----------|
| ephe | 19 | 20 | 1 | 95% |
| rai-pal | 42 | 40 | 0 | 100% |
| arcomage-hd | 62 | 43 | 1 | 98% |
| docmost | 295 | 250 | 2 | 99% |

> **Coverage improvement**: OXC transform coverage has increased dramatically — from 26–57% to **95–100%** across all four projects. Nearly all React components are now compiled by OXC rather than falling through to uncompiled source.
>
> **Why are speedups lower on small projects?** With near-complete coverage, OXC now does more work per build (compiling ~all files vs previously skipping 43–74%). The smaller projects (ephe, rai-pal) have fewer React files so the per-file speedup advantage is offset by fixed build overhead. The speedup scales with project size as the compilation workload grows.
>
> **Why is OXC output smaller?** OXC's memoization produces fewer cache slots than Babel in most cases (see memoization benchmarks above), resulting in less `useMemoCache` overhead. The exception is `ephe` where OXC's over-memoization adds slightly more code than Babel.
>
> **Scaling trend**: The speedup increases with project size — from near-parity on a 19-file project to **1.47x on a 295-file monorepo** (docmost). This demonstrates that OXC's native Rust performance advantage compounds as compilation workload grows.
>
> **Note:** These E2E numbers were measured before the Phases 113–130 performance regression in the effects/aliasing analysis. Real-world build speedups may have decreased for projects with many large components. Re-benchmarking is pending the performance optimization work.

### Running Benchmarks

```bash
# Build NAPI binding first
cd napi/react-compiler && npm install && npx napi build --release

# Comparative benchmark (OXC vs Babel — compile speed + batch + Vite sim)
cd ../../benchmarks && node scripts/bench-compare.mjs

# Per-fixture OXC-only latency
node bench.mjs

# Memoization structural comparison
node scripts/babel-compile.mjs --diff

# SSR render timing comparison
node scripts/runtime-bench.mjs

# Render equivalence (HTML output comparison)
node scripts/render-compare.mjs

# E2E real-world project builds (clones repos, ~5 min)
node e2e/e2e-bench.mjs

# Quick run (fewer iterations for CI)
node scripts/bench-compare.mjs --iterations 20 --warmup 5
```

## Known Limitations

### General

- **Active development** — Upstream conformance is at 31.5% (540/1717 fixtures) with 92% render equivalence (23/25 fixtures produce correct HTML output). The compiler does not crash on any upstream fixture (0 panics), but output frequently diverges from the reference implementation in structure (cache slot counts, scope boundaries, validation gaps).
- **Performance regression on large files** — The mutation/aliasing analysis passes (Phases 113–130) introduced O(n²+) scaling. Small components compile 5–67x faster than Babel, but large components (150+ LOC) are currently slower. This is the highest-priority optimization target.
- **No oxlint integration** — Lint rules exist in `crates/oxc_react_compiler_lint` and are callable via the NAPI binding, but they are not integrated into the oxlint binary. This would require upstream work in the [oxc repo](https://github.com/oxc-project/oxc) to support external plugin crates.
- **Source maps** — Source map generation covers compiled function regions with per-line identity mappings for unmodified code. Complex source map chaining with other Vite plugins has not been verified.

### Memoization & Scope Analysis

- **Validation gaps (677 fixtures)** — The dominant failure category. We compile functions that upstream correctly rejects. Missing validation checks need to be ported from upstream.
- **Slot count divergences (690 fixtures)** — Both sides compile but reactive scope computation produces different cache slot counts. Root cause: `effective_range` approximation differs from upstream's pure mutation BFS. An `use_mutable_range` A/B flag exists but net-regresses as mutable ranges are still too narrow.
- **Codegen structure (86 fixtures)** — Slot count matches upstream but code within scopes differs. Improved from 240 through formatting fixes (const/let, dead call elimination, dependency ordering, dot notation, optional chaining).
- **Manual memoization preservation** — Reduced from 58 to 4 false positives through preserve-memo validation relaxation (Phase 124).

### Validation

- **False-positive bail-outs (44 fixtures)** — We reject functions that upstream compiles successfully. Down from 135 through Flow preprocessing, preserve-memo relaxation, frozen-mutation tuning, and ref-access fixes.
- **Upstream errors we miss (677 fixtures)** — Validation gaps where upstream correctly bails but we compile successfully. This is now the largest gap.

### Other

- **@flow fixtures** — Flow component/hook syntax is preprocessed into standard function declarations (Phase 128). Some Flow type annotations still cause parse errors — these are permanently skipped as Flow is deprecated.

## License

MIT
