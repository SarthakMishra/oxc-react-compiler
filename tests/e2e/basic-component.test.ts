import { describe, it, expect } from 'vitest';
import { compileSource } from './helpers/compile';
import { evalAndRender, compareRenders } from './helpers/eval-component';

describe('basic component rendering', () => {
  describe('original source evaluation (baseline)', () => {
    it('can render a simple component from source', () => {
      const source = `
function Greeting({ name }) {
  return <div>Hello, {name}!</div>;
}
`;
      const html = evalAndRender(source, { name: 'World' });
      expect(html).toBe('<div>Hello, World!</div>');
    });

    it('can render a static component', () => {
      const source = `
function StaticMessage() {
  return <div>Hello, static world!</div>;
}
`;
      const html = evalAndRender(source, {});
      expect(html).toBe('<div>Hello, static world!</div>');
    });

    it('can render nested elements', () => {
      const source = `
function Card({ title, body }) {
  return (
    <div>
      <h1>{title}</h1>
      <p>{body}</p>
    </div>
  );
}
`;
      const html = evalAndRender(source, { title: 'Title', body: 'Body' });
      expect(html).toContain('<h1>Title</h1>');
      expect(html).toContain('<p>Body</p>');
    });
  });

  describe('compilation produces output', () => {
    it('transforms a simple component', () => {
      const compiled = compileSource(`
function Greeting({ name }) {
  return <div>Hello, {name}!</div>;
}
`);
      expect(compiled).not.toBeNull();
      expect(compiled).toContain('function');
    });

    it('transforms a static component', () => {
      const compiled = compileSource(`
function StaticMessage() {
  return <div>Hello!</div>;
}
`);
      expect(compiled).not.toBeNull();
    });

    it('transforms a component with derived values', () => {
      const compiled = compileSource(`
function Card({ title, body }) {
  return (
    <div>
      <h1>{title}</h1>
      <p>{body}</p>
    </div>
  );
}
`);
      expect(compiled).not.toBeNull();
    });
  });

  describe('dual-mode comparison', () => {
    it('simple props passthrough renders identically', () => {
      const source = `
function Greeting({ name }) {
  return <div>Hello, {name}!</div>;
}
`;
      const compiled = compileSource(source)!;
      const result = compareRenders(source, compiled, { name: 'World' });
      expect(result.match).toBe(true);
    });

    it('static component renders identically', () => {
      const source = `
function StaticMessage() {
  return <div>Hello!</div>;
}
`;
      const compiled = compileSource(source)!;
      const result = compareRenders(source, compiled, {});
      expect(result.match).toBe(true);
    });
  });
});
