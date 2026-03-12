/**
 * OXC React Compiler Vite Plugin
 *
 * Usage:
 *   import { reactCompiler } from 'oxc-react-compiler/vite';
 *
 *   export default defineConfig({
 *     plugins: [reactCompiler()],
 *   });
 */

import { createHash } from 'node:crypto';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import type { ReactCompilerOptions } from './options';

export type { ReactCompilerOptions };

interface CacheEntry {
  contentHash: string;
  code: string;
  map: object | null;
}

const CACHE_VERSION = 1;
const CACHE_FILE_NAME = 'oxc-react-compiler-cache.json';

interface DiskCache {
  version: number;
  optionsHash: string;
  entries: Record<string, CacheEntry>;
}

export function reactCompiler(options: ReactCompilerOptions = {}): any {
  // Dynamic import of the native binding
  let binding: any;
  let enableSourceMap = options.sourceMap;
  let optionsHash: string;

  // In-memory cache: file ID → cached transform result
  const cache = new Map<string, CacheEntry>();

  function computeOptionsHash(): string {
    const h = createHash('md5');
    h.update(JSON.stringify({
      compilationMode: options.compilationMode,
      outputMode: options.outputMode,
      sourceMap: enableSourceMap,
      gating: options.gating,
    }));
    return h.digest('hex');
  }

  function contentHash(code: string): string {
    return createHash('md5').update(code).digest('hex');
  }

  return {
    name: 'oxc-react-compiler',
    enforce: 'pre' as const,

    configResolved(config: any) {
      // Default: enable source maps in dev mode, disable in production build
      if (enableSourceMap === undefined) {
        enableSourceMap = config.command === 'serve';
      }
      optionsHash = computeOptionsHash();
    },

    async buildStart() {
      try {
        // @ts-ignore - native binding generated at build time
        binding = await import('../index.js');
      } catch (e) {
        console.warn('[oxc-react-compiler] Failed to load native binding:', e);
      }

      // Recompute options hash and clear cache if options changed
      const newHash = computeOptionsHash();
      if (optionsHash && newHash !== optionsHash) {
        cache.clear();
      }
      optionsHash = newHash;

      // Load disk cache if cacheDir is configured
      if (options.cacheDir) {
        const cacheFile = join(options.cacheDir, CACHE_FILE_NAME);
        try {
          const raw = readFileSync(cacheFile, 'utf8');
          const disk: DiskCache = JSON.parse(raw);
          if (disk.version === CACHE_VERSION && disk.optionsHash === optionsHash) {
            for (const [id, entry] of Object.entries(disk.entries)) {
              cache.set(id, entry);
            }
          }
        } catch {
          // File missing or unreadable — start with empty cache
        }
      }
    },

    transform(code: string, id: string) {
      // Only process React-relevant files
      if (!isReactFile(id)) return null;

      // Quick check: skip files without React patterns
      if (!mightContainReactCode(code)) return null;

      if (!binding) return null;

      // Check cache
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
          gatingImportSource: options.gating?.importSource,
          gatingFunctionName: options.gating?.functionName,
        });

        if (!result.transformed) {
          // Cache the "not transformed" result too
          cache.set(id, { contentHash: hash, code: '', map: null });
          return null;
        }

        // Parse source map JSON if available, Vite accepts object or string
        const map = result.sourceMap ? JSON.parse(result.sourceMap) : null;

        cache.set(id, { contentHash: hash, code: result.code, map });

        return {
          code: result.code,
          map,
        };
      } catch (e) {
        console.error(`[oxc-react-compiler] Error transforming ${id}:`, e);
        return null;
      }
    },

    handleHotUpdate({ file, server }: { file: string; server: any }) {
      if (!isReactFile(file)) return;

      // Evict cache entry so next transform recompiles
      cache.delete(file);

      // Invalidate the module to force re-transform with the compiler.
      const mod = server.moduleGraph.getModuleById(file);
      if (mod) {
        server.moduleGraph.invalidateModule(mod);
      }
    },

    closeBundle() {
      if (!options.cacheDir || cache.size === 0) return;

      const entries: Record<string, CacheEntry> = {};
      for (const [id, entry] of cache) {
        entries[id] = entry;
      }

      const disk: DiskCache = {
        version: CACHE_VERSION,
        optionsHash,
        entries,
      };

      try {
        mkdirSync(options.cacheDir, { recursive: true });
        writeFileSync(
          join(options.cacheDir, CACHE_FILE_NAME),
          JSON.stringify(disk),
          'utf8',
        );
      } catch (e) {
        console.warn('[oxc-react-compiler] Failed to write disk cache:', e);
      }
    },
  };
}

function isReactFile(id: string): boolean {
  // Filter to .tsx, .jsx, .ts, .js files
  return /\.(tsx?|jsx?)$/.test(id) && !id.includes('node_modules');
}

function mightContainReactCode(code: string): boolean {
  // Quick heuristic check for React patterns
  return (
    code.includes('function') ||
    code.includes('=>') ||
    code.includes('use')
  ) && (
    code.includes('return') ||
    code.includes('jsx') ||
    code.includes('JSX') ||
    code.includes('<')
  );
}

export default reactCompiler;
