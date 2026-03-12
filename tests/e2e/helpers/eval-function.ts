import vm from 'vm';
import React from 'react';
import { transformSync } from 'esbuild';

/**
 * Shared runtime utilities matching upstream's shared-runtime.ts.
 * Provides mutate, identity, makeObject, throwInput, etc.
 */
export const sharedRuntime = {
  mutate(obj: Record<string, unknown>): Record<string, unknown> {
    obj.mutated = true;
    return obj;
  },

  identity<T>(x: T): T {
    return x;
  },

  makeObject(props: Record<string, unknown> = {}): Record<string, unknown> {
    return { ...props };
  },

  throwInput(x: unknown): never {
    throw x;
  },

  arrayPush<T>(arr: T[], item: T): T[] {
    arr.push(item);
    return arr;
  },

  setProperty(
    obj: Record<string, unknown>,
    key: string,
    value: unknown
  ): Record<string, unknown> {
    obj[key] = value;
    return obj;
  },
};

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

interface EvalResult {
  returnValue: unknown;
  logs: string[];
  error: string | null;
}

/**
 * Evaluate a source string as a function module, call the exported function
 * with the given args, and capture the return value, console output, and errors.
 *
 * The source should define a function (named or default export) that we'll call.
 */
export function evalFunction(
  source: string,
  args: unknown[] = [],
  functionName?: string
): EvalResult {
  const logs: string[] = [];
  const exports: Record<string, unknown> = {};
  const moduleObj = { exports };

  // Mock compiler runtime
  const compilerRuntime = {
    c: (size: number) => {
      return new Array(size).fill(Symbol.for('react.memo_cache_sentinel'));
    },
  };

  const mockConsole = {
    log: (...a: unknown[]) => logs.push(a.map(String).join(' ')),
    warn: (...a: unknown[]) => logs.push('[warn] ' + a.map(String).join(' ')),
    error: (...a: unknown[]) => logs.push('[error] ' + a.map(String).join(' ')),
  };

  const sandbox: Record<string, unknown> = {
    React,
    require: (mod: string) => {
      if (mod === 'react') return React;
      if (mod === 'react/jsx-runtime') return require('react/jsx-runtime');
      if (mod === 'react/compiler-runtime') return compilerRuntime;
      throw new Error(`Unsupported require: ${mod}`);
    },
    module: moduleObj,
    exports,
    console: mockConsole,
    Symbol,
    Object,
    Array,
    JSON,
    Math,
    parseInt,
    parseFloat,
    isNaN,
    isFinite,
    undefined,
    NaN,
    Infinity,
    ...sharedRuntime,
  };

  let code = stripJsx(source);

  // Add function export detection
  const funcMatch = source.match(/(?:export\s+(?:default\s+)?)?function\s+(\w+)/);
  const targetName = functionName || (funcMatch ? funcMatch[1] : null);

  if (targetName) {
    code += `\nmodule.exports.__fn = ${targetName};`;
  }

  let returnValue: unknown = undefined;
  let error: string | null = null;

  try {
    vm.runInNewContext(code, sandbox, {
      filename: 'eval-function.js',
      timeout: 5000,
    });

    const fn = moduleObj.exports.__fn as ((...a: unknown[]) => unknown) | undefined;
    if (!fn || typeof fn !== 'function') {
      error = `No function found to call. Exports: ${Object.keys(moduleObj.exports).join(', ')}`;
    } else {
      returnValue = fn(...args);
    }
  } catch (err) {
    error = String(err);
  }

  return { returnValue, logs, error };
}

/**
 * Compare the evaluation results of original vs compiled source.
 */
export function compareFunctionEvals(
  originalSource: string,
  compiledSource: string,
  args: unknown[] = [],
  functionName?: string
): {
  original: EvalResult;
  compiled: EvalResult;
  returnMatch: boolean;
  logsMatch: boolean;
  bothSucceed: boolean;
} {
  const original = evalFunction(originalSource, args, functionName);
  const compiled = evalFunction(compiledSource, args, functionName);

  const returnMatch =
    JSON.stringify(original.returnValue) === JSON.stringify(compiled.returnValue);
  const logsMatch =
    JSON.stringify(original.logs) === JSON.stringify(compiled.logs);
  const bothSucceed = original.error === null && compiled.error === null;

  return { original, compiled, returnMatch, logsMatch, bothSucceed };
}
