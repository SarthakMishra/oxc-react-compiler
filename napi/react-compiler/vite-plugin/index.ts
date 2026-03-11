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
}

export function reactCompiler(options: ReactCompilerOptions = {}): any {
  // Dynamic import of the native binding
  let binding: any;

  return {
    name: 'oxc-react-compiler',
    enforce: 'pre' as const,

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
        });

        if (!result.transformed) return null;

        return {
          code: result.code,
          map: null, // TODO: source map support
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
