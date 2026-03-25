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
 *
 * IMPORTANT: The upstream React Compiler test suite defaults to
 * compilationMode: "all" (compile all functions, not just components/hooks).
 * This script mirrors that behavior by default and parses @annotation
 * comments to override options per-fixture, matching the upstream test
 * infrastructure.
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

/**
 * Parse @annotation directives from source file comments.
 * Returns an object with the plugin environment config overrides.
 *
 * Supported annotations (matching upstream test infrastructure):
 *   @compilationMode:"infer" | "all" | "annotation" | "syntax"
 *   @panicThreshold:"ALL_ERRORS" | "NONE" | "CRITICAL_ERRORS"
 *   @gating
 *   @validatePreserveExistingMemoizationGuarantees
 *   @enablePreserveExistingMemoizationGuarantees:false
 *   @validateRefAccessDuringRender
 *   @validateNoSetStateInRender
 *   @validateNoSetStateInEffects
 *   @validateExhaustiveMemoizationDependencies
 *   @enableNewMutationAliasingModel
 *   etc.
 */
function parseAnnotations(source) {
  const environment = {};
  const pluginOpts = {};

  for (const line of source.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('//')) {
      if (trimmed.length > 0 && !trimmed.startsWith('/*') && !trimmed.startsWith('*')) {
        break; // Stop at first non-comment, non-empty line
      }
      continue;
    }
    const comment = trimmed.replace(/^\/\/\s*/, '');

    // Parse individual annotations from the comment
    // Multiple annotations can appear on one line: // @foo @bar:"baz"
    const annotationRegex = /@(\w+)(?::(?:"([^"]*)"|\[([^\]]*)\]|(\S+)))?/g;
    let match;
    while ((match = annotationRegex.exec(comment)) !== null) {
      const key = match[1];
      const value = match[2] ?? match[3] ?? match[4] ?? true;

      switch (key) {
        case 'compilationMode':
          pluginOpts.compilationMode = value;
          break;
        case 'panicThreshold':
          pluginOpts.panicThreshold = value;
          break;
        case 'gating':
          pluginOpts.gating = {
            source: 'ReactForgetFeatureFlag',
            importSpecifierName: 'isForgetEnabled_Fixtures',
          };
          break;
        case 'enablePreserveExistingMemoizationGuarantees':
          environment.enablePreserveExistingMemoizationGuarantees =
            value === true || value === 'true';
          break;
        case 'validatePreserveExistingMemoizationGuarantees':
          environment.validatePreserveExistingMemoizationGuarantees =
            value === true || value === 'true';
          break;
        case 'validateRefAccessDuringRender':
          environment.validateRefAccessDuringRender =
            value === true || value === 'true';
          break;
        case 'validateNoSetStateInRender':
          environment.validateNoSetStateInRender =
            value === true || value === 'true';
          break;
        case 'validateNoSetStateInEffects':
          environment.validateNoSetStateInEffects =
            value === true || value === 'true';
          break;
        case 'validateExhaustiveMemoizationDependencies':
          if (value === 'false') {
            environment.validateExhaustiveDeps = false;
          } else {
            environment.validateExhaustiveDeps = true;
          }
          break;
        case 'enableNewMutationAliasingModel':
          environment.enableNewMutationAliasingModel =
            value === true || value === 'true';
          break;
        case 'enableAssumeHooksFollowRulesOfReact':
          environment.enableAssumeHooksFollowRulesOfReact =
            value === true || value === 'true';
          break;
        case 'enableTreatRefLikeIdentifiersAsRefs':
          environment.enableTreatRefLikeIdentifiersAsRefs =
            value === true || value === 'true';
          break;
        case 'enableJsxOutlining':
          environment.enableJsxOutlining =
            value === true || value === 'true';
          break;
        case 'validateNoFreezingKnownMutableFunctions':
          environment.validateNoFreezingKnownMutableFunctions =
            value === true || value === 'true';
          break;
        case 'validateNoCapitalizedCalls':
          environment.validateNoCapitalizedCalls =
            value === true || value === 'true';
          break;
        case 'validateNoJSXInTryStatements':
          environment.validateNoJSXInTryStatements =
            value === true || value === 'true';
          break;
        case 'validateNoVoidUseMemo':
          environment.validateNoVoidUseMemo =
            value === true || value === 'true';
          break;
        case 'validateNoImpureFunctionsInRender':
          environment.validateNoImpureFunctionsInRender =
            value === true || value === 'true';
          break;
        case 'validateNoDerivedComputationsInEffects':
          environment.validateNoDerivedComputationsInEffects =
            value === true || value === 'true';
          break;
        // target:"ssr" etc.
        case 'target':
          environment.target = value;
          break;
        // @flow annotation — skip
        case 'flow':
          break;
        // @debug — skip
        case 'debug':
          break;
        // @loggerTestOnly — skip (affects test logging only)
        case 'loggerTestOnly':
          break;
        // @outputMode — lint, ssr, etc.
        case 'outputMode':
          // Not a plugin option in babel-plugin-react-compiler
          break;
        // @expectNothingCompiled — test assertion only
        case 'expectNothingCompiled':
          break;
        default:
          // Unknown annotation — ignore
          break;
      }
    }
  }

  return { environment, pluginOpts };
}

const files = collectFiles(fixturesDir);
console.log(`Processing ${files.length} fixture files...`);

let success = 0;
let errors = 0;

for (const filepath of files) {
  const source = fs.readFileSync(filepath, 'utf-8');
  const ext = path.extname(filepath);
  const expectedPath = filepath.replace(/\.[^.]+$/, '.expected');

  const { environment, pluginOpts } = parseAnnotations(source);

  // Build plugin options matching upstream test defaults:
  // compilationMode: "all" (compile all functions, not just components/hooks)
  // panicThreshold: "ALL_ERRORS" (bail on any error, matching upstream test behavior)
  const pluginConfig = {
    compilationMode: pluginOpts.compilationMode || 'all',
    panicThreshold: pluginOpts.panicThreshold || 'ALL_ERRORS',
  };

  // Only include environment config if annotations override specific settings
  if (Object.keys(environment).length > 0) {
    pluginConfig.environment = environment;
  }

  // Add gating config if specified
  if (pluginOpts.gating) {
    pluginConfig.gating = pluginOpts.gating;
  }

  // Resolve plugin path explicitly to handle the array format
  const pluginPath = path.join(__dirname, 'node_modules', 'babel-plugin-react-compiler');

  try {
    const result = transformSync(source, {
      filename: filepath,
      plugins: [[pluginPath, pluginConfig]],
      presets: ext === '.tsx' || ext === '.ts'
        ? [[path.join(__dirname, 'node_modules', '@babel', 'preset-typescript'), { isTSX: ext === '.tsx', allExtensions: true }]]
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
