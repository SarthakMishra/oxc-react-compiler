#!/usr/bin/env node
/**
 * Run upstream babel-plugin-react-compiler on fixture inputs to generate
 * expected outputs (.expected files) for differential comparison.
 *
 * Usage:
 *   # Install dependencies first (one-time):
 *   npm install --save-dev babel-plugin-react-compiler @babel/core
 *
 *   # Generate expected outputs:
 *   node tests/conformance/run-upstream.mjs
 *
 * The script processes all .tsx/.ts/.js files in upstream-fixtures/ and
 * writes a corresponding .expected file with the Babel plugin output.
 */

import { transformSync } from '@babel/core';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const fixturesDir = path.join(__dirname, 'upstream-fixtures');

if (!fs.existsSync(fixturesDir)) {
  console.error(`Fixtures directory not found: ${fixturesDir}`);
  console.error('Run ./tests/conformance/download-upstream.sh first.');
  process.exit(1);
}

const EXTENSIONS = new Set(['.tsx', '.ts', '.js', '.jsx']);

function collectFiles(dir) {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectFiles(fullPath));
    } else if (EXTENSIONS.has(path.extname(entry.name))) {
      files.push(fullPath);
    }
  }
  return files.sort();
}

const files = collectFiles(fixturesDir);
console.log(`Processing ${files.length} fixture files...`);

let success = 0;
let errors = 0;

for (const filepath of files) {
  const source = fs.readFileSync(filepath, 'utf-8');
  const ext = path.extname(filepath);
  const expectedPath = filepath.replace(/\.[^.]+$/, '.expected');

  try {
    const result = transformSync(source, {
      filename: filepath,
      plugins: ['babel-plugin-react-compiler'],
      presets: ext === '.tsx' || ext === '.ts'
        ? [['@babel/preset-typescript', { isTSX: ext === '.tsx', allExtensions: true }]]
        : [],
      parserOpts: {
        plugins: ['jsx', 'typescript'],
      },
      // Don't output source maps for comparison
      sourceMaps: false,
    });

    if (result && result.code) {
      fs.writeFileSync(expectedPath, result.code, 'utf-8');
      success++;
    } else {
      // Plugin may have decided not to transform
      fs.writeFileSync(expectedPath, source, 'utf-8');
      success++;
    }
  } catch (err) {
    // Write error info as the expected output so we can track which
    // fixtures the upstream plugin also fails on.
    fs.writeFileSync(expectedPath, `// UPSTREAM ERROR: ${err.message}\n`, 'utf-8');
    errors++;
  }

  if ((success + errors) % 100 === 0) {
    console.log(`  Processed ${success + errors} / ${files.length}...`);
  }
}

console.log(`\nDone! ${success} succeeded, ${errors} errored.`);
console.log(`Expected files written to ${fixturesDir}`);
