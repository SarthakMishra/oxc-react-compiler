import { describe, it, expect } from 'vitest';
import { compileSource } from './helpers/compile';
import { evalAndRender, compareRenders } from './helpers/eval-component';

describe('list rendering', () => {
  describe('original source evaluation (baseline)', () => {
    it('renders array.map', () => {
      const source = `
function ItemList({ items }) {
  return (
    <ul>
      {items.map(function(item) { return <li key={item}>{item}</li>; })}
    </ul>
  );
}
`;
      const html = evalAndRender(source, { items: ['apple', 'banana', 'cherry'] });
      expect(html).toContain('<li>apple</li>');
      expect(html).toContain('<li>banana</li>');
      expect(html).toContain('<li>cherry</li>');
    });

    it('renders empty array', () => {
      const source = `
function ItemList({ items }) {
  return (
    <ul>
      {items.map(function(item) { return <li key={item}>{item}</li>; })}
    </ul>
  );
}
`;
      const html = evalAndRender(source, { items: [] });
      expect(html).toBe('<ul></ul>');
    });
  });

  describe('compilation produces output', () => {
    it('transforms list component', () => {
      const compiled = compileSource(`
function ItemList({ items }) {
  return (
    <ul>
      {items.map(function(item) { return <li key={item}>{item}</li>; })}
    </ul>
  );
}
`);
      expect(compiled).not.toBeNull();
    });
  });

  describe('dual-mode comparison', () => {
    it('list rendering matches original', () => {
      const source = `
function ItemList({ items }) {
  return (
    <ul>
      {items.map(function(item) { return <li key={item}>{item}</li>; })}
    </ul>
  );
}
`;
      const compiled = compileSource(source)!;
      const result = compareRenders(source, compiled, {
        items: ['apple', 'banana', 'cherry'],
      });
      expect(result.match).toBe(true);
    });
  });
});
