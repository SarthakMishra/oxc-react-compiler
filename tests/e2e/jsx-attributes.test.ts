import { describe, it, expect } from 'vitest';
import { compileSource } from './helpers/compile';
import { evalAndRender, compareRenders } from './helpers/eval-component';

describe('JSX attributes', () => {
  describe('original source evaluation (baseline)', () => {
    it('renders className prop', () => {
      const source = `
function StyledBox({ className }) {
  return <div className={className}>content</div>;
}
`;
      const html = evalAndRender(source, { className: 'box-primary' });
      expect(html).toBe('<div class="box-primary">content</div>');
    });

    it('renders multiple attributes', () => {
      const source = `
function Link({ href, title }) {
  return <a href={href} title={title}>Click me</a>;
}
`;
      const html = evalAndRender(source, { href: 'https://example.com', title: 'Example' });
      expect(html).toContain('href="https://example.com"');
      expect(html).toContain('title="Example"');
    });
  });

  describe('compilation produces output', () => {
    it('transforms component with className', () => {
      const compiled = compileSource(`
function StyledBox({ className }) {
  return <div className={className}>content</div>;
}
`);
      expect(compiled).not.toBeNull();
    });

    it('transforms component with multiple attrs', () => {
      const compiled = compileSource(`
function Link({ href, title }) {
  return <a href={href} title={title}>Click me</a>;
}
`);
      expect(compiled).not.toBeNull();
    });
  });

  describe('dual-mode comparison', () => {
    it('className renders identically', () => {
      const source = `
function StyledBox({ className }) {
  return <div className={className}>content</div>;
}
`;
      const compiled = compileSource(source)!;
      const result = compareRenders(source, compiled, { className: 'box-primary' });
      expect(result.match).toBe(true);
    });
  });
});
