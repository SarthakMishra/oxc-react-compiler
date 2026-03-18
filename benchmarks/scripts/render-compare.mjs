#!/usr/bin/env node
/**
 * Headless Render Comparison (Gap 3b)
 *
 * Compiles benchmark fixtures with both OXC and Babel, renders each
 * with ReactDOMServer using identical props sequences, and compares
 * HTML output to detect semantic divergences.
 *
 * Usage:
 *   node render-compare.mjs [--fixture name] [--format json|markdown] [--verbose]
 */

import { readFileSync, writeFileSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';
import vm from 'vm';

const require = createRequire(import.meta.url);
const __dirname = dirname(fileURLToPath(import.meta.url));
const benchDir = join(__dirname, '..');

// --- Parse args ---
const args = process.argv.slice(2);
const fixtureName = args.find((a, i) => args[i - 1] === '--fixture') || '';
const format = args.find((a, i) => args[i - 1] === '--format') || 'markdown';
const verbose = args.includes('--verbose');

// --- Load dependencies ---
let babel, reactCompilerPlugin;
try {
  babel = require('@babel/core');
  reactCompilerPlugin = require('babel-plugin-react-compiler');
} catch (err) {
  console.error('Failed to load Babel or babel-plugin-react-compiler.');
  console.error('Run: npm install');
  process.exit(1);
}

let oxcBinding;
try {
  oxcBinding = require(join(benchDir, '../napi/react-compiler'));
} catch (err) {
  console.error('Failed to load OXC NAPI binding.');
  process.exit(1);
}

let React, ReactDOMServer, esbuild;
try {
  React = require('react');
  ReactDOMServer = require('react-dom/server');
  esbuild = require('esbuild');
} catch (err) {
  console.error('Failed to load React/esbuild. Run: npm install react react-dom esbuild');
  process.exit(1);
}

// --- JSX transform ---
function stripJsx(source) {
  const result = esbuild.transformSync(source, {
    loader: 'tsx',
    jsx: 'automatic',
    jsxImportSource: 'react',
    format: 'cjs',
    target: 'es2020',
  });
  return result.code;
}

// --- Component extraction ---
function extractComponentName(source) {
  const exportDefault = source.match(/export\s+default\s+function\s+(\w+)/);
  if (exportDefault) return exportDefault[1];

  const exportNamed = source.match(/export\s+function\s+(\w+)/);
  if (exportNamed) return exportNamed[1];

  const funcDecls = [...source.matchAll(/(?:^|\n)\s*function\s+([A-Z]\w*)/g)];
  if (funcDecls.length > 0) return funcDecls[0][1];

  const constDecls = [...source.matchAll(/(?:^|\n)\s*const\s+([A-Z]\w*)\s*=/g)];
  if (constDecls.length > 0) return constDecls[0][1];

  return null;
}

/**
 * Evaluate source and render to HTML with given props.
 * Returns { html, error }.
 */
function renderComponent(source, props) {
  const compName = extractComponentName(source);
  if (!compName) {
    return { html: null, error: 'No component found in source' };
  }

  let code;
  try {
    code = stripJsx(source);
  } catch (err) {
    return { html: null, error: `JSX transform failed: ${err.message}` };
  }

  // Create compiler-runtime mock
  const compilerRuntime = {
    c: (size) => new Array(size).fill(Symbol.for('react.memo_cache_sentinel')),
  };

  const exports = {};
  const moduleObj = { exports };

  const sandbox = {
    React,
    require: (mod) => {
      if (mod === 'react') return React;
      if (mod === 'react/jsx-runtime') return require('react/jsx-runtime');
      if (mod === 'react/jsx-dev-runtime') return require('react/jsx-runtime');
      if (mod === 'react/compiler-runtime') return compilerRuntime;
      throw new Error(`Unsupported require: ${mod}`);
    },
    module: moduleObj,
    exports,
    console,
    Symbol,
    setTimeout: globalThis.setTimeout,
    clearTimeout: globalThis.clearTimeout,
    document: undefined,
    window: undefined,
  };

  code += `\nmodule.exports.Component = ${compName};`;

  try {
    vm.runInNewContext(code, sandbox, {
      filename: 'render-compare.js',
      timeout: 5000,
    });
  } catch (err) {
    return { html: null, error: `Eval failed: ${err.message}` };
  }

  const Component = moduleObj.exports.Component;
  if (!Component || typeof Component !== 'function') {
    return { html: null, error: 'Component not a function after eval' };
  }

  try {
    const html = ReactDOMServer.renderToStaticMarkup(
      React.createElement(Component, props)
    );
    return { html, error: null };
  } catch (err) {
    return { html: null, error: `Render failed: ${err.message}` };
  }
}

// --- Compile helpers ---
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

function compileWithOxc(source, filename) {
  try {
    const result = oxcBinding.transformReactFile(source, filename);
    return { code: result.code, error: null, transformed: result.transformed };
  } catch (err) {
    return { code: '', error: err.message, transformed: false };
  }
}

// --- Props sequences for each fixture ---
// Each fixture gets a list of { label, props } representing different states to render.
const propsMap = {
  'simple-counter': [
    { label: 'initial', props: {} },
  ],
  'todo-list': [
    { label: 'empty', props: {} },
  ],
  'form-validation': [
    { label: 'empty', props: {} },
  ],
  'data-table': [
    { label: 'empty', props: { data: [], columns: [{ key: 'name', label: 'Name' }] } },
  ],
  'theme-toggle': [
    { label: 'initial', props: {} },
  ],
  'status-badge': [
    { label: 'confirmed', props: { status: 'confirmed' } },
    { label: 'pending', props: { status: 'pending' } },
    { label: 'cancelled', props: { status: 'cancelled' } },
    { label: 'completed', props: { status: 'completed' } },
  ],
  'avatar-group': [
    { label: 'single', props: { users: [{ name: 'Alice' }] } },
    { label: 'three', props: { users: [{ name: 'A' }, { name: 'B' }, { name: 'C' }] } },
    { label: 'overflow', props: { users: [{ name: 'A' }, { name: 'B' }, { name: 'C' }, { name: 'D' }, { name: 'E' }], max: 3 } },
  ],
  'search-input': [
    { label: 'default', props: { onSearch: () => {} } },
    { label: 'with-placeholder', props: { onSearch: () => {}, placeholder: 'Search...' } },
  ],
  'toolbar': [
    { label: 'defaults', props: {} },
  ],
  'time-slot-picker': [
    { label: 'no-selection', props: { slots: [{ time: '09:00', available: true }, { time: '10:00', available: true }], selectedDate: '2026-03-12', onSelect: () => {} } },
    { label: 'with-booked', props: { slots: [{ time: '09:00', available: false }, { time: '10:00', available: true }], selectedDate: '2026-03-12', onSelect: () => {} } },
  ],
  'color-picker': [
    { label: 'default', props: { onColorChange: () => {} } },
    { label: 'with-initial', props: { initialColor: '#ff0000', onColorChange: () => {} } },
  ],
  'command-menu': [
    { label: 'default', props: { items: [{ id: '1', label: 'Copy', action: () => {}, category: 'edit' }, { id: '2', label: 'Paste', action: () => {}, category: 'edit' }] } },
  ],
  'booking-list': [
    { label: 'empty', props: { bookings: [], onCancel: () => {}, onReschedule: () => {}, onConfirm: () => {} } },
    { label: 'with-data', props: { bookings: [
      { id: '1', title: 'Meeting', startTime: '2026-03-12T09:00:00', endTime: '2026-03-12T10:00:00', attendees: [{ name: 'Alice', email: 'alice@test.com', status: 'accepted' }], status: 'confirmed' },
      { id: '2', title: 'Call', startTime: '2026-03-13T14:00:00', endTime: '2026-03-13T15:00:00', attendees: [{ name: 'Bob', email: 'bob@test.com', status: 'pending' }], status: 'pending' },
    ], onCancel: () => {}, onReschedule: () => {}, onConfirm: () => {} } },
  ],
  'availability-schedule': [
    { label: 'default', props: { onSave: () => {} } },
  ],
  'canvas-sidebar': [
    { label: 'empty', props: { layers: [{ id: '1', name: 'Layer 1', visible: true, locked: false, opacity: 1, elements: 3 }], activeLayerId: '1', onLayerSelect: () => {}, onLayerToggleVisible: () => {}, onLayerToggleLock: () => {}, onLayerRename: () => {}, onLayerReorder: () => {}, onLayerDelete: () => {}, onLayerAdd: () => {}, onLayerDuplicate: () => {}, onLayerOpacity: () => {} } },
  ],
  'multi-step-form': [
    { label: 'default', props: { steps: [{ title: 'Step 1', description: 'First step', fields: [{ name: 'name', label: 'Name', type: 'text', required: true }] }], onSubmit: () => {} } },
  ],
};

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

// --- Run comparison ---
const results = [];

for (const fixture of fixtures) {
  const source = readFileSync(join(benchDir, 'fixtures', fixture.file), 'utf-8');
  const propsSequence = propsMap[fixture.name] || [{ label: 'default', props: {} }];

  // Compile with both compilers
  const babelResult = compileWithBabel(source, fixture.file);
  const oxcResult = compileWithOxc(source, fixture.file);

  const fixtureResult = {
    fixture: fixture.name,
    size_tier: fixture.size_tier,
    babel_compile_error: babelResult.error,
    oxc_compile_error: oxcResult.error,
    renders: [],
    verdict: 'unknown',
  };

  if (babelResult.error && oxcResult.error) {
    fixtureResult.verdict = 'both_compile_error';
    results.push(fixtureResult);
    continue;
  }

  let allMatch = true;
  let anyError = false;

  for (const { label, props } of propsSequence) {
    // Render original (uncompiled) source
    const originalRender = renderComponent(source, props);

    // Render Babel-compiled
    const babelRender = babelResult.error
      ? { html: null, error: 'compile_error' }
      : renderComponent(babelResult.code, props);

    // Render OXC-compiled
    const oxcRender = oxcResult.error
      ? { html: null, error: 'compile_error' }
      : renderComponent(oxcResult.code, props);

    const babelMatch = originalRender.html !== null && babelRender.html !== null
      ? originalRender.html === babelRender.html
      : null;
    const oxcMatch = originalRender.html !== null && oxcRender.html !== null
      ? originalRender.html === oxcRender.html
      : null;
    const babelOxcMatch = babelRender.html !== null && oxcRender.html !== null
      ? babelRender.html === oxcRender.html
      : null;

    if (oxcMatch === false || babelOxcMatch === false) allMatch = false;
    if (originalRender.error || babelRender.error || oxcRender.error) anyError = true;

    const renderResult = {
      label,
      original: { html: originalRender.html, error: originalRender.error },
      babel: { html: babelRender.html, error: babelRender.error, matchesOriginal: babelMatch },
      oxc: { html: oxcRender.html, error: oxcRender.error, matchesOriginal: oxcMatch },
      babelOxcMatch,
    };

    fixtureResult.renders.push(renderResult);

    if (verbose) {
      const hasIssue = oxcMatch === false || babelOxcMatch === false ||
        originalRender.error || babelRender.error || oxcRender.error;
      if (hasIssue) {
        console.log(`\n--- ${fixture.name} / ${label} ---`);
        if (originalRender.error) console.log(`  Original ERROR: ${originalRender.error}`);
        else console.log(`  Original: ${originalRender.html.substring(0, 150)}`);
        if (babelRender.error) console.log(`  Babel ERROR: ${babelRender.error}`);
        else console.log(`  Babel:    ${babelRender.html.substring(0, 150)}`);
        if (oxcRender.error) console.log(`  OXC ERROR: ${oxcRender.error}`);
        else console.log(`  OXC:      ${oxcRender.html.substring(0, 150)}`);
      }
    }
  }

  // Classify verdict
  if (anyError && !allMatch) {
    fixtureResult.verdict = 'render_error';
  } else if (anyError && allMatch) {
    fixtureResult.verdict = 'partial_error';
  } else if (allMatch) {
    fixtureResult.verdict = 'semantic_match';
  } else {
    fixtureResult.verdict = 'semantic_divergence';
  }

  results.push(fixtureResult);
}

// --- Output ---
if (format === 'json') {
  const output = {
    timestamp: new Date().toISOString(),
    fixtures: results.length,
    results,
  };
  const outPath = join(benchDir, 'render-report.json');
  writeFileSync(outPath, JSON.stringify(output, null, 2));
  console.log(`Render comparison report written to ${outPath}`);
} else {
  console.log('# Headless Render Comparison (OXC vs Babel vs Original)\n');
  console.log('| Fixture | Tier | Props Tested | OXC=Orig | Babel=Orig | OXC=Babel | Verdict |');
  console.log('|---------|------|-------------|----------|------------|-----------|---------|');

  for (const r of results) {
    const totalRenders = r.renders.length;
    const oxcOrigMatches = r.renders.filter((rr) => rr.oxc.matchesOriginal === true).length;
    const babelOrigMatches = r.renders.filter((rr) => rr.babel.matchesOriginal === true).length;
    const babelOxcMatches = r.renders.filter((rr) => rr.babelOxcMatch === true).length;

    const oxcOrigStr = r.oxc_compile_error ? 'ERR' : `${oxcOrigMatches}/${totalRenders}`;
    const babelOrigStr = r.babel_compile_error ? 'ERR' : `${babelOrigMatches}/${totalRenders}`;
    const babelOxcStr = (r.oxc_compile_error || r.babel_compile_error) ? 'ERR' : `${babelOxcMatches}/${totalRenders}`;

    console.log(
      `| ${r.fixture} | ${r.size_tier} | ${totalRenders} | ${oxcOrigStr} | ${babelOrigStr} | ${babelOxcStr} | ${r.verdict} |`
    );
  }

  // Summary
  const byVerdict = {};
  for (const r of results) {
    byVerdict[r.verdict] = (byVerdict[r.verdict] || 0) + 1;
  }
  console.log('\n## Summary\n');
  for (const [verdict, count] of Object.entries(byVerdict)) {
    console.log(`- **${verdict}**: ${count} fixture(s)`);
  }

  // Render equivalence score
  const totalRenderPairs = results.reduce((s, r) => s + r.renders.length, 0);
  const matchingPairs = results.reduce(
    (s, r) => s + r.renders.filter((rr) => rr.oxc.matchesOriginal === true).length,
    0
  );
  const score = totalRenderPairs > 0 ? (matchingPairs / totalRenderPairs).toFixed(3) : 'N/A';
  console.log(`\n**Render Equivalence Score (OXC vs Original)**: ${score} (${matchingPairs}/${totalRenderPairs} render pairs match)`);
}
