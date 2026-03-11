/**
 * OXC React Compiler Vite Plugin
 *
 * Usage:
 *   import { reactCompiler } from '@oxc-react/vite';
 *
 *   export default defineConfig({
 *     plugins: [reactCompiler()],
 *   });
 */

interface ReactCompilerOptions {
  compilationMode?: 'infer' | 'all' | 'syntax' | 'annotation';
  outputMode?: 'client' | 'ssr' | 'lint';
  target?: 'react17' | 'react18' | 'react19';
  include?: string[];
  exclude?: string[];
  /** Enable source map generation (default: true in dev, false in build) */
  sourceMap?: boolean;
}

export function reactCompiler(options: ReactCompilerOptions = {}): any {
  // Dynamic import of the native binding
  let binding: any;
  let enableSourceMap = options.sourceMap;

  return {
    name: 'oxc-react-compiler',
    enforce: 'pre' as const,

    configResolved(config: any) {
      // Default: enable source maps in dev mode, disable in production build
      if (enableSourceMap === undefined) {
        enableSourceMap = config.command === 'serve';
      }
    },

    async buildStart() {
      try {
        // @ts-ignore - native binding generated at build time
        binding = await import('../index.js');
      } catch (e) {
        console.warn('[oxc-react-compiler] Failed to load native binding:', e);
      }
    },

    transform(code: string, id: string) {
      // Only process React-relevant files
      if (!isReactFile(id)) return null;

      // Quick check: skip files without React patterns
      if (!mightContainReactCode(code)) return null;

      if (!binding) return null;

      try {
        const result = binding.transformReactFile(code, id, {
          compilationMode: options.compilationMode,
          outputMode: options.outputMode,
          sourceMap: enableSourceMap,
        });

        if (!result.transformed) return null;

        // Parse source map JSON if available, Vite accepts object or string
        const map = result.sourceMap ? JSON.parse(result.sourceMap) : null;

        return {
          code: result.code,
          map,
        };
      } catch (e) {
        console.error(`[oxc-react-compiler] Error transforming ${id}:`, e);
        return null;
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
