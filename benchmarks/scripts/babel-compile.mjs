#!/usr/bin/env node
/**
 * Compile fixtures using Babel's babel-plugin-react-compiler for comparison.
 *
 * Produces .babel.js snapshots and .diff.json structural diff reports.
 *
 * Usage:
 *   node babel-compile.mjs [--fixture name] [--update-snapshots] [--diff]
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const __dirname = dirname(fileURLToPath(import.meta.url));
const benchDir = join(__dirname, '..');

// --- Parse args ---
const args = process.argv.slice(2);
const fixtureName = args.find((a, i) => args[i - 1] === '--fixture') || '';
const updateSnapshots = args.includes('--update-snapshots');
const doDiff = args.includes('--diff');
const format = args.find((a, i) => args[i - 1] === '--format') || 'markdown';

// --- Load Babel and plugin ---
let babel, reactCompilerPlugin;
try {
  babel = require('@babel/core');
  reactCompilerPlugin = require('babel-plugin-react-compiler');
} catch (err) {
  console.error('Failed to load Babel or babel-plugin-react-compiler.');
  console.error('Run: npm install @babel/core @babel/preset-react @babel/preset-typescript babel-plugin-react-compiler');
  process.exit(1);
}

// --- Load OXC binding ---
let oxcBinding;
try {
  oxcBinding = require(join(benchDir, '../napi/react-compiler'));
} catch (err) {
  console.error('Failed to load OXC NAPI binding.');
  process.exit(1);
}

/**
 * Compile a fixture with Babel's react-compiler plugin.
 */
function compileWithBabel(source, filename) {
  try {
    const result = babel.transformSync(source, {
      filename,
      presets: [
        ['@babel/preset-react', { runtime: 'automatic' }],
        ['@babel/preset-typescript', { isTSX: true, allExtensions: true }],
      ],
      plugins: [reactCompilerPlugin],
      sourceType: 'module',
    });
    return { code: result?.code || '', error: null };
  } catch (err) {
    return { code: '', error: err.message };
  }
}

/**
 * Compile a fixture with OXC.
 */
function compileWithOxc(source, filename) {
  try {
    const result = oxcBinding.transformReactFile(source, filename);
    return { code: result.code, error: null, transformed: result.transformed };
  } catch (err) {
    return { code: '', error: err.message, transformed: false };
  }
}

/**
 * Extract memoization patterns from compiled output.
 */
function extractPatterns(code) {
  const patterns = {
    cacheSize: 0,
    sentinelChecks: 0,
    dependencyChecks: 0,
    scopeBlocks: 0,
    cacheReads: 0,
    cacheWrites: 0,
  };

  // useMemoCache(N)
  const cacheCall = code.match(/useMemoCache\((\d+)\)|_c\((\d+)\)/);
  if (cacheCall) {
    patterns.cacheSize = parseInt(cacheCall[1] || cacheCall[2], 10);
  }

  // Sentinel checks
  const sentinels = code.match(/\$\[\d+\]\s*===\s*Symbol\.for\(["']react\.memo_cache_sentinel["']\)/g);
  patterns.sentinelChecks = sentinels ? sentinels.length : 0;

  // Dependency checks: $[N] !== expr
  const depChecks = code.match(/\$\[\d+\]\s*!==\s*\w+/g);
  patterns.dependencyChecks = depChecks ? depChecks.length : 0;

  // Scope blocks: if ($[N]
  const scopes = code.match(/if\s*\(\$\[\d+\]/g);
  patterns.scopeBlocks = scopes ? scopes.length : 0;

  // Cache reads: x = $[N]
  const reads = code.match(/\w+\s*=\s*\$\[\d+\]/g);
  patterns.cacheReads = reads ? reads.length : 0;

  // Cache writes: $[N] = expr
  const writes = code.match(/\$\[\d+\]\s*=\s*[^=]/g);
  patterns.cacheWrites = writes ? writes.length : 0;

  return patterns;
}

/**
 * Classify the divergence between OXC and Babel outputs.
 */
function classifyDivergence(oxcPatterns, babelPatterns) {
  if (babelPatterns.cacheSize === 0 && oxcPatterns.cacheSize === 0) {
    return { type: 'cosmetic', severity: 'ok', details: 'Neither compiler memoized' };
  }

  if (babelPatterns.cacheSize > 0 && oxcPatterns.cacheSize === 0) {
    return { type: 'semantic_difference', severity: 'bug', details: 'Babel memoizes but OXC does not' };
  }

  if (babelPatterns.cacheSize === 0 && oxcPatterns.cacheSize > 0) {
    return { type: 'over_memoization', severity: 'investigate', details: 'OXC memoizes but Babel does not' };
  }

  // Both memoize — compare structure
  const cacheDiff = oxcPatterns.cacheSize - babelPatterns.cacheSize;
  const scopeDiff = oxcPatterns.scopeBlocks - babelPatterns.scopeBlocks;

  if (cacheDiff === 0 && scopeDiff === 0) {
    return { type: 'cosmetic', severity: 'ok', details: 'Same memoization structure' };
  }

  if (cacheDiff < 0) {
    return {
      type: 'conservative_miss',
      severity: 'acceptable',
      details: `OXC uses ${Math.abs(cacheDiff)} fewer cache slots`,
    };
  }

  if (cacheDiff > 0) {
    return {
      type: 'over_memoization',
      severity: 'investigate',
      details: `OXC uses ${cacheDiff} more cache slots`,
    };
  }

  return {
    type: 'cosmetic',
    severity: 'ok',
    details: `Scope block difference: ${scopeDiff}`,
  };
}

// --- Load fixtures ---
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

// --- Process each fixture ---
const snapshotDir = join(benchDir, 'snapshots');
mkdirSync(snapshotDir, { recursive: true });

const diffResults = [];

for (const fixture of fixtures) {
  const source = readFileSync(join(benchDir, 'fixtures', fixture.file), 'utf-8');

  const babelResult = compileWithBabel(source, fixture.file);
  const oxcResult = compileWithOxc(source, fixture.file);

  // Update Babel snapshots
  if (updateSnapshots && babelResult.code) {
    const snapshotPath = join(snapshotDir, `${fixture.name}.babel.js`);
    writeFileSync(snapshotPath, babelResult.code);
    console.log(`Updated Babel snapshot: ${fixture.name}`);
  }

  // Compute structural diff
  if (doDiff || updateSnapshots) {
    const oxcPatterns = extractPatterns(oxcResult.code || '');
    const babelPatterns = extractPatterns(babelResult.code || '');
    const divergence = classifyDivergence(oxcPatterns, babelPatterns);

    const diff = {
      fixture: fixture.name,
      size_tier: fixture.size_tier,
      babel_error: babelResult.error,
      oxc_error: oxcResult.error,
      oxc_transformed: oxcResult.transformed,
      oxc_patterns: oxcPatterns,
      babel_patterns: babelPatterns,
      divergence,
    };

    diffResults.push(diff);

    if (updateSnapshots) {
      const diffPath = join(snapshotDir, `${fixture.name}.diff.json`);
      writeFileSync(diffPath, JSON.stringify(diff, null, 2));
    }
  }
}

// --- Output diff report ---
if (doDiff || !updateSnapshots) {
  if (diffResults.length === 0) {
    // If no diff was computed, compute now
    for (const fixture of fixtures) {
      const source = readFileSync(join(benchDir, 'fixtures', fixture.file), 'utf-8');
      const babelResult = compileWithBabel(source, fixture.file);
      const oxcResult = compileWithOxc(source, fixture.file);
      const oxcPatterns = extractPatterns(oxcResult.code || '');
      const babelPatterns = extractPatterns(babelResult.code || '');
      const divergence = classifyDivergence(oxcPatterns, babelPatterns);

      diffResults.push({
        fixture: fixture.name,
        size_tier: fixture.size_tier,
        babel_error: babelResult.error,
        oxc_patterns: oxcPatterns,
        babel_patterns: babelPatterns,
        divergence,
      });
    }
  }

  if (format === 'json') {
    const output = { timestamp: new Date().toISOString(), results: diffResults };
    writeFileSync(join(benchDir, 'diff-report.json'), JSON.stringify(output, null, 2));
    console.log('Diff report written to diff-report.json');
  } else {
    console.log('# OXC vs Babel Structural Comparison\n');
    console.log('| Fixture | Tier | OXC Slots | Babel Slots | OXC Scopes | Babel Scopes | Classification | Severity |');
    console.log('|---------|------|-----------|-------------|------------|--------------|----------------|----------|');

    for (const r of diffResults) {
      const babelErr = r.babel_error ? 'ERR' : '';
      console.log(
        `| ${r.fixture} | ${r.size_tier} | ${r.oxc_patterns.cacheSize} | ${babelErr || r.babel_patterns.cacheSize} | ${r.oxc_patterns.scopeBlocks} | ${babelErr || r.babel_patterns.scopeBlocks} | ${r.divergence.type} | ${r.divergence.severity} |`
      );
    }

    // Summary
    const byType = {};
    for (const r of diffResults) {
      byType[r.divergence.type] = (byType[r.divergence.type] || 0) + 1;
    }
    console.log('\n## Summary\n');
    for (const [type, count] of Object.entries(byType)) {
      console.log(`- **${type}**: ${count} fixture(s)`);
    }

    // Score
    const totalSites = diffResults.reduce((s, r) => s + Math.max(r.babel_patterns.cacheSize, r.oxc_patterns.cacheSize), 0);
    const semanticDivergences = diffResults.filter((r) => r.divergence.type === 'semantic_difference').length;
    const score = totalSites > 0 ? (1.0 - semanticDivergences / diffResults.length).toFixed(3) : 'N/A';
    console.log(`\n**Correctness Score**: ${score}`);
  }
}

if (updateSnapshots) {
  console.log(`\nUpdated ${fixtures.length} Babel snapshots and diff reports.`);
}
