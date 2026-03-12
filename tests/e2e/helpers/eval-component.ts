import vm from 'vm';
import React from 'react';
import ReactDOMServer from 'react-dom/server';
import { transformSync } from 'esbuild';

/**
 * Transform JSX to plain JavaScript using esbuild.
 */
function stripJsx(source: string): string {
  const result = transformSync(source, {
    loader: 'tsx',
    jsx: 'automatic',
    jsxImportSource: 'react',
    format: 'cjs',
    target: 'es2020',
  });
  return result.code;
}

/**
 * Evaluate a source string that exports a React component,
 * then render it to static HTML for comparison.
 *
 * JSX is transformed via esbuild before evaluation.
 * Uses vm.runInNewContext to sandbox evaluation.
 */
export function evalAndRender(
  source: string,
  props: Record<string, unknown> = {}
): string {
  const exports: Record<string, unknown> = {};
  const moduleObj = { exports };

  // Create a mock for react/compiler-runtime's `c` function
  const compilerRuntime = {
    c: (size: number) => {
      return new Array(size).fill(Symbol.for('react.memo_cache_sentinel'));
    },
  };

  const sandbox: Record<string, unknown> = {
    React,
    require: (mod: string) => {
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
  };

  // Transform JSX and imports to plain JS
  let code = stripJsx(source);

  // Append export capture code based on original source
  code += '\n' + getFunctionExportCode(source);

  try {
    vm.runInNewContext(code, sandbox, {
      filename: 'eval-component.js',
      timeout: 5000,
    });
  } catch (err) {
    throw new Error(`Failed to evaluate source:\n${err}\n\nTransformed code:\n${code}`);
  }

  // Get the component from exports
  const Component = (moduleObj.exports.default ||
    moduleObj.exports.Component ||
    Object.values(moduleObj.exports).find(
      (v) => typeof v === 'function'
    )) as React.FC<Record<string, unknown>> | undefined;

  if (!Component || typeof Component !== 'function') {
    throw new Error(
      `No component found in exports. Available: ${Object.keys(moduleObj.exports).join(', ')}`
    );
  }

  // Render to static HTML
  return ReactDOMServer.renderToStaticMarkup(
    React.createElement(Component, props)
  );
}

/**
 * Extract function names from source and generate export assignment code.
 */
function getFunctionExportCode(source: string): string {
  // Match: export default function Name
  const exportDefault = source.match(/export\s+default\s+function\s+(\w+)/);
  if (exportDefault) {
    return `module.exports.default = ${exportDefault[1]};`;
  }

  // Match: export function Name
  const exportNamed = source.match(/export\s+function\s+(\w+)/);
  if (exportNamed) {
    return `module.exports.Component = ${exportNamed[1]};`;
  }

  // Match: function Name (capitalized = component)
  const funcDecls = [...source.matchAll(/(?:^|\n)\s*function\s+([A-Z]\w*)/g)];
  if (funcDecls.length > 0) {
    return `module.exports.Component = ${funcDecls[0][1]};`;
  }

  // Match: const Name = ...
  const constDecls = [...source.matchAll(/(?:^|\n)\s*const\s+([A-Z]\w*)\s*=/g)];
  if (constDecls.length > 0) {
    return `module.exports.Component = ${constDecls[0][1]};`;
  }

  return '// no component found to export';
}

/**
 * Compare the rendered HTML output of original vs compiled source.
 * Returns { original, compiled, match }.
 */
export function compareRenders(
  originalSource: string,
  compiledSource: string,
  props: Record<string, unknown> = {}
): { original: string; compiled: string; match: boolean } {
  const original = evalAndRender(originalSource, props);
  const compiled = evalAndRender(compiledSource, props);
  return {
    original,
    compiled,
    match: original === compiled,
  };
}
