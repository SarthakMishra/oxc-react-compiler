#!/usr/bin/env node
/**
 * Runtime Performance Benchmark: OXC vs Babel compiled output
 *
 * Measures SSR render timing as a proxy for runtime performance.
 * Compiles each fixture with both compilers, then renders the output
 * multiple times with ReactDOMServer.renderToString() and compares
 * render latency.
 *
 * This tests the quality of compiled output — well-memoized code should
 * render comparably or faster than unmemoized code on repeated renders
 * with unchanged props.
 *
 * Usage:
 *   node scripts/runtime-bench.mjs [--iterations N] [--format json]
 */

import { readFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';
import vm from 'vm';

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

const iterations = parseInt(getArg('iterations', '200'), 10);
const warmupRuns = parseInt(getArg('warmup', '50'), 10);
const format = getArg('format', 'markdown');

// --- Load compilers ---
let oxcBinding;
try {
  oxcBinding = require(join(benchDir, '../napi/react-compiler'));
} catch {
  console.error('Failed to load OXC NAPI binding.');
  process.exit(1);
}

let babel, reactCompilerPlugin;
try {
  babel = require('@babel/core');
  reactCompilerPlugin = require('babel-plugin-react-compiler');
} catch {
  console.error('Failed to load Babel or babel-plugin-react-compiler.');
  process.exit(1);
}

let esbuild;
try {
  esbuild = require('esbuild');
} catch {
  console.error('esbuild required for JSX transformation. Run: cd benchmarks && npm install');
  process.exit(1);
}

// --- Load React for SSR ---
const React = require('react');
const ReactDOMServer = require('react-dom/server');

// --- Load fixtures ---
const manifest = JSON.parse(readFileSync(join(benchDir, 'fixtures/manifest.json'), 'utf-8'));
const fixtures = manifest.fixtures;

// --- Compiler runtime mock ---
const compilerRuntimeCode = `
  const $cache = new WeakMap();
  function c(size) {
    const component = c._currentComponent || {};
    let cache = $cache.get(component);
    if (!cache) {
      cache = new Array(size).fill(Symbol.for("react.memo_cache_sentinel"));
      $cache.set(component, cache);
    }
    return cache;
  }
  c._currentComponent = null;
  c._enter = function(comp) { c._currentComponent = comp; };
  c._exit = function() { c._currentComponent = null; };
`;

// Props sequences for rendering
const propsMap = {
  'simple-counter': [{}],
  'todo-list': [{}],
  'form-validation': [{}],
  'data-table': [{ data: [], columns: [] }],
  'status-badge': [
    { status: 'confirmed' },
    { status: 'pending' },
    { status: 'cancelled' },
  ],
  'theme-toggle': [{}],
  'avatar-group': [{ users: [] }],
  'search-input': [{ onSearch: () => {} }],
  'toolbar': [{}],
  'time-slot-picker': [{ slots: [] }],
  'color-picker': [{}],
  'command-menu': [{}],
  'booking-list': [{ bookings: [] }],
  'canvas-sidebar': [{}],
  'availability-schedule': [{}],
  'multi-step-form': [{}],
};

function compileOxc(source, filename) {
  try {
    const result = oxcBinding.transformReactFile(source, filename);
    return { code: result.code, error: null };
  } catch (err) {
    return { code: '', error: err.message };
  }
}

function compileBabel(source, filename) {
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

function compileOriginal(source, filename) {
  try {
    const result = babel.transformSync(source, {
      filename,
      presets: [
        ['@babel/preset-react', { runtime: 'automatic' }],
        ['@babel/preset-typescript', { isTSX: true, allExtensions: true }],
      ],
      sourceType: 'module',
    });
    return { code: result?.code || '', error: null };
  } catch (err) {
    return { code: '', error: err.message };
  }
}

/**
 * Extract the default-exported component name from source.
 */
function extractComponentName(source) {
  const match = source.match(/export\s+default\s+function\s+(\w+)/);
  if (match) return match[1];
  const match2 = source.match(/export\s+default\s+(\w+)/);
  if (match2) return match2[1];
  const match3 = source.match(/function\s+([A-Z]\w*)\s*\(/);
  if (match3) return match3[1];
  return null;
}

/**
 * Prepare compiled code for VM execution by transforming JSX and handling imports.
 */
function prepareForVM(code) {
  try {
    const result = esbuild.transformSync(code, {
      loader: 'jsx',
      format: 'cjs',
      target: 'es2020',
      jsx: 'automatic',
      jsxImportSource: 'react',
    });
    return result.code;
  } catch {
    return null;
  }
}

/**
 * Create a sandboxed VM context for rendering.
 */
function createSandbox() {
  const sandbox = {
    React,
    require: (mod) => {
      if (mod === 'react') return React;
      if (mod === 'react/jsx-runtime' || mod === 'react/jsx-dev-runtime') return require('react/jsx-runtime');
      if (mod === 'react-dom/server') return ReactDOMServer;
      if (mod === 'react/compiler-runtime') return { c: sandbox._c };
      return {};
    },
    exports: {},
    module: { exports: {} },
    console,
    Symbol,
    Array,
    Object,
    Map,
    Set,
    WeakMap,
    process: { env: {} },
    setTimeout,
    clearTimeout,
  };

  // Setup compiler runtime
  vm.runInNewContext(compilerRuntimeCode, sandbox, { timeout: 2000 });
  sandbox._c = sandbox.c;

  return sandbox;
}

function percentile(sorted, p) {
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, idx)];
}

function fmtUs(ns) {
  return (ns / 1000).toFixed(1) + ' µs';
}

// ============================================================
// Main benchmark
// ============================================================
console.log('# Runtime Performance Benchmark: SSR Render Timing\n');
console.log(`Iterations: ${iterations}, Warmup: ${warmupRuns}\n`);

const results = [];

for (const fixture of fixtures) {
  const source = readFileSync(join(benchDir, 'fixtures', fixture.file), 'utf-8');
  const componentName = extractComponentName(source);
  const props = propsMap[fixture.name] || [{}];

  // Compile with all three modes
  const original = compileOriginal(source, fixture.file);
  const oxc = compileOxc(source, fixture.file);
  const babelResult = compileBabel(source, fixture.file);

  // Prepare for VM
  const origCode = original.code ? prepareForVM(original.code) : null;
  const oxcCode = oxc.code ? prepareForVM(oxc.code) : null;
  const babelCode = babelResult.code ? prepareForVM(babelResult.code) : null;

  if (!origCode || !componentName) {
    results.push({
      name: fixture.name,
      size_tier: fixture.size_tier,
      error: 'Could not prepare original code',
    });
    continue;
  }

  // Helper: render in a sandbox and time it
  function benchmarkRender(code, label) {
    if (!code) return null;

    try {
      const sandbox = createSandbox();
      vm.runInNewContext(code, sandbox, { timeout: 5000 });

      const Component = sandbox.module.exports.default || sandbox.exports.default || sandbox.module.exports[componentName];
      if (!Component) return null;

      // Warmup
      for (let i = 0; i < warmupRuns; i++) {
        for (const p of props) {
          try {
            ReactDOMServer.renderToString(React.createElement(Component, p));
          } catch { break; }
        }
      }

      // Measured
      const times = [];
      for (let i = 0; i < iterations; i++) {
        const start = process.hrtime.bigint();
        for (const p of props) {
          try {
            ReactDOMServer.renderToString(React.createElement(Component, p));
          } catch { break; }
        }
        times.push(Number(process.hrtime.bigint() - start));
      }
      times.sort((a, b) => a - b);

      return {
        p50: percentile(times, 50),
        p95: percentile(times, 95),
      };
    } catch {
      return null;
    }
  }

  const origTiming = benchmarkRender(origCode, 'original');
  const oxcTiming = benchmarkRender(oxcCode, 'oxc');
  const babelTiming = benchmarkRender(babelCode, 'babel');

  results.push({
    name: fixture.name,
    size_tier: fixture.size_tier,
    original: origTiming,
    oxc: oxcTiming,
    babel: babelTiming,
    oxc_vs_original: origTiming && oxcTiming ? parseFloat((origTiming.p50 / oxcTiming.p50).toFixed(2)) : null,
    babel_vs_original: origTiming && babelTiming ? parseFloat((origTiming.p50 / babelTiming.p50).toFixed(2)) : null,
  });
}

// --- Output ---
if (format === 'json') {
  const report = { timestamp: new Date().toISOString(), config: { iterations, warmup: warmupRuns }, results };
  const reportPath = join(benchDir, 'runtime-report.json');
  writeFileSync(reportPath, JSON.stringify(report, null, 2));
  console.log(`JSON report written to ${reportPath}`);
} else {
  console.log('| Fixture | Size | Original p50 | OXC p50 | Babel p50 | OXC vs Orig | Babel vs Orig |');
  console.log('|---------|------|-------------|---------|-----------|-------------|---------------|');

  for (const r of results) {
    if (r.error) {
      console.log(`| ${r.name} | ${r.size_tier} | — | — | — | ${r.error} | — |`);
      continue;
    }
    const origP50 = r.original ? fmtUs(r.original.p50) : '—';
    const oxcP50 = r.oxc ? fmtUs(r.oxc.p50) : 'error';
    const babelP50 = r.babel ? fmtUs(r.babel.p50) : 'error';
    const oxcRatio = r.oxc_vs_original ? `${r.oxc_vs_original}x` : '—';
    const babelRatio = r.babel_vs_original ? `${r.babel_vs_original}x` : '—';
    console.log(`| ${r.name} | ${r.size_tier} | ${origP50} | ${oxcP50} | ${babelP50} | ${oxcRatio} | ${babelRatio} |`);
  }

  // Summary
  const valid = results.filter((r) => r.oxc && r.babel && r.original);
  if (valid.length > 0) {
    const avgOxcRatio = valid.reduce((s, r) => s + (r.oxc_vs_original || 1), 0) / valid.length;
    const avgBabelRatio = valid.reduce((s, r) => s + (r.babel_vs_original || 1), 0) / valid.length;
    console.log(`\n**Average render ratio** (>1 = faster than uncompiled):`);
    console.log(`  OXC compiled: ${avgOxcRatio.toFixed(2)}x vs original`);
    console.log(`  Babel compiled: ${avgBabelRatio.toFixed(2)}x vs original`);
  }
}
