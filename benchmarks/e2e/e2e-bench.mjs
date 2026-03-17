#!/usr/bin/env node
/**
 * End-to-End Vite Build Benchmark: OXC vs Babel React Compiler
 *
 * Clones real open-source projects that use Vite + React, builds them with
 * babel-plugin-react-compiler (baseline), then patches the Vite config to
 * swap in the OXC compiler plugin and rebuilds.
 *
 * Compares: build time, bundle size, and OXC transform coverage.
 *
 * Cloned repos are shallow (--depth 1) and have .git stripped after cloning —
 * no history is kept. All config changes are applied programmatically and are
 * fully reproducible by re-running this script. See e2e/README.md for details.
 *
 * Usage:
 *   node e2e-bench.mjs [options]
 *
 * Options:
 *   --project name    Run only the named project (default: all)
 *   --skip-clone      Reuse already-cloned repos in .workspace/
 *   --iterations N    Build iterations for timing (default: 3)
 *   --format json     Output JSON report to e2e/reports/
 *   --verbose         Show build output and patched configs
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync, cpSync, rmSync, readdirSync, statSync } from 'fs';
import { join, dirname, resolve, relative } from 'path';
import { fileURLToPath } from 'url';
import { execSync, execFileSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const OXC_ROOT = resolve(__dirname, '../..');
const NAPI_DIR = join(OXC_ROOT, 'napi/react-compiler');
const WORKSPACE_DIR = join(__dirname, '.workspace');
const REPORT_DIR = join(__dirname, 'reports');

// --- Parse CLI args ---
const args = process.argv.slice(2);
function getArg(name, defaultValue) {
  const idx = args.indexOf(`--${name}`);
  if (idx === -1) return defaultValue;
  if (typeof defaultValue === 'boolean') return true;
  return args[idx + 1] ?? defaultValue;
}

const projectFilter = getArg('project', '');
const skipClone = getArg('skip-clone', false);
const buildIterations = parseInt(getArg('iterations', '3'), 10);
const format = getArg('format', 'markdown');
const verbose = getArg('verbose', false);

// --- Project registry ---
// hasReactCompiler: whether babel-plugin-react-compiler is already configured
// needsCompilerSetup: if true, the script installs babel-plugin-react-compiler
//   and patches the vite config to add it for the Babel baseline
// monorepoInstallDir: for monorepos, where to run `pnpm install` (root)
// appDir: the specific app directory within the monorepo
const PROJECTS = [
  {
    name: 'ephe',
    repo: 'https://github.com/unvalley/ephe.git',
    scale: 'small',
    stars: 569,
    description: 'Ephemeral markdown paper / daily planner PWA',
    viteConfigDir: '.',
    viteConfigFile: 'vite.config.ts',
    buildCmd: 'npx vite build',
    distDir: 'dist',
    hasReactCompiler: true,
  },
  {
    name: 'rai-pal',
    repo: 'https://github.com/Raicuparta/rai-pal.git',
    scale: 'medium',
    stars: 684,
    description: 'Mod manager for universal game mods (Tauri + React)',
    viteConfigDir: '.',
    viteConfigFile: 'vite.config.ts',
    buildCmd: 'npx vite build',
    distDir: 'dist',
    hasReactCompiler: true,
  },
  {
    name: 'arcomage-hd',
    repo: 'https://github.com/arcomage/arcomage-hd.git',
    scale: 'large',
    stars: 170,
    description: 'Web HD remaster of Arcomage card game',
    viteConfigDir: '.',
    viteConfigFile: 'vite.config.ts',
    buildCmd: 'npx vite build',
    distDir: 'dist',
    hasReactCompiler: true,
  },
  {
    name: 'docmost',
    repo: 'https://github.com/docmost/docmost.git',
    scale: 'large',
    stars: 10700,
    description: 'Open-source collaborative wiki & documentation (295 React files)',
    viteConfigDir: 'apps/client',
    viteConfigFile: 'vite.config.ts',
    buildCmd: 'npx vite build',
    distDir: 'apps/client/dist',
    hasReactCompiler: false,
    needsCompilerSetup: true,
    monorepoInstallDir: '.',
    appDir: 'apps/client',
    // Workspace deps and tsc must run before vite build
    preBuildCmd: 'cd packages/editor-ext && npx tsc --build && cd ../../apps/client && npx tsc || true',
  },
  // twenty disabled: requires Node ^24.5.0 + complex nx monorepo build chain
  // with multiple workspace packages (twenty-shared, twenty-ui) that must be
  // pre-built via nx before twenty-front can compile.
  // {
  //   name: 'twenty',
  //   repo: 'https://github.com/twentyhq/twenty.git',
  //   ...
  // },
];

// ============================================================
// OXC Vite Plugin — standalone JS (no TS compilation needed)
// ============================================================
const OXC_PLUGIN_JS = `
// oxc-react-compiler vite plugin — inlined for e2e benchmarking
const { createHash } = require('node:crypto');

function isReactFile(id) {
  return /\\.(tsx?|jsx?)$/.test(id) && !id.includes('node_modules');
}

function mightContainReactCode(code) {
  return (
    code.includes('function') || code.includes('=>') || code.includes('use')
  ) && (
    code.includes('return') || code.includes('jsx') || code.includes('JSX') || code.includes('<')
  );
}

function oxcReactCompiler(options = {}) {
  let binding;
  let enableSourceMap = options.sourceMap;
  const cache = new Map();
  const stats = { compiled: 0, skipped: 0, errors: 0 };
  oxcReactCompiler._stats = stats;

  function contentHash(code) {
    return createHash('md5').update(code).digest('hex');
  }

  return {
    name: 'oxc-react-compiler',
    enforce: 'pre',

    configResolved(config) {
      if (enableSourceMap === undefined) {
        enableSourceMap = config.command === 'serve';
      }
    },

    async buildStart() {
      try {
        binding = require('oxc-react-compiler');
      } catch (e) {
        console.warn('[oxc-react-compiler] Failed to load native binding:', e.message);
      }
    },

    transform(code, id) {
      if (!isReactFile(id)) return null;
      if (!mightContainReactCode(code)) return null;
      if (!binding) return null;

      const hash = contentHash(code);
      const cached = cache.get(id);
      if (cached && cached.contentHash === hash) {
        return cached.code ? { code: cached.code, map: cached.map } : null;
      }

      try {
        const result = binding.transformReactFile(code, id, {
          compilationMode: options.compilationMode,
          outputMode: options.outputMode,
          sourceMap: enableSourceMap,
        });

        if (!result.transformed) {
          cache.set(id, { contentHash: hash, code: '', map: null });
          stats.skipped++;
          return null;
        }

        // Validate output parses correctly before returning it.
        // Use esbuild's transform as a fast syntax check.
        try {
          const esbuild = require('esbuild');
          esbuild.transformSync(result.code, { loader: 'tsx', format: 'esm', logLevel: 'silent' });
        } catch (validationErr) {
          console.warn('[oxc-react-compiler] Invalid output for ' + id.split('/').pop() + ': ' + (validationErr.message || '').slice(0, 120));
          cache.set(id, { contentHash: hash, code: '', map: null });
          stats.errors++;
          return null;
        }

        const map = result.sourceMap ? JSON.parse(result.sourceMap) : null;
        cache.set(id, { contentHash: hash, code: result.code, map });
        stats.compiled++;
        return { code: result.code, map };
      } catch (e) {
        // Gracefully fall through — don't break the build
        stats.errors++;
        return null;
      }
    },
    closeBundle() {
      // Write stats to a file so the parent process can read them
      const fs = require('fs');
      const path = require('path');
      const statsPath = path.join(process.cwd(), '_oxc-stats.json');
      fs.writeFileSync(statsPath, JSON.stringify(stats));
      console.log('[oxc-react-compiler] Stats: ' + JSON.stringify(stats));
    },
  };
}

module.exports = { oxcReactCompiler };
`;

// ============================================================
// Helpers
// ============================================================
function run(cmd, cwd, opts = {}) {
  const execOpts = {
    cwd,
    encoding: 'utf-8',
    stdio: verbose ? 'inherit' : 'pipe',
    timeout: 600_000, // 10 min
    env: { ...process.env, NODE_ENV: 'production', FORCE_COLOR: '0' },
    ...opts,
  };
  try {
    return execSync(cmd, execOpts);
  } catch (err) {
    if (!verbose) {
      console.error(`  Command failed: ${cmd}`);
      console.error(`  stderr: ${err.stderr?.toString().slice(0, 500)}`);
    }
    throw err;
  }
}

function dirSize(dir) {
  if (!existsSync(dir)) return { bytes: 0, files: 0 };
  let bytes = 0, files = 0;
  function walk(d) {
    for (const entry of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, entry.name);
      if (entry.isDirectory()) walk(p);
      else { bytes += statSync(p).size; files++; }
    }
  }
  walk(dir);
  return { bytes, files };
}

function fmtBytes(b) {
  if (b >= 1024 * 1024) return (b / 1024 / 1024).toFixed(1) + ' MB';
  if (b >= 1024) return (b / 1024).toFixed(1) + ' KB';
  return b + ' B';
}

function fmtMs(ms) {
  if (ms >= 1000) return (ms / 1000).toFixed(2) + 's';
  return ms.toFixed(0) + 'ms';
}

function countReactFiles(dir) {
  let count = 0;
  function walk(d) {
    for (const entry of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, entry.name);
      if (entry.name === 'node_modules' || entry.name === '.git' || entry.name === 'dist') continue;
      if (entry.isDirectory()) walk(p);
      else if (/\.(tsx|jsx)$/.test(entry.name)) count++;
    }
  }
  walk(dir);
  return count;
}

function median(arr) {
  const sorted = [...arr].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

// ============================================================
// Vite config patching
// ============================================================

/**
 * Read the existing vite config and generate a patched version that
 * replaces babel-plugin-react-compiler with our OXC plugin.
 *
 * Strategy: Remove the entire `babel: { plugins: [...] }` block from the
 * react() plugin config, clean up empty objects and dangling commas, then
 * add our OXC plugin to the Vite plugins array.
 */
function patchViteConfigForOxc(projectDir, configDir, configFile, project = {}) {
  const configPath = join(projectDir, configDir, configFile);
  const originalContent = readFileSync(configPath, 'utf-8');

  // Save original
  const backupPath = configPath + '.babel-backup';
  writeFileSync(backupPath, originalContent);

  // Drop the OXC plugin file into the project
  const pluginPath = join(projectDir, configDir, '_oxc-plugin.cjs');
  writeFileSync(pluginPath, OXC_PLUGIN_JS);

  let patched = originalContent;

  // Add our import at the top
  const oxcImport = `import { createRequire } from 'node:module';\nconst __require = createRequire(import.meta.url);\nconst { oxcReactCompiler } = __require('./_oxc-plugin.cjs');\n`;
  patched = oxcImport + patched;

  // Remove the entire babel config block from react() options.
  // This handles multi-line blocks like:
  //   babel: {
  //     plugins: [["babel-plugin-react-compiler", ReactCompilerConfig]],
  //   },
  // Use a brace-counting approach instead of regex for robustness.
  const babelPropIdx = patched.indexOf('babel:');
  if (babelPropIdx !== -1) {
    // Find the opening brace after 'babel:'
    const openBrace = patched.indexOf('{', babelPropIdx);
    if (openBrace !== -1) {
      let depth = 1;
      let i = openBrace + 1;
      while (i < patched.length && depth > 0) {
        if (patched[i] === '{') depth++;
        else if (patched[i] === '}') depth--;
        i++;
      }
      // i now points past the closing brace
      // Also consume trailing comma and whitespace
      while (i < patched.length && /[\s,]/.test(patched[i])) i++;

      // Find start of the 'babel:' property (back up past whitespace/newlines)
      let start = babelPropIdx;
      while (start > 0 && /[\s,]/.test(patched[start - 1])) start--;
      // If we backed into a comma, include it
      if (start > 0 && patched[start - 1] === ',') start--;

      patched = patched.slice(0, start) + patched.slice(i);
    }
  }

  // Clean up: react({}) → react(), react({ }) → react()
  patched = patched.replace(/react\(\s*\{\s*\}\s*\)/g, 'react()');

  // Clean up dangling commas in objects: {, } or { ,} → {}
  patched = patched.replace(/\{\s*,\s*\}/g, '{}');

  // Clean up empty options that might have only had babel config
  // e.g., react({\n  }) → react()
  patched = patched.replace(/react\(\s*\{\s*\}\s*\)/g, 'react()');

  // Add OXC plugin to the Vite plugins array
  patched = patched.replace(
    /(plugins\s*:\s*\[)/,
    '$1\n    oxcReactCompiler(),\n    '
  );

  // For projects that don't natively have react/compiler-runtime (React 18),
  // add a Vite resolve alias so OXC's compiled output can import it.
  if (project.needsCompilerSetup) {
    // Check if there's already a resolve.alias in the config
    if (patched.includes('resolve:') && patched.includes('alias:')) {
      // Inject into existing alias block
      patched = patched.replace(
        /(alias\s*:\s*\{)/,
        `$1\n      'react/compiler-runtime': 'react-compiler-runtime',`
      );
    } else if (patched.includes('resolve:')) {
      // resolve exists but no alias — add alias property
      patched = patched.replace(
        /(resolve\s*:\s*\{)/,
        `$1\n    alias: { 'react/compiler-runtime': 'react-compiler-runtime' },`
      );
    } else {
      // No resolve block — add one to defineConfig
      patched = patched.replace(
        /(plugins\s*:\s*\[)/,
        `resolve: {\n    alias: { 'react/compiler-runtime': 'react-compiler-runtime' },\n  },\n  $1`
      );
    }
  }

  writeFileSync(configPath, patched);
  return { backupPath, pluginPath, patchedContent: patched };
}

function restoreViteConfig(projectDir, configDir, configFile) {
  const configPath = join(projectDir, configDir, configFile);
  const backupPath = configPath + '.babel-backup';
  if (existsSync(backupPath)) {
    const original = readFileSync(backupPath, 'utf-8');
    writeFileSync(configPath, original);
    rmSync(backupPath, { force: true });
  }
  const pluginPath = join(projectDir, configDir, '_oxc-plugin.cjs');
  rmSync(pluginPath, { force: true });
}

/**
 * Patch vite config to add babel-plugin-react-compiler for projects that
 * don't already have it configured. Creates a backup for later restoration.
 *
 * For Babel-based projects: adds the plugin to react()'s babel config.
 * For SWC-based projects: replaces react-swc with react + babel compiler.
 */
function patchViteConfigForBabelCompiler(projectDir, project) {
  const configPath = join(projectDir, project.viteConfigDir, project.viteConfigFile);
  const originalContent = readFileSync(configPath, 'utf-8');

  // Save original
  const backupPath = configPath + '.babel-backup';
  writeFileSync(backupPath, originalContent);

  let patched = originalContent;

  if (project.usesSWC) {
    // Replace @vitejs/plugin-react-swc with @vitejs/plugin-react + compiler
    // Change the import
    patched = patched.replace(
      /import\s+react\s+from\s+['"]@vitejs\/plugin-react-swc['"]/,
      `import react from '@vitejs/plugin-react'`
    );

    // Find the react() call and replace its config
    // Remove SWC-specific options and add babel config with compiler
    // Strategy: replace react({ ...swcStuff }) with react({ babel: { plugins: [...compiler] } })
    // Use brace-counting to find the react() call arguments
    const reactCallIdx = patched.indexOf('react(');
    if (reactCallIdx !== -1) {
      const openParen = patched.indexOf('(', reactCallIdx);
      let depth = 1;
      let i = openParen + 1;
      while (i < patched.length && depth > 0) {
        if (patched[i] === '(') depth++;
        else if (patched[i] === ')') depth--;
        i++;
      }
      // Replace everything inside react(...) with our babel config
      patched = patched.slice(0, openParen + 1) +
        `{\n      babel: {\n        plugins: [["babel-plugin-react-compiler", { target: "18" }]],\n      },\n    }` +
        patched.slice(i - 1);
    }
  } else {
    // Babel-based: add babel-plugin-react-compiler to existing react() config
    // Check if react() already has a config object
    const reactCallMatch = patched.match(/react\s*\(/);
    if (reactCallMatch) {
      const reactCallIdx = patched.indexOf(reactCallMatch[0]);
      const afterParen = reactCallIdx + reactCallMatch[0].length;

      // Check what follows the opening paren
      const afterContent = patched.slice(afterParen).trimStart();

      if (afterContent.startsWith(')')) {
        // react() — no args, add babel config
        patched = patched.slice(0, afterParen) +
          `{\n      babel: {\n        plugins: [["babel-plugin-react-compiler", { target: "18" }]],\n      },\n    }` +
          patched.slice(afterParen);
      } else if (afterContent.startsWith('{')) {
        // react({ ... }) — add babel config inside
        // Find the position of the first { after react(
        const braceIdx = patched.indexOf('{', afterParen);
        patched = patched.slice(0, braceIdx + 1) +
          `\n      babel: {\n        plugins: [["babel-plugin-react-compiler"]],\n      },` +
          patched.slice(braceIdx + 1);
      }
    }
  }

  writeFileSync(configPath, patched);
  console.log(`  Patched: added babel-plugin-react-compiler to react() config`);

  if (verbose) {
    console.log('  Babel-patched config (first 30 lines):');
    console.log(patched.split('\n').slice(0, 30).map(l => '    ' + l).join('\n'));
  }
}

/**
 * Link our OXC NAPI binding into the project's node_modules.
 */
function linkOxcBinding(projectDir, configDir) {
  const nmDir = join(projectDir, configDir, 'node_modules', 'oxc-react-compiler');
  mkdirSync(nmDir, { recursive: true });

  // Copy the binding files
  const filesToCopy = ['index.js', 'oxc-react-compiler.node'];
  for (const f of filesToCopy) {
    const src = join(NAPI_DIR, f);
    if (existsSync(src)) {
      cpSync(src, join(nmDir, f));
    }
  }

  // Create a minimal package.json
  writeFileSync(join(nmDir, 'package.json'), JSON.stringify({
    name: 'oxc-react-compiler',
    version: '0.1.0',
    main: 'index.js',
  }));
}

function unlinkOxcBinding(projectDir, configDir) {
  const nmDir = join(projectDir, configDir, 'node_modules', 'oxc-react-compiler');
  rmSync(nmDir, { recursive: true, force: true });
}

// ============================================================
// Build + measure
// ============================================================
function buildProject(projectDir, configDir, buildCmd, distDir, label) {
  const cwd = join(projectDir, configDir);
  const fullDistDir = join(projectDir, distDir);

  // Clean dist
  rmSync(fullDistDir, { recursive: true, force: true });

  const times = [];
  let lastBuildOutput = '';

  for (let i = 0; i < buildIterations; i++) {
    rmSync(fullDistDir, { recursive: true, force: true });

    const start = Date.now();
    try {
      lastBuildOutput = run(buildCmd, cwd, { stdio: 'pipe' }) || '';
    } catch (err) {
      const stdout = err.stdout?.toString() || '';
      const stderr = err.stderr?.toString() || '';
      console.error(`  ⚠ ${label} build ${i + 1}/${buildIterations} failed`);
      if (stderr) console.error(`  stderr (last 500 chars): ${stderr.slice(-500)}`);
      if (stdout) console.error(`  stdout (last 500 chars): ${stdout.slice(-500)}`);
      lastBuildOutput = stdout || stderr;
      times.push(-1); // Mark as failed
      continue;
    }
    times.push(Date.now() - start);
    console.log(`  ${label} build ${i + 1}/${buildIterations}: ${fmtMs(times[times.length - 1])}`);
  }

  const successTimes = times.filter((t) => t > 0);
  const dist = dirSize(fullDistDir);

  return {
    label,
    times,
    successCount: successTimes.length,
    medianMs: successTimes.length > 0 ? median(successTimes) : null,
    minMs: successTimes.length > 0 ? Math.min(...successTimes) : null,
    dist,
    buildOutput: lastBuildOutput.toString(),
  };
}

// ============================================================
// Compare build outputs
// ============================================================
function compareDists(babelDistDir, oxcDistDir) {
  const babelFiles = new Map();
  const oxcFiles = new Map();

  function collect(dir, map, base) {
    if (!existsSync(dir)) return;
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      const rel = relative(base, full);
      if (entry.isDirectory()) collect(full, map, base);
      else {
        map.set(rel, { size: statSync(full).size, path: full });
      }
    }
  }

  collect(babelDistDir, babelFiles, babelDistDir);
  collect(oxcDistDir, oxcFiles, oxcDistDir);

  // Find JS bundle files to compare (main chunks)
  const babelJS = [...babelFiles.entries()].filter(([k]) => k.endsWith('.js'));
  const oxcJS = [...oxcFiles.entries()].filter(([k]) => k.endsWith('.js'));

  const babelJSSize = babelJS.reduce((s, [, v]) => s + v.size, 0);
  const oxcJSSize = oxcJS.reduce((s, [, v]) => s + v.size, 0);

  return {
    babel: { totalFiles: babelFiles.size, jsFiles: babelJS.length, jsBytes: babelJSSize },
    oxc: { totalFiles: oxcFiles.size, jsFiles: oxcJS.length, jsBytes: oxcJSSize },
    jsSizeDelta: oxcJSSize - babelJSSize,
    jsSizePct: babelJSSize > 0 ? ((oxcJSSize - babelJSSize) / babelJSSize * 100).toFixed(1) : 'N/A',
  };
}

// ============================================================
// Main pipeline
// ============================================================
async function benchmarkProject(project) {
  console.log(`\n${'='.repeat(60)}`);
  console.log(`Project: ${project.name} (${project.scale}) — ${project.description}`);
  console.log(`Repo: ${project.repo}`);
  console.log(`${'='.repeat(60)}\n`);

  const projectDir = join(WORKSPACE_DIR, project.name);

  // Step 1: Clone (shallow, then strip .git to save disk and avoid nested repo issues)
  if (!skipClone || !existsSync(projectDir)) {
    console.log('→ Cloning repository...');
    rmSync(projectDir, { recursive: true, force: true });
    mkdirSync(WORKSPACE_DIR, { recursive: true });
    run(`git clone --depth 1 ${project.repo} ${project.name}`, WORKSPACE_DIR);
    // Remove .git — we don't need history or tracking. All config changes
    // are applied programmatically by this script and are fully reproducible.
    const dotGit = join(projectDir, '.git');
    if (existsSync(dotGit)) {
      rmSync(dotGit, { recursive: true, force: true });
      console.log('  Stripped .git directory (not needed for benchmarking)');
    }
  } else {
    console.log('→ Reusing existing clone');
  }

  // Run post-clone commands (e.g., relax Node version constraints)
  if (project.postCloneCmd) {
    console.log('→ Running post-clone setup...');
    try {
      run(project.postCloneCmd, projectDir);
    } catch (err) {
      console.log(`  ⚠ Post-clone step had errors (continuing): ${err.message?.slice(0, 100)}`);
    }
  }

  // Count React files
  const reactFileCount = countReactFiles(join(projectDir, project.viteConfigDir));
  console.log(`  React files (.tsx/.jsx): ${reactFileCount}`);

  // Step 2: Install dependencies
  console.log('→ Installing dependencies...');
  // For monorepos, install from the root; otherwise from viteConfigDir
  const installDir = project.monorepoInstallDir
    ? join(projectDir, project.monorepoInstallDir)
    : join(projectDir, project.viteConfigDir);
  // Detect package manager
  const hasYarnLock = existsSync(join(projectDir, 'yarn.lock'));
  const hasPnpmLock = existsSync(join(projectDir, 'pnpm-lock.yaml'));
  const hasBunLock = existsSync(join(projectDir, 'bun.lockb')) || existsSync(join(projectDir, 'bun.lock'));

  let installCmd;
  if (hasPnpmLock) installCmd = 'pnpm install --frozen-lockfile || pnpm install --no-frozen-lockfile';
  else if (hasYarnLock) installCmd = 'yarn install --immutable || yarn install';
  else if (hasBunLock) installCmd = 'bun install --frozen-lockfile || bun install';
  else installCmd = 'npm install';

  try {
    run(installCmd, installDir);
  } catch {
    // Try fallback without frozen lockfile
    console.log('  ⚠ Frozen lockfile failed, retrying without...');
    if (hasPnpmLock) run('pnpm install --no-frozen-lockfile', installDir);
    else if (hasYarnLock) run('yarn install', installDir);
    else run('npm install', installDir);
  }

  // Step 2a: Run pre-build commands (workspace deps, tsc, etc.)
  if (project.preBuildCmd) {
    console.log('→ Running pre-build steps...');
    try {
      run(project.preBuildCmd, projectDir);
    } catch (err) {
      console.log(`  ⚠ Pre-build step had errors (continuing): ${err.message?.slice(0, 100)}`);
    }
  }

  // Step 2b: For projects without react-compiler, install it + set up Babel baseline
  if (project.needsCompilerSetup) {
    console.log('→ Installing babel-plugin-react-compiler for baseline...');
    const appInstallDir = project.appDir ? join(projectDir, project.appDir) : installDir;

    if (project.usesSWC) {
      // SWC projects need @vitejs/plugin-react (Babel-based) added
      console.log('  Project uses SWC — installing @vitejs/plugin-react for Babel baseline');
      if (hasPnpmLock) {
        run('pnpm add -D babel-plugin-react-compiler react-compiler-runtime @vitejs/plugin-react @babel/core @babel/preset-react @babel/preset-typescript', appInstallDir);
      } else if (hasYarnLock) {
        run('yarn add -D babel-plugin-react-compiler react-compiler-runtime @vitejs/plugin-react @babel/core @babel/preset-react @babel/preset-typescript', appInstallDir);
      } else {
        run('npm install -D babel-plugin-react-compiler react-compiler-runtime @vitejs/plugin-react @babel/core @babel/preset-react @babel/preset-typescript', appInstallDir);
      }
    } else {
      // Babel-based projects just need the compiler plugin
      // Also install react-compiler-runtime for React 18 projects
      if (hasPnpmLock) {
        run('pnpm add -D babel-plugin-react-compiler react-compiler-runtime', appInstallDir);
      } else if (hasYarnLock) {
        run('yarn add -D babel-plugin-react-compiler react-compiler-runtime', appInstallDir);
      } else {
        run('npm install -D babel-plugin-react-compiler react-compiler-runtime', appInstallDir);
      }
    }
  }

  // Step 3: Read vite config for analysis
  const viteConfigPath = join(projectDir, project.viteConfigDir, project.viteConfigFile);
  const viteConfig = existsSync(viteConfigPath) ? readFileSync(viteConfigPath, 'utf-8') : '';
  console.log(`  Vite config: ${project.viteConfigDir}/${project.viteConfigFile} (${viteConfig.length} chars)`);

  // Step 3b: For projects without react-compiler, patch config for Babel baseline
  if (project.needsCompilerSetup) {
    console.log('→ Patching Vite config to add Babel React Compiler baseline...');
    patchViteConfigForBabelCompiler(projectDir, project);
  }

  // Step 4: Babel baseline build
  console.log('\n→ Building with Babel React Compiler (baseline)...');
  const babelResult = buildProject(
    projectDir, project.viteConfigDir, project.buildCmd, project.distDir, 'Babel'
  );

  // Step 4b: Restore config if we patched it for Babel
  if (project.needsCompilerSetup) {
    restoreViteConfig(projectDir, project.viteConfigDir, project.viteConfigFile);
  }

  // Save babel dist for comparison
  const babelDistDir = join(projectDir, project.distDir);
  const babelDistBackup = babelDistDir + '-babel';
  if (existsSync(babelDistDir)) {
    rmSync(babelDistBackup, { recursive: true, force: true });
    cpSync(babelDistDir, babelDistBackup, { recursive: true });
  }

  // Step 5: Patch for OXC and build
  console.log('\n→ Patching Vite config for OXC...');
  let oxcResult;
  let distComparison = null;
  let oxcStats = null;

  try {
    patchViteConfigForOxc(projectDir, project.viteConfigDir, project.viteConfigFile, project);
    linkOxcBinding(projectDir, project.viteConfigDir);

    // Show the patched config
    if (verbose) {
      const patched = readFileSync(viteConfigPath, 'utf-8');
      console.log('  Patched vite config (first 50 lines):');
      console.log(patched.split('\n').slice(0, 50).map(l => '    ' + l).join('\n'));
    }

    console.log('\n→ Building with OXC React Compiler...');
    oxcResult = buildProject(
      projectDir, project.viteConfigDir, project.buildCmd, project.distDir, 'OXC'
    );

    // Step 6: Read OXC plugin stats
    const statsPath = join(projectDir, project.viteConfigDir, '_oxc-stats.json');
    if (existsSync(statsPath)) {
      try {
        oxcStats = JSON.parse(readFileSync(statsPath, 'utf-8'));
        console.log(`  OXC transform stats: ${oxcStats.compiled} compiled, ${oxcStats.skipped} skipped, ${oxcStats.errors} errors`);
      } catch {}
      rmSync(statsPath, { force: true });
    }

    // Step 7: Compare outputs
    const oxcDistDir = join(projectDir, project.distDir);
    if (existsSync(babelDistBackup) && existsSync(oxcDistDir)) {
      distComparison = compareDists(babelDistBackup, oxcDistDir);
    }
  } catch (err) {
    console.error(`  ⚠ OXC build pipeline error: ${err.message}`);
    oxcResult = { label: 'OXC', times: [], successCount: 0, medianMs: null, dist: { bytes: 0, files: 0 } };
  } finally {
    // Restore original config
    restoreViteConfig(projectDir, project.viteConfigDir, project.viteConfigFile);
    unlinkOxcBinding(projectDir, project.viteConfigDir);
    rmSync(babelDistBackup, { recursive: true, force: true });
    // Clean up stats file
    const statsClean = join(projectDir, project.viteConfigDir, '_oxc-stats.json');
    rmSync(statsClean, { force: true });
  }

  return {
    project: project.name,
    scale: project.scale,
    stars: project.stars,
    reactFiles: reactFileCount,
    viteConfig: viteConfig.slice(0, 200) + '...',
    babel: {
      medianMs: babelResult.medianMs,
      minMs: babelResult.minMs,
      successCount: babelResult.successCount,
      dist: babelResult.dist,
    },
    oxc: {
      medianMs: oxcResult?.medianMs,
      minMs: oxcResult?.minMs,
      successCount: oxcResult?.successCount,
      dist: oxcResult?.dist,
      stats: oxcStats,
    },
    speedup: babelResult.medianMs && oxcResult?.medianMs
      ? parseFloat((babelResult.medianMs / oxcResult.medianMs).toFixed(2))
      : null,
    distComparison,
  };
}

// ============================================================
// Run
// ============================================================
console.log('# E2E Vite Build Benchmark: OXC vs Babel React Compiler\n');
console.log(`Date: ${new Date().toISOString()}`);
console.log(`Platform: ${process.platform} ${process.arch}, Node ${process.version}`);
console.log(`Build iterations: ${buildIterations}\n`);

// Filter projects
let projects = PROJECTS;
if (projectFilter) {
  projects = projects.filter((p) => p.name === projectFilter);
  if (projects.length === 0) {
    console.error(`Unknown project: ${projectFilter}`);
    console.error(`Available: ${PROJECTS.map((p) => p.name).join(', ')}`);
    process.exit(1);
  }
}

const results = [];
for (const project of projects) {
  try {
    const result = await benchmarkProject(project);
    results.push(result);
  } catch (err) {
    console.error(`\n✗ Project ${project.name} failed: ${err.message}`);
    results.push({
      project: project.name,
      scale: project.scale,
      error: err.message,
    });
  }
}

// ============================================================
// Report
// ============================================================
console.log('\n' + '='.repeat(60));
console.log('RESULTS SUMMARY');
console.log('='.repeat(60) + '\n');

if (format === 'json') {
  mkdirSync(REPORT_DIR, { recursive: true });
  const reportPath = join(REPORT_DIR, `e2e-report-${Date.now()}.json`);
  writeFileSync(reportPath, JSON.stringify({
    timestamp: new Date().toISOString(),
    config: { buildIterations },
    results,
  }, null, 2));
  console.log(`JSON report: ${reportPath}`);
}

// Markdown summary
console.log('### Build Time Comparison\n');
console.log('| Project | Scale | React Files | Babel Build | OXC Build | Speedup |');
console.log('|---------|-------|-------------|-------------|-----------|---------|');

for (const r of results) {
  if (r.error) {
    console.log(`| ${r.project} | ${r.scale} | — | ERROR | ERROR | — |`);
    continue;
  }
  const babelTime = r.babel.medianMs != null ? fmtMs(r.babel.medianMs) : 'FAIL';
  const oxcTime = r.oxc.medianMs != null ? fmtMs(r.oxc.medianMs) : 'FAIL';
  const speedup = r.speedup != null ? `**${r.speedup}x**` : '—';
  console.log(`| ${r.project} | ${r.scale} | ${r.reactFiles} | ${babelTime} | ${oxcTime} | ${speedup} |`);
}

console.log('\n### Bundle Size Comparison\n');
console.log('| Project | Babel JS Size | OXC JS Size | Delta | Delta % |');
console.log('|---------|--------------|-------------|-------|---------|');

for (const r of results) {
  if (r.error || !r.distComparison) {
    console.log(`| ${r.project} | — | — | — | — |`);
    continue;
  }
  const dc = r.distComparison;
  const delta = dc.jsSizeDelta >= 0 ? '+' + fmtBytes(dc.jsSizeDelta) : '-' + fmtBytes(Math.abs(dc.jsSizeDelta));
  console.log(`| ${r.project} | ${fmtBytes(dc.babel.jsBytes)} | ${fmtBytes(dc.oxc.jsBytes)} | ${delta} | ${dc.jsSizePct}% |`);
}

console.log('\n### Build Success Rate\n');
console.log('| Project | Babel (success/total) | OXC (success/total) |');
console.log('|---------|----------------------|---------------------|');

for (const r of results) {
  if (r.error) continue;
  console.log(`| ${r.project} | ${r.babel.successCount}/${buildIterations} | ${r.oxc.successCount}/${buildIterations} |`);
}

console.log('\n### OXC Transform Coverage\n');
console.log('| Project | React Files | Compiled | Skipped | Errors | Coverage |');
console.log('|---------|------------|----------|---------|--------|----------|');

for (const r of results) {
  if (r.error || !r.oxc.stats) continue;
  const s = r.oxc.stats;
  const total = s.compiled + s.errors;
  const coverage = total > 0 ? ((s.compiled / total) * 100).toFixed(0) + '%' : '—';
  console.log(`| ${r.project} | ${r.reactFiles} | ${s.compiled} | ${s.skipped} | ${s.errors} | ${coverage} |`);
}
