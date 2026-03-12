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

| Tier | LOC     | Hooks | Description                    |
|------|---------|-------|--------------------------------|
| XS   | < 50    | 0-1   | Baseline sanity checks         |
| S    | < 150   | 1-3   | Typical leaf components        |
| M    | < 400   | 3-6   | Mid-complexity pages/forms     |
| L    | 1000+   | 6+    | Dashboards, complex views      |

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

## CI Integration

The `.github/workflows/benchmark.yml` workflow:

1. Builds the NAPI binding
2. Runs `cargo test`
3. Runs benchmarks with JSON output
4. Checks snapshots haven't changed
5. Runs E2E tests
6. Posts a benchmark results comment on PRs
7. Updates `baseline.json` on main merges
