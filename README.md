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

| Metric                      | Value       |
| --------------------------- | ----------- |
| Total upstream fixtures     | 1717        |
| Passing                     | 422 (24.6%) |
| Failing (output divergence) | 1295        |
| Panics / crashes            | 0           |

#### Divergence Breakdown

| Category                                 | Count | % of failures |
| ---------------------------------------- | ----- | ------------- |
| Both compile, slots DIFFER               | 564   | 43.6%         |
| Both compile, slots MATCH (codegen diff) | 268   | 20.7%         |
| We bail, they compile                    | 243   | 18.8%         |
| We compile, they don't                   | 122   | 9.4%          |
| Both no memo (format diff)               | 98    | 7.6%          |

#### Bail-out Breakdown (243 fixtures)

| Error                                 | Count |
| ------------------------------------- | ----- |
| Frozen-mutation false positives       | 86    |
| Silent bail-outs (0 scopes, no error) | 66    |
| Preserve-memo validation              | 35    |
| Locals-reassigned false positives     | 25    |
| Ref-access in render false positives  | 13    |
| Other (globals, hooks, setState)      | 18    |

#### Slot Diff Distribution (564 fixtures where both compile but slot counts differ)

| Diff             | Count | Notes                                     |
| ---------------- | ----- | ----------------------------------------- |
| -1 (under-count) | 125   | Down from 159 after param destructure fix |
| -2               | 97    |                                           |
| -3 to -23        | 134   | Major scope analysis gaps                 |
| +1 (over-count)  | 98    | Up from 80 after param destructure fix    |
| +2               | 47    |                                           |
| +3 to +11        | 63    |                                           |

#### Key Divergence Patterns

Most of the 1295 failures fall into a few root causes:

- **Slot count divergences (564 fixtures)** — Both sides compile but reactive scope computation produces wrong cache slot counts. Under-counts (356 fixtures) typically reflect scope analysis gaps; over-counts (208 fixtures) reflect extra scopes or deps being included. This is the highest-volume issue.
- **Codegen structure (268 fixtures)** — Slot count matches upstream but code within scopes differs (ordering, scope boundaries, variable placement).
- **False-positive bail-outs (243 fixtures)** — We reject functions that upstream compiles successfully. Largest: frozen-mutation validator (86) uses name-based heuristics instead of actual mutable ranges; silent bail-outs (66) produce 0 scopes with no error; preserve-memo validation (35); locals-reassigned (25); ref-access (13).
- **We compile, upstream errors (122 fixtures)** — Validation gaps where upstream correctly bails but we compile successfully.
- **Format-only divergences (98 fixtures)** — Neither side memoizes, but whitespace, import ordering, or other cosmetic differences cause a fixture mismatch.

Conformance runs as a non-blocking CI check — failures are tracked in `tests/conformance/known-failures.txt` and ratcheted as improvements land.

To run conformance tests locally:

```bash
./tests/conformance/download-upstream.sh
cargo test --release upstream_conformance -- --nocapture
```

### Memoization Benchmarks (OXC vs Babel)

The benchmark suite compiles 16 real-world React components through both OXC and the upstream Babel compiler, then structurally compares memoization patterns (cache slot count, scope blocks, dependency checks).

> **Note:** Some slot counts changed significantly from earlier README values (e.g., `canvas-sidebar` 50→20, `booking-list` 23→9). The param destructure fix changed scope boundaries — scopes that previously included destructured params now correctly exclude them. Memoization is more correct (deps are tracked instead of sentinel), but overall slot counts are lower because the scopes are narrower.

| Fixture               | Size | OXC Slots | Babel Slots | Delta | Status            |
| --------------------- | ---- | --------- | ----------- | ----- | ----------------- |
| simple-counter        | XS   | 3         | 2           | +1    | over-memoization  |
| theme-toggle          | XS   | 2         | 4           | -2    | conservative miss |
| status-badge          | XS   | 3         | 7           | -4    | conservative miss |
| avatar-group          | XS   | 3         | 10          | -7    | conservative miss |
| search-input          | S    | 4         | 17          | -13   | conservative miss |
| toolbar               | S    | 3         | 19          | -16   | conservative miss |
| todo-list             | S    | 7         | 24          | -17   | conservative miss |
| form-validation       | S    | 7         | 48          | -41   | conservative miss |
| time-slot-picker      | M    | 6         | 20          | -14   | conservative miss |
| color-picker          | M    | 8         | 30          | -22   | conservative miss |
| data-table            | M    | 11        | 56          | -45   | conservative miss |
| command-menu          | M    | 11        | 37          | -26   | conservative miss |
| booking-list          | L    | 9         | 62          | -53   | conservative miss |
| canvas-sidebar        | L    | 20        | 70          | -50   | conservative miss |
| availability-schedule | L    | 0         | 31          | -31   | bail-out          |
| multi-step-form       | L    | 12        | 92          | -80   | conservative miss |

**Divergence types:**

- **match** — OXC produces the same number of cache slots as Babel.
- **conservative miss** — OXC compiles successfully but memoizes fewer values than Babel. The output is functionally correct but leaves some optimization opportunities on the table.
- **over-memoization** — OXC memoizes more than Babel (minor; 1 fixture).
- **bail-out** — OXC bails on compilation (e.g., due to a false-positive validation error) while Babel compiles successfully.

Most divergences are conservative misses where OXC's reactive scope analysis creates fewer scopes than Babel.

### Performance (p50 latency)

Measured on the 16-fixture benchmark suite at `--release`. RSS stays at ~56 MB across all fixture sizes.

| Size | LOC range   | p50 latency   |
| ---- | ----------- | ------------- |
| XS   | 8–23 LOC    | 55–165 µs     |
| S    | 35–60 LOC   | 55–270 µs     |
| M    | 80–147 LOC  | 165–530 µs    |
| L    | 152–284 LOC | 710 µs–2.0 ms |

### Running Benchmarks

```bash
# Build NAPI binding first
cd napi/react-compiler && npm install && npx napi build --release

# Run benchmark suite
cd ../.. && node benchmarks/bench.mjs

# Compare against Babel (requires babel-plugin-react-compiler)
node benchmarks/bench.mjs --diff

# Update snapshots
node benchmarks/bench.mjs --update-snapshots

# Run render equivalence (E2E HTML comparison)
node benchmarks/scripts/render-compare.mjs
```

## Known Limitations

### General

- **Proof of concept** — This is an AI-generated port and has not been validated against production workloads. Upstream conformance is at 24.6% (422/1717 fixtures). The compiler does not crash on any upstream fixture (0 panics), but output frequently diverges from the reference implementation.
- **No oxlint integration** — Lint rules exist in `crates/oxc_react_compiler_lint` and are callable via the NAPI binding, but they are not integrated into the oxlint binary. This would require upstream work in the [oxc repo](https://github.com/oxc-project/oxc) to support external plugin crates — it is not achievable in this standalone POC repo.
- **Source maps** — Source map generation covers compiled function regions with per-line identity mappings for unmodified code. Complex source map chaining with other Vite plugins has not been verified.

### Memoization & Scope Analysis

- **Slot count divergences (564 fixtures)** — The dominant failure category. 356 fixtures produce too few cache slots (under-counting) and 208 produce too many (over-counting). Root causes include reactive scope boundary computation, dependency inclusion, and scope merging logic. The param destructure fix narrowed scope boundaries, making deps more accurate but reducing total slot counts.
- **Codegen structure (268 fixtures)** — Slot count matches upstream but code within scopes differs (ordering, scope boundaries, variable placement).
- **Manual memoization preservation** — The compiler does not reliably preserve user-written `useMemo`/`useCallback` in all edge cases (35 fixtures affected by preserve-memo validation false positives).

### Validation

- **False-positive bail-outs (243 fixtures)** — We reject functions that upstream compiles successfully. Largest gaps: frozen-mutation validator (86 fixtures) uses name-based heuristics instead of actual mutable ranges from the aliasing analysis; silent bail-outs that produce 0 scopes with no error (66 fixtures); preserve-memo validation (35); locals-reassigned (25); ref-access in render (13); other globals/hooks/setState (18).
- **Upstream errors we miss (122 fixtures)** — Validation gaps where upstream correctly bails but we compile successfully.

### Other

- **@flow fixtures** — Some fixtures use Flow type syntax which the OXC parser cannot handle. These are permanently skipped as Flow is deprecated.

## License

MIT
