/**
 * Configuration options for the OXC React Compiler Vite plugin.
 */
export interface ReactCompilerOptions {
  /**
   * How the compiler finds functions to compile.
   * - 'infer': Compile components and hooks (default)
   * - 'all': Compile everything
   * - 'syntax': Only compile with "use memo" directive
   * - 'annotation': Only compile with annotations
   */
  compilationMode?: 'infer' | 'all' | 'syntax' | 'annotation';

  /**
   * Output mode for the compiler.
   * - 'client': Normal client compilation (default)
   * - 'ssr': Server-side rendering mode
   * - 'lint': Lint-only mode (no code transformation)
   */
  outputMode?: 'client' | 'ssr' | 'lint';

  /**
   * Target React version.
   * @default 'react19'
   */
  target?: 'react17' | 'react18' | 'react19';

  /**
   * Glob patterns for files to include.
   */
  include?: string[];

  /**
   * Glob patterns for files to exclude.
   */
  exclude?: string[];

  /**
   * Enable source map generation.
   * Defaults to true in dev mode, false in production.
   */
  sourceMap?: boolean;

  /**
   * Gating configuration for feature flag wrapping.
   */
  gating?: {
    importSource: string;
    functionName: string;
  };

  /**
   * Directory to persist the transform cache across builds.
   * When set, the cache is written to `<cacheDir>/oxc-react-compiler-cache.json`
   * and reloaded on the next build, skipping re-compilation of unchanged files.
   * Leave unset (default) to use only the in-memory cache for the current build.
   */
  cacheDir?: string;
}
