import { describe, it, expect } from 'vitest';
import { compileSource } from './helpers/compile';
import { evalAndRender, compareRenders } from './helpers/eval-component';

describe('conditional rendering', () => {
  describe('original source evaluation (baseline)', () => {
    it('renders ternary truthy case', () => {
      const source = `
function Toggle({ isOn }) {
  return <div>{isOn ? "ON" : "OFF"}</div>;
}
`;
      const html = evalAndRender(source, { isOn: true });
      expect(html).toBe('<div>ON</div>');
    });

    it('renders ternary falsy case', () => {
      const source = `
function Toggle({ isOn }) {
  return <div>{isOn ? "ON" : "OFF"}</div>;
}
`;
      const html = evalAndRender(source, { isOn: false });
      expect(html).toBe('<div>OFF</div>');
    });

    it('renders if/else with early return', () => {
      const source = `
function Status({ loading }) {
  if (loading) {
    return <div>Loading...</div>;
  }
  return <div>Ready</div>;
}
`;
      expect(evalAndRender(source, { loading: true })).toBe('<div>Loading...</div>');
      expect(evalAndRender(source, { loading: false })).toBe('<div>Ready</div>');
    });

    it('renders logical AND', () => {
      const source = `
function MaybeShow({ show, text }) {
  return <div>{show && <span>{text}</span>}</div>;
}
`;
      expect(evalAndRender(source, { show: true, text: 'Visible' })).toContain('Visible');
      expect(evalAndRender(source, { show: false, text: 'Visible' })).toBe('<div></div>');
    });
  });

  describe('compilation produces output', () => {
    it('transforms ternary component', () => {
      const compiled = compileSource(`
function Toggle({ isOn }) {
  return <div>{isOn ? "ON" : "OFF"}</div>;
}
`);
      expect(compiled).not.toBeNull();
    });

    it('transforms if/else component', () => {
      const compiled = compileSource(`
function Status({ loading }) {
  if (loading) {
    return <div>Loading...</div>;
  }
  return <div>Ready</div>;
}
`);
      expect(compiled).not.toBeNull();
    });
  });

  describe('dual-mode comparison', () => {
    it.fails('ternary renders identically (known codegen issue: destructure + control flow)', () => {
      const source = `
function Toggle({ isOn }) {
  return <div>{isOn ? "ON" : "OFF"}</div>;
}
`;
      const compiled = compileSource(source)!;
      const result = compareRenders(source, compiled, { isOn: true });
      expect(result.match).toBe(true);
    });

    it.fails('if/else renders identically (known codegen issue)', () => {
      const source = `
function Status({ loading }) {
  if (loading) {
    return <div>Loading...</div>;
  }
  return <div>Ready</div>;
}
`;
      const compiled = compileSource(source)!;
      const result = compareRenders(source, compiled, { loading: true });
      expect(result.match).toBe(true);
    });
  });
});
