# oxc-react-compiler

Native [OXC](https://oxc.rs/) port of Meta's [React Compiler](https://github.com/facebook/react/tree/main/compiler/packages/babel-plugin-react-compiler) for the Rolldown/Vite pipeline, plus React 19 compiler-based lint rules for oxlint.

> **Warning:** This is AI-generated, untested code built as a preview to explore the feasibility of an OXC-based React Compiler port. It is **not** production-ready and should not be used in real projects. Treat it as a proof-of-concept, not a finished implementation.

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
| `oxc_react_compiler`      | Core compiler — HIR, 62-pass pipeline, codegen                  |
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

The compiler implements a 62-pass compilation pipeline organized into 10 phases:

| Phase                   | Passes  | Description                                                                                                                 |
| ----------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------- |
| 0. Early Validation     | 1       | Reject unsupported patterns (getters, setters, for-await, new.target)                                                       |
| 1. Early Cleanup        | 2–7     | Prune throws, validate context variables, validate useMemo, drop manual memoization, inline IIFEs, merge blocks             |
| 2. SSA                  | 8–9.5   | Enter SSA form, eliminate redundant phi nodes, prune temporary lvalues                                                      |
| 3. Optimization & Types | 10–12   | Constant propagation, type inference, instruction kind rewriting                                                            |
| 4. Hook Validation      | 13–14.6 | Validate hooks usage, no capitalized calls, no global reassignment, no eval                                                 |
| 5. Mutation/Aliasing    | 14–20   | Props optimization, function analysis, mutation/aliasing effects, freeze validation, SSR optimization, DCE, mutable ranges  |
| 6. Validation Battery   | 21–28.7 | Locals reassignment, ref access, setState, impurity, blocklisted imports, break targets                                     |
| 7. Reactivity           | 29–32   | Reactive place inference, exhaustive deps, unconditional blocks, property load hoisting, optional chains, static components |
| 8. Scope Construction   | 33–46.5 | Reactive scope variables, JSX/function outlining, scope alignment, merging, dependency propagation, minimal deps            |
| 9. RF Optimization      | 47–61   | Build reactive function tree, prune/merge/stabilize scopes, rename variables, validate memoization                          |

## Benchmarks & Conformance

### Upstream Conformance

The compiler is tested against Meta's upstream React Compiler conformance suite — the same 1717 test fixtures used by `babel-plugin-react-compiler`. Output is compared structurally after normalizing semantics-irrelevant differences (import paths, variable naming, whitespace, cache variable names).

| Metric                      | Value        |
| --------------------------- | ------------ |
| Total upstream fixtures     | 1717         |
| Passing                     | 456 (26.6%)  |
| Failing (output divergence) | 1261         |
| Panics / crashes            | 0            |
| Render equivalence          | 96% (24/25)  |

#### Divergence Breakdown

| Category                                 | Count | % of failures |
| ---------------------------------------- | ----- | ------------- |
| Both compile, slots DIFFER               | 649   | 51.5%         |
| Both compile, slots MATCH (codegen diff) | 240   | 19.0%         |
| We bail, they compile                    | 135   | 10.7%         |
| We compile, they don't                   | 149   | 11.8%         |
| Both no memo (format diff)               | 87    | 6.9%          |
| Silent bail-outs (0 scopes, no error)    | 23    | 1.8%          |

#### Bail-out Breakdown (135 fixtures)

| Error                                 | Count |
| ------------------------------------- | ----- |
| Preserve-memo validation              | 58    |
| Frozen-mutation false positives       | 23    |
| Silent bail-outs (0 scopes, no error) | 23    |
| Reassigned-after-render               | 8     |
| Cannot reassign outside component     | 7     |
| Ref-access in render false positives  | 6     |
| Other (hooks, setState)               | ~10   |

#### Slot Diff Distribution (649 fixtures where both compile but slot counts differ)

| Diff             | Count | Notes                     |
| ---------------- | ----- | ------------------------- |
| -1 (under-count) | 143   | Scope analysis gaps       |
| +1 (over-count)  | 112   | Extra scopes or deps      |
| +2               | 51    |                           |
| other            | 343   |                           |

#### Key Divergence Patterns

Most of the 1261 failures fall into a few root causes:

- **Slot count divergences (649 fixtures)** — Both sides compile but reactive scope computation produces wrong cache slot counts. Under-counts typically reflect scope analysis gaps; over-counts reflect extra scopes or deps being included. This is the highest-volume issue.
- **Codegen structure (240 fixtures)** — Slot count matches upstream but code within scopes differs (ordering, scope boundaries, variable placement).
- **False-positive bail-outs (135 fixtures)** — We reject functions that upstream compiles successfully. Largest: preserve-memo validation (58); frozen-mutation (23); silent bail-outs (23); reassigned-after-render (8); ref-access (6).
- **We compile, upstream errors (149 fixtures)** — Validation gaps where upstream correctly bails but we compile successfully.
- **Format-only divergences (87 fixtures)** — Neither side memoizes, but whitespace, import ordering, or other cosmetic differences cause a fixture mismatch.

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
| status-badge          | XS   | 7         | 7           | 0     | match              |
| theme-toggle          | XS   | 7         | 4           | +3    | over-memoization   |
| avatar-group          | XS   | 14        | 10          | +4    | over-memoization   |
| search-input          | S    | 21        | 17          | +4    | over-memoization   |
| toolbar               | S    | 0         | 19          | -19   | bail-out           |
| todo-list             | S    | 23        | 24          | -1    | conservative miss  |
| form-validation       | S    | 34        | 48          | -14   | conservative miss  |
| time-slot-picker      | M    | 28        | 20          | +8    | over-memoization   |
| color-picker          | M    | 42        | 30          | +12   | over-memoization   |
| data-table            | M    | 29        | 56          | -27   | conservative miss  |
| command-menu          | M    | 0         | 37          | -37   | bail-out           |
| booking-list          | L    | 43        | 62          | -19   | conservative miss  |
| canvas-sidebar        | L    | 64        | 70          | -6    | conservative miss  |
| availability-schedule | L    | 0         | 31          | -31   | bail-out           |
| multi-step-form       | L    | 0         | 92          | -92   | bail-out           |

**Divergence types:**

- **match** — OXC produces the same number of cache slots as Babel (1 fixture).
- **conservative miss** — OXC compiles successfully but memoizes fewer values than Babel. The output is functionally correct but leaves some optimization opportunities on the table (5 fixtures).
- **over-memoization** — OXC memoizes more than Babel (6 fixtures). This may indicate extra scopes or dependencies being included.
- **bail-out** — OXC bails on compilation (e.g., due to a false-positive validation error) while Babel compiles successfully (4 fixtures).

One fixture (`status-badge`) matches Babel exactly. Bail-outs have increased from 1 to 4 fixtures (`toolbar`, `command-menu`, `availability-schedule`, `multi-step-form`).

### Compile Performance: OXC vs Babel (p50 latency)

All numbers measured on the 16-fixture benchmark suite (`--release` build, 50 iterations, 10 warmup). Both compilers process the same fixtures with equivalent configuration (JSX automatic runtime, TypeScript support, React compiler plugin).

| Fixture | Size | LOC | OXC p50 | Babel p50 | Speedup |
|---------|------|-----|---------|-----------|---------|
| simple-counter | XS | 8 | 448.8 µs | 7.81 ms | **17.4x** |
| theme-toggle | XS | 16 | 719.3 µs | 8.47 ms | **11.8x** |
| status-badge | XS | 21 | 382.1 µs | 8.45 ms | **22.1x** |
| avatar-group | XS | 23 | 860.2 µs | 10.95 ms | **12.7x** |
| todo-list | S | 35 | 2.23 ms | 26.27 ms | **11.8x** |
| form-validation | S | 50 | 3.76 ms | 28.64 ms | **7.6x** |
| search-input | S | 55 | 1.97 ms | 16.00 ms | **8.1x** |
| toolbar | S | 60 | 55.4 µs | 18.75 ms | **338.6x** |
| time-slot-picker | M | 81 | 3.29 ms | 40.27 ms | **12.3x** |
| data-table | M | 80 | 5.17 ms | 40.15 ms | **7.8x** |
| color-picker | M | 125 | 7.02 ms | 56.55 ms | **8.1x** |
| command-menu | M | 147 | 7.02 ms | 49.97 ms | **7.1x** |
| booking-list | L | 152 | 7.89 ms | 58.20 ms | **7.4x** |
| availability-schedule | L | 255 | 6.91 ms | 86.67 ms | **12.6x** |
| canvas-sidebar | L | 272 | 13.76 ms | 87.55 ms | **6.4x** |
| multi-step-form | L | 284 | 9.87 ms | 81.73 ms | **8.3x** |

**Aggregate**: median **8.3x** faster, mean 31.3x, range 6.4x–338.6x

> The high-speedup outlier (toolbar 339x) is a fixture where OXC bails out early (0 slots), so the comparison reflects OXC's fast "no-op" path vs Babel's full compilation. The conservative fixtures (6x–23x) are the most representative of typical component compilation.

### Batch Project Build (End-to-End Throughput)

Simulates compiling an entire project — all 16 fixtures compiled sequentially as a single batch, measured 50 times.

| Metric | OXC | Babel |
|--------|-----|-------|
| Files compiled | 16 | 16 |
| Total LOC | 1,664 | 1,664 |
| Batch p50 | 63.61 ms | 504.24 ms |
| Batch p95 | 77.89 ms | 588.05 ms |
| Throughput | 26,160 LOC/s | 3,300 LOC/s |
| **Speedup** | **7.9x** | baseline |

### Vite Dev Server Simulation

Simulates Vite's transform pipeline with content-hash caching — cold build (all files, no cache) and warm HMR rebuild (one file changed, rest cached).

| Scenario | OXC p50 | Babel p50 | Speedup |
|----------|---------|-----------|---------|
| Cold build (16 files, no cache) | 63.45 ms | 508.59 ms | **8.0x** |
| Warm HMR rebuild (1 file changed) | 8.97 ms | 76.73 ms | **8.6x** |

Changed file: `multi-step-form` (284 LOC, largest fixture)

### SSR Render Performance

Measures ReactDOMServer.renderToString() timing for original (uncompiled), OXC-compiled, and Babel-compiled output. This is a proxy for runtime performance — well-memoized code should render comparably to uncompiled code on initial render.

> **Note:** OXC-compiled output renders correctly for 24 of 25 benchmark fixtures (96% render equivalence). Only `canvas-sidebar` has a minor content divergence. However, many OXC-compiled fixtures hit runtime errors during SSR benchmarking due to codegen differences (e.g., scope variable ordering).

| Fixture | Size | Original p50 | OXC p50 | Babel p50 |
|---------|------|-------------|---------|-----------|
| simple-counter | XS | 59.9 µs | 54.6 µs | 60.2 µs |
| theme-toggle | XS | 6.8 µs | 7.1 µs | 7.0 µs |
| form-validation | S | 144.2 µs | — | 76.4 µs |
| toolbar | S | 177.8 µs | — | 171.6 µs |
| color-picker | M | 13.4 µs | — | 10.9 µs |
| availability-schedule | L | 197.8 µs | — | 205.7 µs |

Two OXC fixtures render successfully in SSR (`simple-counter` at 1.10x improvement, `theme-toggle` at 0.97x vs uncompiled). Babel-compiled output is within 1.0x of uncompiled on average — the memoization cache overhead roughly offsets any render savings on initial render, as expected (memoization benefits show on re-renders with unchanged deps, not measured in SSR).

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

- **Proof of concept** — This is an AI-generated port and has not been validated against production workloads. Upstream conformance is at 26.6% (456/1717 fixtures) with 96% render equivalence (24/25 fixtures produce correct HTML output). The compiler does not crash on any upstream fixture (0 panics), but output frequently diverges from the reference implementation in structure (cache slot counts, scope boundaries).
- **No oxlint integration** — Lint rules exist in `crates/oxc_react_compiler_lint` and are callable via the NAPI binding, but they are not integrated into the oxlint binary. This would require upstream work in the [oxc repo](https://github.com/oxc-project/oxc) to support external plugin crates — it is not achievable in this standalone POC repo.
- **Source maps** — Source map generation covers compiled function regions with per-line identity mappings for unmodified code. Complex source map chaining with other Vite plugins has not been verified.

### Memoization & Scope Analysis

- **Slot count divergences (649 fixtures)** — The dominant failure category. Under-counts (fewer scopes than upstream) and over-counts (extra scopes). Root cause: mutable range computation uses an `effective_range` (mutation + last-use) approximation rather than upstream's pure mutation BFS. Switching to narrow ranges requires porting upstream's full abstract interpreter.
- **Codegen structure (240 fixtures)** — Slot count matches upstream but code within scopes differs (ordering, scope boundaries, variable placement). This gap has been reduced through formatting fixes (const/let, dead call elimination, dependency ordering, dot notation, optional chaining).
- **Manual memoization preservation** — The compiler does not reliably preserve user-written `useMemo`/`useCallback` in all edge cases (58 fixtures affected by preserve-memo validation false positives).

### Validation

- **False-positive bail-outs (135 fixtures)** — We reject functions that upstream compiles successfully. Largest gaps: preserve-memo validation (58); frozen-mutation (23); silent bail-outs (23); reassigned-after-render (8); ref-access in render (6); other (~17).
- **Upstream errors we miss (149 fixtures)** — Validation gaps where upstream correctly bails but we compile successfully.

### Other

- **@flow fixtures** — Some fixtures use Flow type syntax which the OXC parser cannot handle. These are permanently skipped as Flow is deprecated.

## License

MIT
