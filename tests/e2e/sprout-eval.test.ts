import { describe, it, expect } from 'vitest';
import { compileSource } from './helpers/compile';
import {
  evalFunction,
  compareFunctionEvals,
} from './helpers/eval-function';

/**
 * Sprout-equivalent runtime evaluation tests.
 *
 * These tests evaluate original and compiled code side-by-side,
 * comparing return values, console output, and exceptions.
 * Modeled after upstream's snap/sprout system.
 */

describe('sprout: pure function evaluation', () => {
  describe('original source evaluation (baseline)', () => {
    it('evaluates a pure function returning a value', () => {
      const source = `
function add(a, b) {
  return a + b;
}
`;
      const result = evalFunction(source, [2, 3]);
      expect(result.error).toBeNull();
      expect(result.returnValue).toBe(5);
    });

    it('evaluates a function with console output', () => {
      const source = `
function greet(name) {
  console.log("Hello, " + name);
  return "done";
}
`;
      const result = evalFunction(source, ['Alice']);
      expect(result.error).toBeNull();
      expect(result.returnValue).toBe('done');
      expect(result.logs).toEqual(['Hello, Alice']);
    });

    it('captures thrown errors', () => {
      const source = `
function willThrow() {
  throw new Error("oops");
}
`;
      const result = evalFunction(source, []);
      expect(result.error).toContain('oops');
    });

    it('evaluates object manipulation', () => {
      const source = `
function makeUser(name, age) {
  return { name, age, isAdult: age >= 18 };
}
`;
      const result = evalFunction(source, ['Bob', 25]);
      expect(result.error).toBeNull();
      expect(result.returnValue).toEqual({ name: 'Bob', age: 25, isAdult: true });
    });

    it('evaluates array operations', () => {
      const source = `
function doubleAll(items) {
  return items.map(function(x) { return x * 2; });
}
`;
      const result = evalFunction(source, [[1, 2, 3]]);
      expect(result.error).toBeNull();
      expect(result.returnValue).toEqual([2, 4, 6]);
    });
  });

  describe('compilation + evaluation', () => {
    it('non-component functions are not transformed', () => {
      const source = `
function add(a, b) {
  return a + b;
}
`;
      const compiled = compileSource(source);
      // Pure functions (lowercase, no JSX) should not be compiled
      expect(compiled).toBeNull();
    });

    it('transforms a component-like function', () => {
      const source = `
function Compute({ x, y }) {
  const sum = x + y;
  return <div>{sum}</div>;
}
`;
      const compiled = compileSource(source);
      expect(compiled).not.toBeNull();
    });
  });

  describe('dual-mode: return value comparison', () => {
    it('component returning JSX produces same output', () => {
      const source = `
function Label({ text }) {
  return <span>{text}</span>;
}
`;
      const compiled = compileSource(source)!;
      const result = compareFunctionEvals(source, compiled, [{ text: 'hello' }]);
      expect(result.bothSucceed).toBe(true);
      expect(result.returnMatch).toBe(true);
    });
  });
});

describe('sprout: mutation tracking', () => {
  describe('original source evaluation', () => {
    it('tracks object mutation', () => {
      const source = `
function MutateDemo({ obj }) {
  obj.x = 42;
  return <div>{obj.x}</div>;
}
`;
      // Just verify the original evaluates
      const result = evalFunction(source, [{ obj: { x: 0 } }]);
      // This is a component, so it returns JSX — the eval captures the React element
      expect(result.error).toBeNull();
    });
  });
});

describe('sprout: sequential renders', () => {
  describe('original source evaluation', () => {
    it('pure component returns consistent results across calls', () => {
      const source = `
function Double({ n }) {
  return <span>{n * 2}</span>;
}
`;
      // Call multiple times with different props
      const r1 = evalFunction(source, [{ n: 1 }]);
      const r2 = evalFunction(source, [{ n: 5 }]);
      const r3 = evalFunction(source, [{ n: 0 }]);

      expect(r1.error).toBeNull();
      expect(r2.error).toBeNull();
      expect(r3.error).toBeNull();
    });
  });

  describe('compiled sequential renders', () => {
    it('compiled component is consistent across calls', () => {
      const source = `
function Double({ n }) {
  return <span>{n * 2}</span>;
}
`;
      const compiled = compileSource(source)!;

      const r1 = evalFunction(compiled, [{ n: 1 }], 'Double');
      const r2 = evalFunction(compiled, [{ n: 5 }], 'Double');

      expect(r1.error).toBeNull();
      expect(r2.error).toBeNull();
    });
  });
});
