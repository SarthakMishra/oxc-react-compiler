# OXC React Compiler Benchmarks

Benchmark suite for the OXC React Compiler, comparing compilation speed and output correctness.

## Quick Start

```bash
# Prerequisites: Node.js >= 18, Rust toolchain
# Build the NAPI binding first
cd napi/react-compiler && npm install && npx napi build --release && cd ../..

# Install benchmark dependencies
cd benchmarks && npm install

# Run benchmarks
node bench.mjs
```

## Usage

```bash
# Basic run (100 iterations, markdown output)
node bench.mjs

# Faster run for development
node bench.mjs --iterations 20 --warmup 5

# Batch mode (simulates bundler compiling all files)
node bench.mjs --batch

# JSON output for CI
node bench.mjs --format json

# Filter fixtures
node bench.mjs --filter "counter"

# Update snapshots after compiler changes
node bench.mjs --update-snapshots

# Check snapshots haven't changed
node bench.mjs --check-snapshots
```

## Fixture Tiers

| Tier | LOC   | Hooks | Description                |
| ---- | ----- | ----- | -------------------------- |
| XS   | < 50  | 0-1   | Baseline sanity checks     |
| S    | < 150 | 1-3   | Typical leaf components    |
| M    | < 400 | 3-6   | Mid-complexity pages/forms |
| L    | 1000+ | 6+    | Dashboards, complex views  |

## Adding New Fixtures

1. Add the component file to `fixtures/synthetic/` (or extract from an OSS repo using `scripts/extract-fixtures.sh`)
2. Add an entry to `fixtures/manifest.json` with name, file path, size tier, LOC, and hook count
3. Run `node bench.mjs --update-snapshots` to generate the initial snapshot
4. Commit the fixture, manifest update, and snapshot

### Extracting from OSS repos

```bash
# Edit scripts/extract-fixtures.sh to add/modify repo definitions
# Then run:
npm run extract-fixtures
```

Pin commits to specific SHAs for reproducibility.

## Output Metrics

- **Wall p50/p95**: Total time including NAPI marshalling overhead
- **Rust p50/p95**: Pure Rust compilation time (measured server-side via `std::time::Instant`)
- **Memory**: Process RSS and heap usage after benchmark completion

The difference between Wall and Rust time represents NAPI serialization/deserialization overhead, which is typically < 5% of total time.

## Snapshot Verification

Snapshots capture the exact compiler output for each fixture. When the compiler changes, snapshots must be explicitly updated:

```bash
# After a compiler change
node bench.mjs --check-snapshots  # Fails if output changed
node bench.mjs --update-snapshots # Update to match current output
git diff snapshots/               # Review changes
```

This creates an audit trail: every PR that changes compiler output will show snapshot diffs.

## Divergence Classifications

When the OXC compiler output differs from Babel's `babel-plugin-react-compiler`, divergences are classified into one of four categories:

| Classification          | Acceptable? | Description                                                                                                                                                  |
| ----------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Cosmetic**            | Yes         | Whitespace, variable naming, import ordering, comment differences. No semantic impact.                                                                       |
| **Conservative miss**   | Yes         | OXC memoizes less aggressively than Babel (fewer cache slots, wider scope boundaries). Correct but suboptimal — extra re-renders may occur.                  |
| **Over-memoization**    | Investigate | OXC memoizes more than Babel (extra cache slots or narrower scopes). Usually harmless but could mask bugs if dependencies are wrong.                         |
| **Semantic difference** | No (bug)    | Different runtime behavior — wrong values, missing updates, stale closures, or incorrect dependency tracking. These are correctness bugs that must be fixed. |

### How divergences are detected

1. **Structural comparison**: Parse both OXC and Babel outputs into ASTs. Compare memoization block counts, cache slot allocations, dependency arrays, and scope boundaries.
2. **Runtime comparison** (E2E tests): Render both outputs with identical props/state sequences. Compare HTML output at each step.
3. **Heuristic analysis**: `scripts/analyze-correctness.mjs` extracts memoization patterns (sentinel checks, dependency checks, cache reads/writes) and flags missing patterns.

### Known acceptable divergences

The following divergences are expected and acceptable:

- **Variable naming**: OXC uses `t0`, `t1`, etc. while Babel may use different temp naming.
- **Import style**: OXC uses `import { c as _c } from "react/compiler-runtime"` which may differ in exact import form.
- **Scope granularity**: OXC may produce wider reactive scopes, resulting in fewer but larger memoization blocks. This is a conservative miss, not a bug.
- **Dead code paths**: OXC may retain or eliminate dead code differently than Babel.

### Correctness score methodology

The correctness score is computed per-fixture as:

```
score = 1.0 - (semantic_divergences / total_memoization_sites)
```

Where:

- `total_memoization_sites` = number of `useMemoCache` slots in the Babel reference output
- `semantic_divergences` = number of sites where OXC produces different runtime behavior

**Score interpretation:**

- **1.0**: Perfect parity — OXC produces semantically identical output for all memoization sites.
- **0.9–0.99**: Minor divergences (typically conservative misses). Functionally correct.
- **< 0.9**: Significant divergences requiring investigation.

Fixtures marked `"known_divergent": true` in `manifest.json` are expected to score below 1.0 due to patterns the OXC compiler doesn't yet handle identically (e.g., complex `useReducer` patterns, drag-and-drop state).

### Aggregate score

The aggregate correctness score across all fixtures uses a weighted average:

```
aggregate = Σ(score_i × weight_i) / Σ(weight_i)
```

Weights are based on size tier: XS=1, S=2, M=3, L=4. This ensures larger, more complex components have proportionally more impact on the overall score.
