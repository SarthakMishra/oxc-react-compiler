#!/usr/bin/env node
/**
 * Pre-process fbt fixture input files through babel-plugin-fbt and
 * babel-plugin-fbt-runtime.
 *
 * The upstream React Compiler test infrastructure runs these Babel plugins
 * alongside the React Compiler, so the .expect.md (expected) outputs contain
 * fbt._() / fbt._param() calls instead of <fbt> JSX. To match those expected
 * outputs, we pre-process the input files the same way.
 *
 * This modifies files IN PLACE (the upstream-fixtures/ directory is gitignored).
 *
 * Usage:
 *   cd tests/conformance && npm install  # one-time
 *   node preprocess-fbt.mjs
 *
 * Prerequisites: @babel/core, babel-plugin-fbt, babel-plugin-fbt-runtime,
 *   @babel/preset-typescript (all in devDependencies)
 */

import { transformSync } from '@babel/core';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.join(__dirname, 'upstream-fixtures');

if (!fs.existsSync(fixturesDir)) {
  console.error(`Fixtures directory not found: ${fixturesDir}`);
  console.error('Run ./download-upstream.sh first.');
  process.exit(1);
}

const EXTENSIONS = new Set(['.tsx', '.ts', '.js', '.jsx']);

function collectFiles(dir) {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectFiles(fullPath));
    } else if (EXTENSIONS.has(path.extname(entry.name))
               && !entry.name.endsWith('.expected')
               && !entry.name.endsWith('.expect.md')) {
      files.push(fullPath);
    }
  }
  return files.sort();
}

// Check if a file contains fbt/fbs usage
function needsFbtTransform(filepath) {
  const content = fs.readFileSync(filepath, 'utf-8');
  return (
    content.includes("from 'fbt'") ||
    content.includes('from "fbt"') ||
    content.includes("require('fbt')") ||
    content.includes('<fbt') ||
    content.includes('<fbs')
  );
}

const allFiles = collectFiles(fixturesDir);
const fbtFiles = allFiles.filter(needsFbtTransform);

console.log(`Found ${fbtFiles.length} files with fbt usage out of ${allFiles.length} total.`);

let success = 0;
let unchanged = 0;
let errors = 0;

for (const filepath of fbtFiles) {
  const source = fs.readFileSync(filepath, 'utf-8');
  const ext = path.extname(filepath);

  try {
    // Use onlyRemoveTypeImports to preserve the fbt import binding
    // (without this, TypeScript preset removes the import since fbt is
    // used as a JSX tag, not as a value expression)
    const presets = (ext === '.tsx' || ext === '.ts')
      ? [['@babel/preset-typescript', { isTSX: ext === '.tsx', allExtensions: true, onlyRemoveTypeImports: true }]]
      : [];

    const result = transformSync(source, {
      filename: filepath,
      plugins: ['babel-plugin-fbt', 'babel-plugin-fbt-runtime'],
      presets,
      parserOpts: {
        plugins: ['jsx', 'typescript'],
      },
      configFile: false,
      babelrc: false,
      sourceMaps: false,
    });

    if (result && result.code && result.code !== source) {
      fs.writeFileSync(filepath, result.code, 'utf-8');
      success++;
    } else {
      unchanged++;
    }
  } catch (err) {
    // Some fbt fixtures are intentionally erroneous (error.todo-* fixtures)
    const relPath = path.relative(fixturesDir, filepath);
    console.error(`  SKIP (error): ${relPath}: ${err.message.split('\n')[0]}`);
    errors++;
  }
}

console.log(`\nDone! ${success} transformed, ${unchanged} unchanged, ${errors} errors/skipped.`);
