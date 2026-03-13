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
| `oxc_react_compiler` | Core compiler — HIR, 61-pass pipeline, codegen |
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

The compiler implements a 61-pass compilation pipeline organized into 9 phases:

| Phase | Passes | Description |
|---|---|---|
| 1. Early Cleanup | 2–6 | Prune throws, validate context variables, drop manual memoization, inline IIFEs |
| 2. SSA | 7–9.5 | Enter SSA form, eliminate redundant phi nodes, prune temporary lvalues |
| 3. Optimization & Types | 10–12 | Constant propagation, type inference, instruction kind rewriting |
| 4. Hook Validation | 13–14 | Validate hooks usage, no capitalized calls |
| 5. Mutation/Aliasing | 14–19 | Function analysis, effect inference, SSR optimization, dead code elimination |
| 6. Validation Battery | 20–28.7 | Mutable ranges, ref access, setState, impurity, blocklisted imports |
| 7. Reactivity | 29–32 | Reactive place inference, exhaustive deps, unconditional blocks, property load hoisting, optional chains |
| 8. Scope Construction | 33–46.5 | Reactive scope variables, JSX/function outlining, scope alignment, merging, dependency propagation |
| 9. RF Optimization | 47–61 | Build reactive function tree, prune/merge/stabilize scopes, rename variables, validate memoization |

## Benchmarks & Conformance

### Upstream Conformance

The compiler is tested against Meta's upstream React Compiler conformance suite — the same 1717 test fixtures used by `babel-plugin-react-compiler`. Output is compared structurally after normalizing semantics-irrelevant differences (import paths, variable naming, whitespace, cache variable names).

| Metric | Value |
|---|---|
| Total upstream fixtures | 1717 |
| Passing | 304 (17.7%) |
| Failing (output divergence) | 1413 |
| Panics / crashes | 0 |

#### Divergence Breakdown

| Category | Count | Description |
|---|---|---|
| Compiled with memo | 904 | We produce memoized output but structure/deps/slots differ |
| No expected file | 261 | Upstream fixture has no `.expected` — can't compare |
| Compiled no memo | 152 | Upstream applies DCE/const-prop/outlining; we pass through unchanged |
| Upstream errors | 96 | Upstream bails with error; we compile successfully |
| @flow fixtures | 38 | Use Flow type syntax; OXC parser can't handle them |

#### Key Divergence Patterns

Most of the 1413 failures fall into a few root causes:

- **Over-scoped memoization (~400)** — Reactive scope computation is over-conservative; includes globals/stable values as deps, producing too many cache slots.
- **Sentinel pattern (~280)** — Non-reactive allocations (JSX, objects) need scopes with `Symbol.for("react.memo_cache_sentinel")` check instead of `!==` comparison.
- **Under-scoped (~90)** — Missing scopes for some expressions, producing too few cache slots.
- **Upstream errors we miss (96)** — Frozen value mutation (26), preserve-memo validation (13), exhaustive deps (8), scope reassignment (6), ref aliasing (6), and other validation gaps.
- **Compiled-no-memo transforms (152)** — Upstream applies DCE, constant propagation, and arrow extraction even when no memoization is needed; we return the source unchanged.

Conformance runs as a non-blocking CI check — failures are tracked in `tests/conformance/known-failures.txt` and ratcheted as improvements land.

To run conformance tests locally:

```bash
./tests/conformance/download-upstream.sh
cargo test --release upstream_conformance -- --nocapture
```

### Memoization Benchmarks (OXC vs Babel)

The benchmark suite compiles 16 real-world React components through both OXC and the upstream Babel compiler, then structurally compares memoization patterns (cache slot count, scope blocks, dependency checks).

| Fixture | Tier | OXC Cache | Babel Cache | Delta | Divergence |
|---|---|---|---|---|---|
| simple-counter | XS | 3 | 2 | +1 | over-memoization |
| status-badge | XS | 3 | 7 | -4 | conservative miss |
| theme-toggle | S | 5 | 7 | -2 | conservative miss |
| toolbar | S | 3 | 19 | -16 | conservative miss |
| search-input | S | 4 | 17 | -13 | conservative miss |
| avatar-group | S | 5 | 12 | -7 | conservative miss |
| todo-list | M | 8 | 25 | -17 | conservative miss |
| color-picker | M | 8 | 30 | -22 | conservative miss |
| form-validation | M | 6 | 47 | -41 | conservative miss |
| time-slot-picker | M | 3 | 17 | -14 | conservative miss |
| data-table | M | 11 | 56 | -45 | conservative miss |
| booking-list | L | 8 | 61 | -53 | conservative miss |
| command-menu | L | 3 | 29 | -26 | conservative miss |
| canvas-sidebar | L | 5 | 55 | -50 | conservative miss |
| multi-step-form | L | 9 | 89 | -80 | conservative miss |
| availability-schedule | M | 0 | 17 | -17 | semantic difference |

**Divergence types:**

- **conservative miss** — OXC compiles successfully but memoizes fewer values than Babel. The output is functionally correct but leaves some optimization opportunities on the table.
- **over-memoization** — OXC memoizes more than Babel (minor; only 1 fixture).
- **semantic difference** — OXC produces structurally different output (e.g., skips memoization entirely).

Most divergences are conservative misses where OXC's reactive scope analysis merges fewer scopes than Babel. This is the primary area for improving feature parity.

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

- **Proof of concept** — This is an AI-generated port and has not been validated against production workloads. Upstream conformance is at 17.7% (304/1717 fixtures). The compiler does not crash on any upstream fixture (0 panics), but output frequently diverges from the reference implementation.
- **No oxlint integration** — Lint rules exist in `crates/oxc_react_compiler_lint` and are callable via the NAPI binding, but they are not integrated into the oxlint binary. This would require upstream work in the [oxc repo](https://github.com/oxc-project/oxc) to support external plugin crates — it is not achievable in this standalone POC repo.
- **Source maps** — Source map generation covers compiled function regions with per-line identity mappings for unmodified code. Complex source map chaining with other Vite plugins has not been verified.

### Memoization & Scope Analysis

- **Over-scoped memoization** — OXC's reactive scope analysis is over-conservative, including globals and stable values as dependencies, producing more cache slots than needed in some cases and fewer in others. This is the dominant divergence pattern (904 fixtures).
- **Sentinel pattern** — Non-reactive allocations (JSX, objects, arrays) need scopes using the `Symbol.for("react.memo_cache_sentinel")` check pattern. This accounts for ~280 divergences.
- **Manual memoization preservation** — The compiler does not reliably preserve user-written `useMemo`/`useCallback` in all edge cases (13 upstream error fixtures + 62 preserve-memo-validation fixtures).

### Feature Gaps

- **Upstream error detection** — 96 fixtures where the upstream compiler correctly bails with an error but OXC compiles successfully. Key gaps: frozen value mutation (26), preserve-memo validation (13), exhaustive deps (8), scope reassignment (6), ref aliasing (6).
- **Compiled-no-memo transforms** — 152 fixtures where upstream applies DCE, constant propagation, or arrow extraction even without memoization. OXC returns the source unchanged.
- **@flow fixtures** — 38 fixtures use Flow type syntax which the OXC parser cannot handle. These are expected to remain unsupported as Flow is deprecated.

## License

MIT
