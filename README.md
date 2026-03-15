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
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { reactCompiler } from 'oxc-react-compiler/vite';

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
  compilationMode: 'infer',

  // Output mode (default: 'client')
  outputMode: 'client',

  // Enable/disable source maps (default: true in dev, false in build)
  sourceMap: true,
})
```

## Configuration Reference

### `compilationMode`

| Value | Description |
|---|---|
| `'infer'` | **(Default)** Compile functions that look like React components or hooks (PascalCase names, `use`-prefixed names) |
| `'all'` | Compile all top-level functions |
| `'syntax'` | Only compile functions containing the `"use memo"` directive |
| `'annotation'` | Same as `syntax` — only compile annotated functions |

### `outputMode`

| Value | Description |
|---|---|
| `'client'` | **(Default)** Normal client-side compilation with `useMemoCache`-based memoization |
| `'ssr'` | Server-side rendering mode (skips client-specific optimizations) |
| `'lint'` | Lint-only mode — runs analysis passes and collects diagnostics without transforming code |

### `target`

| Value | Description |
|---|---|
| `'react19'` | **(Default)** Target React 19 runtime APIs |
| `'react18'` | Target React 18 |
| `'react17'` | Target React 17 |

### `sourceMap`

| Value | Description |
|---|---|
| `true` | Enable source map generation |
| `false` | Disable source maps |
| *(unset)* | **(Default)** Auto-detect: enabled in `vite serve`, disabled in `vite build` |

### `include` / `exclude`

Glob patterns to include or exclude specific files from compilation. By default, all `.tsx`, `.jsx`, `.ts`, `.js` files (excluding `node_modules`) are considered.

### `gating`

Feature flag configuration for wrapping compiled output:

```ts
reactCompiler({
  gating: {
    importSource: 'my-flags',
    functionName: 'isCompilerEnabled',
  },
})
```

### `cacheDir`

Directory for persisting the transform cache across builds. When set, the cache is written to `<cacheDir>/oxc-react-compiler-cache.json` and reloaded on the next build, skipping re-compilation of unchanged files. Leave unset to use only the in-memory cache.

```ts
reactCompiler({
  cacheDir: 'node_modules/.cache',
})
```

## Lint Rules

### Tier 1: AST-Level Rules

These rules use static AST analysis (no compilation required):

| Rule | Description |
|---|---|
| `rules-of-hooks` | Hooks must be called at the top level, not in conditions/loops |
| `no-jsx-in-try` | JSX should not be used inside try blocks — use error boundaries instead |
| `no-ref-access-in-render` | `ref.current` should not be read during render |
| `no-set-state-in-render` | `setState` should not be called unconditionally during render |
| `no-set-state-in-effects` | Synchronous `setState` in effect bodies causes extra re-renders |
| `use-memo-validation` | `useMemo`/`useCallback` must have exactly 2 arguments |
| `no-capitalized-calls` | PascalCase names should be JSX components, not called as functions |
| `purity` | Detect impure function calls during render |
| `incompatible-library` | Warn on libraries with known React incompatibilities |
| `static-components` | Detect inline component definitions that cause remounting |
| `no-deriving-state-in-effects` | Derived state should be computed during render, not in effects |

### Tier 2: Compiler-Dependent Rules

These rules require the full compiler pipeline (HIR, effect system, reactive scopes). Use the `_with_source` API variants:

| Rule | Description |
|---|---|
| `check_hooks_tier2` | Full Rules of Hooks with CFG analysis |
| `check_immutability` | Detect mutation of frozen/immutable values |
| `check_preserve_manual_memoization` | Verify compiler preserves manual `useMemo`/`useCallback` |
| `check_memo_dependencies` | Exhaustive dependency checking for `useMemo`/`useCallback` |
| `check_exhaustive_effect_deps` | Exhaustive dependency checking for `useEffect`/`useLayoutEffect` |

### Using Lint Rules via NAPI

```ts
import { lintReactFile } from 'oxc-react-compiler';

const result = lintReactFile(sourceCode, 'component.tsx');
for (const diag of result.diagnostics) {
  console.log(`${diag.message} at ${diag.start}:${diag.end}`);
}
```

## Crates

| Crate | Description |
|---|---|
| `oxc_react_compiler` | Core compiler — HIR, 62-pass pipeline, codegen |
| `oxc_react_compiler_lint` | Lint rules for oxlint (replaces `eslint-plugin-react-compiler`) |
| `oxc-react-compiler-napi` | NAPI-RS bindings + Vite plugin |

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

| Phase | Passes | Description |
|---|---|---|
| 0. Early Validation | 1 | Reject unsupported patterns (getters, setters, for-await, new.target) |
| 1. Early Cleanup | 2–7 | Prune throws, validate context variables, validate useMemo, drop manual memoization, inline IIFEs, merge blocks |
| 2. SSA | 8–9.5 | Enter SSA form, eliminate redundant phi nodes, prune temporary lvalues |
| 3. Optimization & Types | 10–12 | Constant propagation, type inference, instruction kind rewriting |
| 4. Hook Validation | 13–14.6 | Validate hooks usage, no capitalized calls, no global reassignment, no eval |
| 5. Mutation/Aliasing | 14–20 | Props optimization, function analysis, mutation/aliasing effects, freeze validation, SSR optimization, DCE, mutable ranges |
| 6. Validation Battery | 21–28.7 | Locals reassignment, ref access, setState, impurity, blocklisted imports, break targets |
| 7. Reactivity | 29–32 | Reactive place inference, exhaustive deps, unconditional blocks, property load hoisting, optional chains, static components |
| 8. Scope Construction | 33–46.5 | Reactive scope variables, JSX/function outlining, scope alignment, merging, dependency propagation, minimal deps |
| 9. RF Optimization | 47–61 | Build reactive function tree, prune/merge/stabilize scopes, rename variables, validate memoization |

## Benchmarks & Conformance

### Upstream Conformance

The compiler is tested against Meta's upstream React Compiler conformance suite — the same 1717 test fixtures used by `babel-plugin-react-compiler`. Output is compared structurally after normalizing semantics-irrelevant differences (import paths, variable naming, whitespace, cache variable names).

| Metric | Value |
|---|---|
| Total upstream fixtures | 1717 |
| Passing | 428 (24.9%) |
| Failing (output divergence) | 1289 |
| Panics / crashes | 0 |

#### Divergence Breakdown

| Root Cause | Fixtures | Description |
|---|---|---|
| Slot over-count (too many scopes/deps) | ~443 | We create too many reactive scopes or include extra dependencies |
| False-positive frozen-mutation bail-out | 162 | Validator rejects code upstream compiles successfully |
| Slot under-count (missing scopes) | ~162 | We miss required scopes for some dependency patterns |
| Same slots, different codegen structure | ~150 | Slot count matches but generated code within scopes differs |
| We memoize, upstream returns unchanged | ~102 | We add `_c()` caching but upstream returns source unchanged |
| Both no-memo, output differs | ~43 | Neither side memoizes but output structure differs |
| @flow fixtures | 38 | Use Flow type syntax; OXC parser can't handle them |
| Upstream errors we should match | ~35 | Upstream bails with error; we compile successfully |
| Other false-positive bail-outs | ~42 | useMemo/useCallback args (17), ref-access (11), globals (11), setState (3) |

#### Key Divergence Patterns

Most of the 1289 failures fall into a few root causes:

- **Slot count divergences (~605)** — Reactive scope computation produces wrong cache slot counts. ~443 over-count (extra scopes/deps), ~162 under-count (missing scopes). This is the highest-volume issue.
- **False-positive bail-outs (~204)** — We reject functions that upstream compiles successfully. Largest: frozen-mutation validator (162) uses name-based heuristics instead of actual mutable ranges.
- **Codegen structure (~150)** — Slot count matches but code ordering or scope boundaries differ.
- **Unnecessary memoization (~145)** — We add memoization that upstream doesn't. Root causes: missing DCE, const-prop edge cases, or incorrect scope creation for non-reactive functions.
- **Upstream errors we miss (~35)** — Validation gaps where upstream correctly bails but we compile. Partially addressed (4 done via `validate_no_unsupported_nodes`).
- **@flow fixtures (38)** — OXC parser limitation; Flow is deprecated and these are permanently skipped.

Conformance runs as a non-blocking CI check — failures are tracked in `tests/conformance/known-failures.txt` and ratcheted as improvements land.

To run conformance tests locally:

```bash
./tests/conformance/download-upstream.sh
cargo test --release upstream_conformance -- --nocapture
```

### Memoization Benchmarks (OXC vs Babel)

The benchmark suite compiles 16 real-world React components through both OXC and the upstream Babel compiler, then structurally compares memoization patterns (cache slot count, scope blocks, dependency checks).

| Fixture | Size | OXC Cache | Babel Cache | Delta | Divergence |
|---|---|---|---|---|---|
| simple-counter | XS | 2 | 2 | 0 | match |
| status-badge | XS | 3 | 7 | -4 | conservative miss |
| avatar-group | XS | 6 | 10 | -4 | conservative miss |
| theme-toggle | S | 6 | 4 | +2 | over-memoization |
| search-input | S | 11 | 17 | -6 | conservative miss |
| toolbar | S | 13 | 19 | -6 | conservative miss |
| todo-list | S | 10 | 24 | -14 | conservative miss |
| time-slot-picker | M | 17 | 20 | -3 | conservative miss |
| color-picker | M | 23 | 30 | -7 | conservative miss |
| data-table | M | 22 | 56 | -34 | conservative miss |
| form-validation | S | 35 | 48 | -13 | conservative miss |
| availability-schedule | L | 21 | 31 | -10 | conservative miss |
| booking-list | L | 23 | 62 | -39 | conservative miss |
| command-menu | M | 0 | 37 | -37 | bail-out |
| canvas-sidebar | L | 50 | 70 | -20 | conservative miss |
| multi-step-form | L | 53 | 92 | -39 | conservative miss |

**Divergence types:**

- **match** — OXC produces the same number of cache slots as Babel.
- **conservative miss** — OXC compiles successfully but memoizes fewer values than Babel. The output is functionally correct but leaves some optimization opportunities on the table.
- **over-memoization** — OXC memoizes more than Babel (minor; 1 fixture).
- **bail-out** — OXC bails on compilation (e.g., due to a false-positive validation error) while Babel compiles successfully.

Most divergences are conservative misses where OXC's reactive scope analysis creates fewer scopes than Babel. Compared to earlier versions, cache slot counts have improved significantly across all fixtures (e.g., `simple-counter` now matches exactly, `canvas-sidebar` went from 5 to 50 slots vs Babel's 70).

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

- **Proof of concept** — This is an AI-generated port and has not been validated against production workloads. Upstream conformance is at 24.9% (428/1717 fixtures). The compiler does not crash on any upstream fixture (0 panics), but output frequently diverges from the reference implementation.
- **No oxlint integration** — Lint rules exist in `crates/oxc_react_compiler_lint` and are callable via the NAPI binding, but they are not integrated into the oxlint binary. This would require upstream work in the [oxc repo](https://github.com/oxc-project/oxc) to support external plugin crates — it is not achievable in this standalone POC repo.
- **Source maps** — Source map generation covers compiled function regions with per-line identity mappings for unmodified code. Complex source map chaining with other Vite plugins has not been verified.

### Memoization & Scope Analysis

- **Slot count divergences (~605)** — The dominant failure category. ~443 fixtures produce too many cache slots (over-counting), ~162 produce too few (under-counting). Root causes include reactive scope boundary computation, dependency inclusion, and scope merging logic.
- **Codegen structure (~150)** — Slot count matches upstream but code within scopes differs (ordering, scope boundaries, variable placement).
- **Unnecessary memoization (~145)** — We add `_c()` caching for functions that upstream returns unchanged. Missing DCE, const-prop edge cases, or incorrect scope creation for non-reactive functions.
- **Manual memoization preservation** — The compiler does not reliably preserve user-written `useMemo`/`useCallback` in all edge cases.

### Validation

- **False-positive bail-outs (~204)** — We reject functions that upstream compiles successfully. Largest gap: frozen-mutation validator (162 fixtures) uses name-based heuristics instead of actual mutable ranges from the aliasing analysis. Other false positives in useMemo/useCallback args (17), ref-access (11), globals (11), setState (3).
- **Upstream errors we miss (~35)** — Validation gaps where upstream correctly bails but we compile. Partially addressed via `validate_no_unsupported_nodes`.

### Other

- **@flow fixtures** — 38 fixtures use Flow type syntax which the OXC parser cannot handle. These are permanently skipped as Flow is deprecated.

## License

MIT
