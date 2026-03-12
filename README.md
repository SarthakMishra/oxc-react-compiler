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

The compiler implements a 62-pass compilation pipeline. Architecture details (HIR data structures, pass ordering, etc.) are documented inline in the source code.

## Known Limitations

- **Proof of concept** — This is an AI-generated port and has not been validated against production workloads. Behavioral equivalence with the upstream compiler is not guaranteed.
- **Source maps** — Source map generation covers compiled function regions with per-line identity mappings for unmodified code. Complex source map chaining with other Vite plugins has not been verified.
- **No oxlint integration** — Lint rules exist in `crates/oxc_react_compiler_lint` and are callable via the NAPI binding, but they are not integrated into the oxlint binary. This would require upstream work in the [oxc repo](https://github.com/oxc-project/oxc) to support external plugin crates — it is not achievable in this standalone POC repo.

## License

MIT
