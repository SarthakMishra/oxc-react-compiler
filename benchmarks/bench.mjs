#!/usr/bin/env node
/**
 * OXC React Compiler Benchmark Harness
 *
 * Compiles React component fixtures using the OXC compiler and reports timing.
 * Optionally compares against Babel's react-compiler plugin if installed.
 *
 * Usage:
 *   node bench.mjs [options]
 *
 * Options:
 *   --iterations N      Number of measured iterations (default: 100)
 *   --warmup N          Number of warmup iterations (default: 20)
 *   --batch             Run all fixtures per iteration (bundler simulation)
 *   --format json|markdown  Output format (default: markdown)
 *   --filter pattern    Only run fixtures matching pattern
 *   --update-snapshots  Write current OXC output to snapshots/
 *   --check-snapshots   Verify snapshots match current output
 *   --update-babel-snapshots  Generate Babel comparison snapshots and diff reports
 *   --diff              Run OXC vs Babel structural comparison
 */

import { readFileSync, writeFileSync, existsSync, readdirSync, mkdirSync } from 'fs';
import { join, dirname, basename } from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const __dirname = dirname(fileURLToPath(import.meta.url));

// --- Parse CLI args ---
const args = process.argv.slice(2);
function getArg(name, defaultValue) {
  const idx = args.indexOf(`--${name}`);
  if (idx === -1) return defaultValue;
  if (typeof defaultValue === 'boolean') return true;
  return args[idx + 1] ?? defaultValue;
}

const iterations = parseInt(getArg('iterations', '100'), 10);
const warmup = parseInt(getArg('warmup', '20'), 10);
const batch = getArg('batch', false);
const format = getArg('format', 'markdown');
const filter = getArg('filter', '');
const updateSnapshots = getArg('update-snapshots', false);
const checkSnapshots = getArg('check-snapshots', false);

// --- Load NAPI binding ---
let oxcBinding;
try {
  oxcBinding = require(join(__dirname, '../napi/react-compiler'));
} catch (err) {
  console.error('Failed to load OXC NAPI binding. Run `npx napi build --release` first.');
  console.error(err.message);
  process.exit(1);
}

// --- Load fixtures ---
const manifestPath = join(__dirname, 'fixtures/manifest.json');
if (!existsSync(manifestPath)) {
  console.error('No manifest.json found. Run extract-fixtures.sh or create synthetic fixtures.');
  process.exit(1);
}

const manifest = JSON.parse(readFileSync(manifestPath, 'utf-8'));
let fixtures = manifest.fixtures.map((f) => ({
  ...f,
  source: readFileSync(join(__dirname, 'fixtures', f.file), 'utf-8'),
}));

if (filter) {
  fixtures = fixtures.filter((f) => f.name.includes(filter) || f.file.includes(filter));
}

if (fixtures.length === 0) {
  console.error('No fixtures found matching filter:', filter);
  process.exit(1);
}

// --- Snapshot mode ---
if (updateSnapshots) {
  const snapshotDir = join(__dirname, 'snapshots');
  mkdirSync(snapshotDir, { recursive: true });

  for (const fixture of fixtures) {
    const result = oxcBinding.transformReactFile(fixture.source, fixture.file);
    const snapshotPath = join(snapshotDir, `${fixture.name}.oxc.js`);
    writeFileSync(snapshotPath, result.code);
    console.log(`Updated: ${snapshotPath}`);
  }
  console.log(`\nUpdated ${fixtures.length} snapshots.`);
  process.exit(0);
}

if (checkSnapshots) {
  const snapshotDir = join(__dirname, 'snapshots');
  let mismatches = 0;

  for (const fixture of fixtures) {
    const result = oxcBinding.transformReactFile(fixture.source, fixture.file);
    const snapshotPath = join(snapshotDir, `${fixture.name}.oxc.js`);

    if (!existsSync(snapshotPath)) {
      console.log(`MISSING: ${snapshotPath}`);
      mismatches++;
      continue;
    }

    const expected = readFileSync(snapshotPath, 'utf-8');
    if (result.code !== expected) {
      console.log(`MISMATCH: ${fixture.name}`);
      mismatches++;
    } else {
      console.log(`OK: ${fixture.name}`);
    }
  }

  if (mismatches > 0) {
    console.error(`\n${mismatches} snapshot(s) differ. Run --update-snapshots to update.`);
    process.exit(1);
  }
  console.log('\nAll snapshots match.');
  process.exit(0);
}

// --- Benchmark mode ---
function percentile(sorted, p) {
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, idx)];
}

function benchmarkFixture(fixture) {
  const { source, file } = fixture;

  // Warmup
  for (let i = 0; i < warmup; i++) {
    oxcBinding.transformReactFileTimed(source, file);
  }

  // Measured runs
  const wallTimes = [];
  const rustTimes = [];

  for (let i = 0; i < iterations; i++) {
    const wallStart = process.hrtime.bigint();
    const result = oxcBinding.transformReactFileTimed(source, file);
    const wallEnd = process.hrtime.bigint();

    wallTimes.push(Number(wallEnd - wallStart));
    rustTimes.push(result.rustCompileNs);
  }

  wallTimes.sort((a, b) => a - b);
  rustTimes.sort((a, b) => a - b);

  return {
    name: fixture.name,
    size_tier: fixture.size_tier,
    loc: fixture.loc,
    wall: {
      min: wallTimes[0],
      p50: percentile(wallTimes, 50),
      p95: percentile(wallTimes, 95),
      p99: percentile(wallTimes, 99),
      max: wallTimes[wallTimes.length - 1],
    },
    rust: {
      min: rustTimes[0],
      p50: percentile(rustTimes, 50),
      p95: percentile(rustTimes, 95),
      p99: percentile(rustTimes, 99),
      max: rustTimes[rustTimes.length - 1],
    },
  };
}

function benchmarkBatch() {
  // Warmup
  for (let i = 0; i < warmup; i++) {
    for (const f of fixtures) {
      oxcBinding.transformReactFileTimed(f.source, f.file);
    }
  }

  const wallTimes = [];
  const rustTimes = [];

  for (let i = 0; i < iterations; i++) {
    const wallStart = process.hrtime.bigint();
    let rustTotal = 0;

    for (const f of fixtures) {
      const result = oxcBinding.transformReactFileTimed(f.source, f.file);
      rustTotal += result.rustCompileNs;
    }

    const wallEnd = process.hrtime.bigint();
    wallTimes.push(Number(wallEnd - wallStart));
    rustTimes.push(rustTotal);
  }

  wallTimes.sort((a, b) => a - b);
  rustTimes.sort((a, b) => a - b);

  return {
    name: 'BATCH (all fixtures)',
    size_tier: '-',
    loc: fixtures.reduce((sum, f) => sum + f.loc, 0),
    wall: {
      min: wallTimes[0],
      p50: percentile(wallTimes, 50),
      p95: percentile(wallTimes, 95),
      p99: percentile(wallTimes, 99),
      max: wallTimes[wallTimes.length - 1],
    },
    rust: {
      min: rustTimes[0],
      p50: percentile(rustTimes, 50),
      p95: percentile(rustTimes, 95),
      p99: percentile(rustTimes, 99),
      max: rustTimes[rustTimes.length - 1],
    },
  };
}

function fmtNs(ns) {
  if (ns >= 1_000_000) return (ns / 1_000_000).toFixed(2) + 'ms';
  if (ns >= 1_000) return (ns / 1_000).toFixed(1) + 'µs';
  return ns + 'ns';
}

// --- Run benchmarks ---
console.log(`OXC React Compiler Benchmark`);
console.log(`Fixtures: ${fixtures.length}, Warmup: ${warmup}, Iterations: ${iterations}\n`);

const memBefore = process.memoryUsage();
const results = [];

if (batch) {
  results.push(benchmarkBatch());
} else {
  for (const fixture of fixtures) {
    const result = benchmarkFixture(fixture);
    results.push(result);
  }
}

const memAfter = process.memoryUsage();
const memDelta = {
  rss: memAfter.rss - memBefore.rss,
  heapUsed: memAfter.heapUsed - memBefore.heapUsed,
};

// --- Output ---
if (format === 'json') {
  const output = {
    timestamp: new Date().toISOString(),
    config: { iterations, warmup, batch },
    memory: { rss_bytes: memAfter.rss, heap_used_bytes: memAfter.heapUsed },
    results,
  };
  const jsonPath = join(__dirname, 'results.json');
  writeFileSync(jsonPath, JSON.stringify(output, null, 2));
  console.log(`Results written to ${jsonPath}`);
} else {
  // Markdown table
  console.log('| Fixture | Size | LOC | Wall p50 | Wall p95 | Rust p50 | Rust p95 |');
  console.log('|---------|------|-----|----------|----------|----------|----------|');

  for (const r of results) {
    console.log(
      `| ${r.name} | ${r.size_tier} | ${r.loc} | ${fmtNs(r.wall.p50)} | ${fmtNs(r.wall.p95)} | ${fmtNs(r.rust.p50)} | ${fmtNs(r.rust.p95)} |`
    );
  }

  console.log(`\nMemory: RSS=${(memAfter.rss / 1024 / 1024).toFixed(1)}MB, Heap=${(memAfter.heapUsed / 1024 / 1024).toFixed(1)}MB`);
}
