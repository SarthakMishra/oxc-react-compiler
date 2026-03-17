# E2E Vite Build Benchmark

End-to-end benchmark that compares **OXC React Compiler** vs **Babel React Compiler** on real open-source projects.

## How it works

The script (`e2e-bench.mjs`) performs these steps for each project:

1. **Clone** — shallow-clones the repo into `.workspace/<project>/` and **strips the `.git` directory** (no history needed; saves disk)
2. **Install** — runs the project's native package manager (`pnpm`, `yarn`, or `npm`)
3. **Babel baseline build** — builds with the project's existing `babel-plugin-react-compiler` setup (or patches it in for projects that don't have it)
4. **OXC build** — patches the Vite config to replace Babel's compiler with the OXC Vite plugin, then rebuilds
5. **Compare** — reports build time, bundle size, and OXC transform coverage (compiled / skipped / validation errors)

All Vite config changes are **applied programmatically** and restored after each run. No manual edits are needed — the script is the single source of truth for reproducing results.

## Running

```bash
# Prerequisites: build the NAPI binding first
cd napi/react-compiler && npm install && npx napi build --release && cd ../..

# Full suite (~8-10 min, clones all repos)
cd benchmarks && node e2e/e2e-bench.mjs

# Single project
node e2e/e2e-bench.mjs --project docmost

# Reuse already-cloned repos (skip git clone)
node e2e/e2e-bench.mjs --skip-clone

# Quick run (1 iteration per build instead of 3)
node e2e/e2e-bench.mjs --iterations 1

# JSON report
node e2e/e2e-bench.mjs --format json

# Verbose (show build output + patched configs)
node e2e/e2e-bench.mjs --verbose
```

## Workspace layout

```
e2e/
  e2e-bench.mjs       # benchmark script
  projects.json        # project metadata (informational, not used by script)
  README.md            # this file
  .workspace/          # gitignored — cloned project sources (auto-created)
    ephe/
    rai-pal/
    arcomage-hd/
    docmost/
  reports/             # gitignored — JSON reports when --format json
```

The `.workspace/` directory is **gitignored** and fully disposable. Deleting it and re-running the script reproduces the same results (repos are cloned fresh).

## Reproducibility

All config modifications are deterministic and code-driven:

- **Vite config patching**: The script backs up the original config, programmatically removes `babel-plugin-react-compiler` references, injects the OXC plugin, and restores the original after benchmarking.
- **No git tracking in workspace**: `.git` is stripped after cloning. The cloned source is treated as a read-only snapshot — the script's patches are the only modifications, and they're fully described in `e2e-bench.mjs`.
- **Shallow clones**: `--depth 1` ensures only the latest commit is fetched (no history bloat).
- **Package manager lockfiles**: Dependencies are installed from the project's own lockfile for deterministic installs.

To reproduce someone else's results: run the same script on the same Node version. The project registry in `e2e-bench.mjs` (the `PROJECTS` array) pins repos by URL; pass `--skip-clone` to reuse an existing workspace.

## Adding a new project

Add an entry to the `PROJECTS` array in `e2e-bench.mjs`:

```js
{
  name: 'my-project',
  repo: 'https://github.com/org/my-project.git',
  scale: 'medium',            // small | medium | large | xlarge
  stars: 1234,
  description: 'Short description',
  viteConfigDir: '.',          // directory containing vite.config.ts
  viteConfigFile: 'vite.config.ts',
  buildCmd: 'npx vite build',
  distDir: 'dist',
  hasReactCompiler: true,      // false if project doesn't already use babel-plugin-react-compiler
  // Optional fields for projects without react-compiler:
  needsCompilerSetup: false,   // true to auto-install + patch babel-plugin-react-compiler
  usesSWC: false,              // true if project uses @vitejs/plugin-react-swc
  monorepoInstallDir: '.',     // where to run `pnpm install` (monorepo root)
  appDir: 'apps/client',       // app directory within monorepo
  preBuildCmd: '',             // shell command to build workspace deps before vite build
  postCloneCmd: '',            // shell command run after cloning (e.g., relax Node constraints)
}
```

Then run `node e2e/e2e-bench.mjs --project my-project --verbose` to test it.

## OXC validation layer

The OXC Vite plugin includes an **esbuild validation step**: after OXC compiles a file, the output is syntax-checked with `esbuild.transformSync()`. If the compiled output has syntax errors (duplicate declarations, invalid AST, etc.), the file silently falls through to the original uncompiled source. This ensures OXC codegen bugs never break the build — they just reduce coverage.

The "Validation Errors" column in the results shows how many files hit this path. These represent real OXC compiler bugs that need fixing.
