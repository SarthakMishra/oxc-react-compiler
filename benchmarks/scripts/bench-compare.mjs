#!/usr/bin/env node
/**
 * Comparative Benchmark: OXC vs Babel React Compiler
 *
 * Measures compile latency for both compilers across all fixtures,
 * then reports per-fixture and aggregate throughput comparisons.
 *
 * Sections:
 *   1. Per-fixture compile latency (p50, p95) for both compilers
 *   2. Batch "project build" — compile all fixtures end-to-end
 *   3. Vite transform pipeline simulation (cold build + warm HMR rebuild)
 *
 * Usage:
 *   node scripts/bench-compare.mjs [options]
 *
 * Options:
 *   --iterations N     Measured iterations per fixture (default: 50)
 *   --warmup N         Warmup iterations (default: 10)
 *   --format json      Output JSON instead of markdown
 *   --filter pattern   Only run fixtures matching pattern
 *   --section 1|2|3|all  Run specific section (default: all)
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';
import { createHash } from 'crypto';

const require = createRequire(import.meta.url);
const __dirname = dirname(fileURLToPath(import.meta.url));
const benchDir = join(__dirname, '..');

// --- Parse CLI args ---
const args = process.argv.slice(2);
function getArg(name, defaultValue) {
  const idx = args.indexOf(`--${name}`);
  if (idx === -1) return defaultValue;
  if (typeof defaultValue === 'boolean') return true;
  return args[idx + 1] ?? defaultValue;
}

const iterations = parseInt(getArg('iterations', '50'), 10);
const warmup = parseInt(getArg('warmup', '10'), 10);
const format = getArg('format', 'markdown');
const filter = getArg('filter', '');
const section = getArg('section', 'all');

// --- Load compilers ---
let oxcBinding;
try {
  oxcBinding = require(join(benchDir, '../napi/react-compiler'));
} catch (err) {
  console.error('Failed to load OXC NAPI binding. Run `npx napi build --release` first.');
  process.exit(1);
}

let babel, reactCompilerPlugin;
try {
  babel = require('@babel/core');
  reactCompilerPlugin = require('babel-plugin-react-compiler');
} catch (err) {
  console.error('Failed to load Babel or babel-plugin-react-compiler.');
  console.error('Run: cd benchmarks && npm install');
  process.exit(1);
}

// --- Load fixtures ---
const manifestPath = join(benchDir, 'fixtures/manifest.json');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf-8'));
let fixtures = manifest.fixtures.map((f) => ({
  ...f,
  source: readFileSync(join(benchDir, 'fixtures', f.file), 'utf-8'),
}));

if (filter) {
  fixtures = fixtures.filter((f) => f.name.includes(filter) || f.file.includes(filter));
}

if (fixtures.length === 0) {
  console.error('No fixtures found matching filter:', filter);
  process.exit(1);
}

// --- Compiler wrappers ---
function compileOxc(source, filename) {
  return oxcBinding.transformReactFile(source, filename);
}

function compileBabel(source, filename) {
  return babel.transformSync(source, {
    filename,
    presets: [
      ['@babel/preset-react', { runtime: 'automatic' }],
      ['@babel/preset-typescript', { isTSX: true, allExtensions: true }],
    ],
    plugins: [reactCompilerPlugin],
    sourceType: 'module',
  });
}

// --- Stats helpers ---
function percentile(sorted, p) {
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, idx)];
}

function fmtNs(ns) {
  if (ns >= 1_000_000) return (ns / 1_000_000).toFixed(2) + ' ms';
  if (ns >= 1_000) return (ns / 1_000).toFixed(1) + ' µs';
  return ns + ' ns';
}

function fmtMs(ms) {
  if (ms >= 1000) return (ms / 1000).toFixed(2) + ' s';
  return ms.toFixed(2) + ' ms';
}

// ============================================================
// SECTION 1: Per-fixture compile latency comparison
// ============================================================
function runPerFixtureBenchmark() {
  console.log('## Section 1: Per-Fixture Compile Latency (OXC vs Babel)\n');
  console.log(`Fixtures: ${fixtures.length}, Warmup: ${warmup}, Iterations: ${iterations}\n`);

  const results = [];

  for (const fixture of fixtures) {
    const { source, file, name, size_tier, loc } = fixture;

    // --- OXC benchmark ---
    // Warmup
    for (let i = 0; i < warmup; i++) {
      try { compileOxc(source, file); } catch {}
    }
    // Measured
    const oxcTimes = [];
    for (let i = 0; i < iterations; i++) {
      const start = process.hrtime.bigint();
      try { compileOxc(source, file); } catch {}
      oxcTimes.push(Number(process.hrtime.bigint() - start));
    }
    oxcTimes.sort((a, b) => a - b);

    // --- Babel benchmark ---
    // Warmup
    for (let i = 0; i < warmup; i++) {
      try { compileBabel(source, file); } catch {}
    }
    // Measured
    const babelTimes = [];
    for (let i = 0; i < iterations; i++) {
      const start = process.hrtime.bigint();
      try { compileBabel(source, file); } catch {}
      babelTimes.push(Number(process.hrtime.bigint() - start));
    }
    babelTimes.sort((a, b) => a - b);

    const oxcP50 = percentile(oxcTimes, 50);
    const oxcP95 = percentile(oxcTimes, 95);
    const babelP50 = percentile(babelTimes, 50);
    const babelP95 = percentile(babelTimes, 95);
    const speedup = (babelP50 / oxcP50).toFixed(1);

    results.push({
      name, size_tier, loc,
      oxc_p50: oxcP50, oxc_p95: oxcP95,
      babel_p50: babelP50, babel_p95: babelP95,
      speedup: parseFloat(speedup),
    });
  }

  return results;
}

function printPerFixtureResults(results) {
  console.log('| Fixture | Size | LOC | OXC p50 | Babel p50 | Speedup | OXC p95 | Babel p95 |');
  console.log('|---------|------|-----|---------|-----------|---------|---------|-----------|');

  for (const r of results) {
    console.log(
      `| ${r.name} | ${r.size_tier} | ${r.loc} | ${fmtNs(r.oxc_p50)} | ${fmtNs(r.babel_p50)} | **${r.speedup}x** | ${fmtNs(r.oxc_p95)} | ${fmtNs(r.babel_p95)} |`
    );
  }

  // Aggregate stats
  const avgSpeedup = results.reduce((s, r) => s + r.speedup, 0) / results.length;
  const medianSpeedup = percentile(
    results.map((r) => r.speedup).sort((a, b) => a - b),
    50
  );
  const minSpeedup = Math.min(...results.map((r) => r.speedup));
  const maxSpeedup = Math.max(...results.map((r) => r.speedup));

  console.log(`\n**Aggregate**: median ${medianSpeedup.toFixed(1)}x, mean ${avgSpeedup.toFixed(1)}x, range ${minSpeedup.toFixed(1)}x–${maxSpeedup.toFixed(1)}x\n`);
}

// ============================================================
// SECTION 2: Batch "project build" comparison
// ============================================================
function runBatchBenchmark() {
  console.log('## Section 2: Batch Project Build (All Fixtures End-to-End)\n');
  console.log(`Compiling ${fixtures.length} files × ${iterations} iterations\n`);

  const totalLOC = fixtures.reduce((s, f) => s + f.loc, 0);

  // --- OXC batch ---
  for (let i = 0; i < warmup; i++) {
    for (const f of fixtures) {
      try { compileOxc(f.source, f.file); } catch {}
    }
  }

  const oxcBatchTimes = [];
  for (let i = 0; i < iterations; i++) {
    const start = process.hrtime.bigint();
    for (const f of fixtures) {
      try { compileOxc(f.source, f.file); } catch {}
    }
    oxcBatchTimes.push(Number(process.hrtime.bigint() - start));
  }
  oxcBatchTimes.sort((a, b) => a - b);

  // --- Babel batch ---
  for (let i = 0; i < warmup; i++) {
    for (const f of fixtures) {
      try { compileBabel(f.source, f.file); } catch {}
    }
  }

  const babelBatchTimes = [];
  for (let i = 0; i < iterations; i++) {
    const start = process.hrtime.bigint();
    for (const f of fixtures) {
      try { compileBabel(f.source, f.file); } catch {}
    }
    babelBatchTimes.push(Number(process.hrtime.bigint() - start));
  }
  babelBatchTimes.sort((a, b) => a - b);

  const oxcP50 = percentile(oxcBatchTimes, 50);
  const babelP50 = percentile(babelBatchTimes, 50);
  const oxcP95 = percentile(oxcBatchTimes, 95);
  const babelP95 = percentile(babelBatchTimes, 95);

  return {
    total_loc: totalLOC,
    file_count: fixtures.length,
    oxc_p50: oxcP50,
    oxc_p95: oxcP95,
    babel_p50: babelP50,
    babel_p95: babelP95,
    speedup: parseFloat((babelP50 / oxcP50).toFixed(1)),
    oxc_throughput_loc_per_sec: Math.round((totalLOC / (oxcP50 / 1e9))),
    babel_throughput_loc_per_sec: Math.round((totalLOC / (babelP50 / 1e9))),
  };
}

function printBatchResults(result) {
  console.log(`| Metric | OXC | Babel |`);
  console.log(`|--------|-----|-------|`);
  console.log(`| Files compiled | ${result.file_count} | ${result.file_count} |`);
  console.log(`| Total LOC | ${result.total_loc} | ${result.total_loc} |`);
  console.log(`| Batch p50 | ${fmtNs(result.oxc_p50)} | ${fmtNs(result.babel_p50)} |`);
  console.log(`| Batch p95 | ${fmtNs(result.oxc_p95)} | ${fmtNs(result.babel_p95)} |`);
  console.log(`| Throughput (LOC/s) | ${result.oxc_throughput_loc_per_sec.toLocaleString()} | ${result.babel_throughput_loc_per_sec.toLocaleString()} |`);
  console.log(`| **Speedup** | **${result.speedup}x** | baseline |`);
  console.log();
}

// ============================================================
// SECTION 3: Vite transform pipeline simulation
// ============================================================
function runViteSimulation() {
  console.log('## Section 3: Vite Transform Pipeline Simulation\n');

  const VITE_ITERATIONS = 10;

  // Simulate Vite's mightContainReactCode heuristic
  function mightContainReactCode(source) {
    return /\b(use[A-Z]|useState|useEffect|useMemo|useCallback|useRef|React\.|jsx|JSX)\b/.test(source);
  }

  function contentHash(source) {
    return createHash('md5').update(source).digest('hex');
  }

  // --- Cold build: compile all files (no cache) ---
  function coldBuild(compiler, fixtures) {
    const start = process.hrtime.bigint();
    for (const f of fixtures) {
      if (!mightContainReactCode(f.source)) continue;
      try { compiler(f.source, f.file); } catch {}
    }
    return Number(process.hrtime.bigint() - start);
  }

  // --- Warm rebuild: one file changed, rest cached ---
  function warmRebuild(compiler, fixtures, changedIndex) {
    // Pre-populate cache
    const cache = new Map();
    for (const f of fixtures) {
      const hash = contentHash(f.source);
      try {
        const result = compiler(f.source, f.file);
        cache.set(f.file, { hash, code: result?.code || '' });
      } catch {
        cache.set(f.file, { hash, code: '' });
      }
    }

    const start = process.hrtime.bigint();
    for (let i = 0; i < fixtures.length; i++) {
      const f = fixtures[i];
      if (!mightContainReactCode(f.source)) continue;

      const hash = contentHash(f.source);
      const cached = cache.get(f.file);

      if (i === changedIndex || !cached || cached.hash !== hash) {
        // Cache miss — recompile
        try { compiler(f.source, f.file); } catch {}
      }
      // Cache hit — no-op (just the hash comparison)
    }
    return Number(process.hrtime.bigint() - start);
  }

  // Warmup both compilers
  for (let i = 0; i < 3; i++) {
    coldBuild(compileOxc, fixtures);
    coldBuild(compileBabel, fixtures);
  }

  // Change the largest fixture (last)
  const changedIdx = fixtures.length - 1;

  // --- OXC cold ---
  const oxcColdTimes = [];
  for (let i = 0; i < VITE_ITERATIONS; i++) {
    oxcColdTimes.push(coldBuild(compileOxc, fixtures));
  }
  oxcColdTimes.sort((a, b) => a - b);

  // --- Babel cold ---
  const babelColdTimes = [];
  for (let i = 0; i < VITE_ITERATIONS; i++) {
    babelColdTimes.push(coldBuild(compileBabel, fixtures));
  }
  babelColdTimes.sort((a, b) => a - b);

  // --- OXC warm ---
  const oxcWarmTimes = [];
  for (let i = 0; i < VITE_ITERATIONS; i++) {
    oxcWarmTimes.push(warmRebuild(compileOxc, fixtures, changedIdx));
  }
  oxcWarmTimes.sort((a, b) => a - b);

  // --- Babel warm ---
  const babelWarmTimes = [];
  for (let i = 0; i < VITE_ITERATIONS; i++) {
    babelWarmTimes.push(warmRebuild(compileBabel, fixtures, changedIdx));
  }
  babelWarmTimes.sort((a, b) => a - b);

  return {
    cold: {
      oxc_p50: percentile(oxcColdTimes, 50),
      babel_p50: percentile(babelColdTimes, 50),
      speedup: parseFloat((percentile(babelColdTimes, 50) / percentile(oxcColdTimes, 50)).toFixed(1)),
    },
    warm: {
      oxc_p50: percentile(oxcWarmTimes, 50),
      babel_p50: percentile(babelWarmTimes, 50),
      speedup: parseFloat((percentile(babelWarmTimes, 50) / percentile(oxcWarmTimes, 50)).toFixed(1)),
      changed_file: fixtures[changedIdx].name,
    },
  };
}

function printViteResults(result) {
  console.log('Simulates a Vite dev server transform pipeline with content-hash caching.\n');
  console.log(`| Scenario | OXC p50 | Babel p50 | Speedup |`);
  console.log(`|----------|---------|-----------|---------|`);
  console.log(`| Cold build (${fixtures.length} files, no cache) | ${fmtNs(result.cold.oxc_p50)} | ${fmtNs(result.cold.babel_p50)} | **${result.cold.speedup}x** |`);
  console.log(`| Warm HMR rebuild (1 file changed) | ${fmtNs(result.warm.oxc_p50)} | ${fmtNs(result.warm.babel_p50)} | **${result.warm.speedup}x** |`);
  console.log(`\nChanged file: \`${result.warm.changed_file}\` (largest fixture)\n`);
}

// ============================================================
// Run selected sections
// ============================================================
const report = { timestamp: new Date().toISOString(), config: { iterations, warmup } };

console.log('# OXC vs Babel React Compiler — Comparative Benchmark\n');
console.log(`Date: ${report.timestamp}`);
console.log(`Platform: ${process.platform} ${process.arch}, Node ${process.version}\n`);

if (section === 'all' || section === '1') {
  const perFixture = runPerFixtureBenchmark();
  printPerFixtureResults(perFixture);
  report.per_fixture = perFixture;
}

if (section === 'all' || section === '2') {
  const batch = runBatchBenchmark();
  printBatchResults(batch);
  report.batch = batch;
}

if (section === 'all' || section === '3') {
  const vite = runViteSimulation();
  printViteResults(vite);
  report.vite = vite;
}

// --- Memory usage ---
const mem = process.memoryUsage();
console.log(`\n**Memory**: RSS ${(mem.rss / 1024 / 1024).toFixed(1)} MB, Heap ${(mem.heapUsed / 1024 / 1024).toFixed(1)} MB`);

// --- Write JSON report ---
if (format === 'json') {
  report.memory = { rss_bytes: mem.rss, heap_used_bytes: mem.heapUsed };
  const reportPath = join(benchDir, 'compare-report.json');
  writeFileSync(reportPath, JSON.stringify(report, null, 2));
  console.log(`\nJSON report written to ${reportPath}`);
}
