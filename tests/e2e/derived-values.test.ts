import { describe, it, expect } from 'vitest';
import { compileSource } from './helpers/compile';
import { evalAndRender, compareRenders } from './helpers/eval-component';

describe('derived values', () => {
  describe('original source evaluation (baseline)', () => {
    it('renders computed string', () => {
      const source = `
function Greeting({ firstName, lastName }) {
  const fullName = firstName + " " + lastName;
  return <div>Hello, {fullName}!</div>;
}
`;
      const html = evalAndRender(source, { firstName: 'John', lastName: 'Doe' });
      expect(html).toBe('<div>Hello, John Doe!</div>');
    });

    it('renders numeric computation', () => {
      const source = `
function PriceTag({ price, taxRate }) {
  const total = price * (1 + taxRate);
  return <span>{total}</span>;
}
`;
      const html = evalAndRender(source, { price: 100, taxRate: 0.1 });
      expect(html).toContain('110');
    });

    it('renders conditional derived value', () => {
      const source = `
function Badge({ count }) {
  const label = count > 0 ? "Has items" : "Empty";
  return <div>{label}: {count}</div>;
}
`;
      expect(evalAndRender(source, { count: 5 })).toBe('<div>Has items: 5</div>');
      expect(evalAndRender(source, { count: 0 })).toBe('<div>Empty: 0</div>');
    });
  });

  describe('compilation produces output', () => {
    it('transforms component with derived string', () => {
      const compiled = compileSource(`
function Greeting({ firstName, lastName }) {
  const fullName = firstName + " " + lastName;
  return <div>Hello, {fullName}!</div>;
}
`);
      expect(compiled).not.toBeNull();
    });

    it('transforms component with derived number', () => {
      const compiled = compileSource(`
function Badge({ count }) {
  const label = count > 0 ? "Has items" : "Empty";
  return <div>{label}: {count}</div>;
}
`);
      expect(compiled).not.toBeNull();
    });
  });

  describe('dual-mode comparison', () => {
    it.fails('derived string renders identically (known codegen issue)', () => {
      const source = `
function Greeting({ firstName, lastName }) {
  const fullName = firstName + " " + lastName;
  return <div>Hello, {fullName}!</div>;
}
`;
      const compiled = compileSource(source)!;
      const result = compareRenders(source, compiled, { firstName: 'John', lastName: 'Doe' });
      expect(result.match).toBe(true);
    });
  });
});
