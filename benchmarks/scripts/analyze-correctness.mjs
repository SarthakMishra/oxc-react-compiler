#!/usr/bin/env node
/**
 * Deep correctness analysis for OXC React Compiler output.
 *
 * Analyzes compiled output to extract memoization patterns:
 * - useMemoCache(N) calls and slot counts
 * - Cache sentinel checks ($[i] === Symbol.for("react.memo_cache_sentinel"))
 * - Dependency checks ($[i] !== dep)
 * - Scope block boundaries
 *
 * Usage:
 *   node analyze-correctness.mjs [--fixture name] [--all] [--format json|markdown]
 */

import { readFileSync, existsSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const __dirname = dirname(fileURLToPath(import.meta.url));
const benchDir = join(__dirname, '..');

// Load NAPI binding
let oxcBinding;
try {
  oxcBinding = require(join(benchDir, '../napi/react-compiler'));
} catch (err) {
  console.error('Failed to load OXC NAPI binding.');
  process.exit(1);
}

// Parse args
const args = process.argv.slice(2);
const fixtureName = args.find((a, i) => args[i - 1] === '--fixture') || '';
const analyzeAll = args.includes('--all');
const format = args.find((a, i) => args[i - 1] === '--format') || 'markdown';

/**
 * Analyze compiled output for memoization patterns.
 */
function analyzeOutput(code) {
  const analysis = {
    hasMemoCache: false,
    memoCacheSize: 0,
    sentinelChecks: 0,
    dependencyChecks: 0,
    cacheReads: 0,
    cacheWrites: 0,
    scopeBlocks: 0,
    issues: [],
  };

  // Check for useMemoCache / _c import
  const cacheImport = code.match(/import\s*\{[^}]*\bc\b[^}]*\}\s*from\s*["']react\/compiler-runtime["']/);
  analysis.hasMemoCache = !!cacheImport;

  // Check for _c(N) call
  const cacheCall = code.match(/_c\((\d+)\)/);
  if (cacheCall) {
    analysis.memoCacheSize = parseInt(cacheCall[1], 10);
  }

  // Count sentinel checks: $[N] === Symbol.for("react.memo_cache_sentinel")
  const sentinelChecks = code.match(/\$\[\d+\]\s*===\s*Symbol\.for\(["']react\.memo_cache_sentinel["']\)/g);
  analysis.sentinelChecks = sentinelChecks ? sentinelChecks.length : 0;

  // Count dependency checks: $[N] !== someVar
  const depChecks = code.match(/\$\[\d+\]\s*!==\s*\w+/g);
  analysis.dependencyChecks = depChecks ? depChecks.length : 0;

  // Count cache reads: varName = $[N]
  const cacheReads = code.match(/\w+\s*=\s*\$\[\d+\]/g);
  analysis.cacheReads = cacheReads ? cacheReads.length : 0;

  // Count cache writes: $[N] = expr
  const cacheWrites = code.match(/\$\[\d+\]\s*=\s*[^=]/g);
  analysis.cacheWrites = cacheWrites ? cacheWrites.length : 0;

  // Count if/else scope blocks (memoization boundaries)
  const scopeBlocks = code.match(/if\s*\(\$\[\d+\]/g);
  analysis.scopeBlocks = scopeBlocks ? scopeBlocks.length : 0;

  // Issue detection
  if (analysis.hasMemoCache && analysis.memoCacheSize === 0) {
    analysis.issues.push('imports compiler-runtime but never calls _c()');
  }
  if (analysis.memoCacheSize > 0 && analysis.scopeBlocks === 0) {
    analysis.issues.push('allocates cache slots but has no scope blocks');
  }
  if (analysis.cacheWrites > analysis.memoCacheSize) {
    analysis.issues.push(`more cache writes (${analysis.cacheWrites}) than slots (${analysis.memoCacheSize})`);
  }
  if (analysis.sentinelChecks > 0 && analysis.dependencyChecks === 0) {
    analysis.issues.push('uses sentinel checks but no dependency checks (may over-cache)');
  }

  // Classify divergence type
  if (analysis.issues.length > 0) {
    analysis.divergenceType = 'conservative_miss';
  } else if (analysis.hasMemoCache && analysis.scopeBlocks > 0) {
    analysis.divergenceType = 'ok';
  } else {
    analysis.divergenceType = 'no_memoization';
  }

  return analysis;
}

// Load fixtures
const manifestPath = join(benchDir, 'fixtures/manifest.json');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf-8'));
let fixtures = manifest.fixtures;

if (fixtureName) {
  fixtures = fixtures.filter((f) => f.name === fixtureName);
}

if (fixtures.length === 0) {
  console.error('No fixtures found.');
  process.exit(1);
}

// Analyze each fixture
const results = [];

for (const fixture of fixtures) {
  const source = readFileSync(join(benchDir, 'fixtures', fixture.file), 'utf-8');
  const compiled = oxcBinding.transformReactFile(source, fixture.file);

  const analysis = analyzeOutput(compiled.code);
  results.push({
    fixture: fixture.name,
    size_tier: fixture.size_tier,
    transformed: compiled.transformed,
    ...analysis,
  });
}

// Output
if (format === 'json') {
  const output = {
    timestamp: new Date().toISOString(),
    results,
  };
  const outPath = join(benchDir, 'correctness-report.json');
  writeFileSync(outPath, JSON.stringify(output, null, 2));
  console.log(`Report written to ${outPath}`);
} else {
  console.log('# Correctness Analysis Report\n');
  console.log('| Fixture | Tier | Transformed | Cache Size | Scopes | Sentinel | Dep Checks | Status | Issues |');
  console.log('|---------|------|-------------|------------|--------|----------|------------|--------|--------|');

  for (const r of results) {
    const status = r.divergenceType === 'ok' ? 'OK' : r.divergenceType === 'no_memoization' ? 'NO_MEMO' : 'WARN';
    const issues = r.issues.length > 0 ? r.issues.join('; ') : '-';
    console.log(
      `| ${r.fixture} | ${r.size_tier} | ${r.transformed} | ${r.memoCacheSize} | ${r.scopeBlocks} | ${r.sentinelChecks} | ${r.dependencyChecks} | ${status} | ${issues} |`
    );
  }

  const okCount = results.filter((r) => r.divergenceType === 'ok').length;
  const total = results.length;
  console.log(`\nCorrectness: ${okCount}/${total} fixtures fully memoized`);
}
