#!/usr/bin/env node
/**
 * Vite Plugin Cache Performance Measurement
 *
 * Measures the performance improvement from the in-memory content-hash cache
 * by simulating cold and warm transform passes over the benchmark fixtures.
 *
 * Usage:
 *   node benchmarks/scripts/vite-cache-bench.mjs
 *
 * Prerequisites:
 *   - NAPI binding built: cd napi/react-compiler && npx napi build --release
 */

import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { performance } from 'node:perf_hooks';

const require = createRequire(import.meta.url);
const __dirname = dirname(fileURLToPath(import.meta.url));
const benchDir = join(__dirname, '..');

// Load NAPI binding
let binding;
try {
  binding = require(join(benchDir, '../napi/react-compiler'));
} catch (err) {
  console.error('Failed to load OXC NAPI binding. Build it first:');
  console.error('  cd napi/react-compiler && npm install && npx napi build --release');
  process.exit(1);
}

// Load fixtures
const manifest = JSON.parse(readFileSync(join(benchDir, 'fixtures/manifest.json'), 'utf-8'));
const fixtures = manifest.fixtures.map((f) => ({
  name: f.name,
  file: join(benchDir, 'fixtures', f.file),
  source: readFileSync(join(benchDir, 'fixtures', f.file), 'utf-8'),
  tier: f.size_tier,
}));

console.log(`Loaded ${fixtures.length} fixtures\n`);

// --- Simulate cold compile (no cache) ---
function coldCompile(fixtures) {
  const results = [];
  for (const f of fixtures) {
    const start = performance.now();
    try {
      binding.transformReactFile(f.source, f.file, {});
    } catch {
      // Ignore errors for benchmarking
    }
    results.push({ name: f.name, ms: performance.now() - start });
  }
  return results;
}

// --- Simulate warm rebuild (cache hit for all but one file) ---
// Cache stores { contentHash, code, map } per file
function warmRebuild(fixtures, changedIndex) {
  // Build cache from cold compile
  const cache = new Map();
  for (const f of fixtures) {
    const hash = createHash('md5').update(f.source).digest('hex');
    try {
      const result = binding.transformReactFile(f.source, f.file, {});
      cache.set(f.file, { contentHash: hash, code: result.code, map: null });
    } catch {
      cache.set(f.file, { contentHash: hash, code: '', map: null });
    }
  }

  // Now simulate a rebuild where only changedIndex is a miss
  const results = [];
  for (let i = 0; i < fixtures.length; i++) {
    const f = fixtures[i];
    const hash = createHash('md5').update(f.source).digest('hex');
    const start = performance.now();

    if (i === changedIndex) {
      // Cache miss: recompile
      try {
        binding.transformReactFile(f.source, f.file, {});
      } catch {
        // Ignore
      }
    } else {
      // Cache hit: just hash check
      const cached = cache.get(f.file);
      if (cached && cached.contentHash === hash) {
        // Hit — return cached (no-op)
      }
    }

    results.push({ name: f.name, ms: performance.now() - start, hit: i !== changedIndex });
  }
  return results;
}

// --- Run benchmarks ---
console.log('=== Cold Compile (no cache) ===\n');

// Warm up JIT
coldCompile(fixtures);

const ITERATIONS = 5;
const coldRuns = [];
for (let i = 0; i < ITERATIONS; i++) {
  coldRuns.push(coldCompile(fixtures));
}

// Average cold times
const coldAvg = fixtures.map((f, idx) => {
  const avg = coldRuns.reduce((s, run) => s + run[idx].ms, 0) / ITERATIONS;
  return { name: f.name, tier: f.tier, ms: avg };
});

let totalCold = 0;
for (const r of coldAvg) {
  console.log(`  ${r.name.padEnd(25)} ${r.tier.padEnd(4)} ${r.ms.toFixed(2)}ms`);
  totalCold += r.ms;
}
console.log(`  ${'TOTAL'.padEnd(25)}      ${totalCold.toFixed(2)}ms\n`);

console.log('=== Warm Rebuild (single file change) ===\n');

// Simulate changing the largest fixture (last one typically)
const changedIdx = fixtures.length - 1;
const warmRuns = [];
for (let i = 0; i < ITERATIONS; i++) {
  warmRuns.push(warmRebuild(fixtures, changedIdx));
}

const warmAvg = fixtures.map((f, idx) => {
  const avg = warmRuns.reduce((s, run) => s + run[idx].ms, 0) / ITERATIONS;
  const hit = idx !== changedIdx;
  return { name: f.name, tier: f.tier, ms: avg, hit };
});

let totalWarm = 0;
let cacheHits = 0;
let cacheMisses = 0;
for (const r of warmAvg) {
  const status = r.hit ? 'HIT' : 'MISS';
  console.log(`  ${r.name.padEnd(25)} ${r.tier.padEnd(4)} ${r.ms.toFixed(4)}ms  ${status}`);
  totalWarm += r.ms;
  if (r.hit) cacheHits++;
  else cacheMisses++;
}
console.log(`  ${'TOTAL'.padEnd(25)}      ${totalWarm.toFixed(4)}ms\n`);

console.log('=== Summary ===\n');
console.log(`  Cold compile total:     ${totalCold.toFixed(2)}ms`);
console.log(`  Warm rebuild total:     ${totalWarm.toFixed(4)}ms`);
console.log(`  Speedup:                ${(totalCold / totalWarm).toFixed(1)}x`);
console.log(`  Warm/Cold ratio:        ${((totalWarm / totalCold) * 100).toFixed(2)}%`);
console.log(`  Cache hit rate:         ${cacheHits}/${cacheHits + cacheMisses} (${((cacheHits / (cacheHits + cacheMisses)) * 100).toFixed(1)}%)`);
console.log(`  Changed file:           ${fixtures[changedIdx].name}`);
