# Code Generation

> ReactiveFunction to JavaScript output, import insertion, and source maps.
> Upstream: `src/ReactiveScopes/CodegenReactiveFunction.ts`, `src/Entrypoint/Imports.ts`
> Rust modules: `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`

---

### Gap 1: CodegenFunction

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Pipeline position:** Pass #62, Phase 11
**Current state:** `reactive_scopes/codegen.rs` is a stub.
**What's needed:**

Convert the tree-shaped ReactiveFunction into JavaScript output. This is one of the ~5 files that must be reimplemented for OXC (not a 1:1 port).

**Edit-based approach** (recommended first implementation):
- Generate the compiled function body as a JavaScript string
- Replace the original function body in the source with the compiled output
- Preserves formatting, comments, and non-compiled code

Core codegen patterns:

1. **Cache allocation**: `const $ = _c(N)` where N is total cache slots needed
   - Count: sum of (dependencies + declarations) across all scopes

2. **Scope blocks**: Each `ReactiveScopeBlock` becomes:
   ```javascript
   if ($[0] !== dep0 || $[1] !== dep1) {
       // recomputed instructions
       $[0] = dep0;
       $[1] = dep1;
       $[2] = computedValue;
   }
   const result = $[2];
   ```

3. **Constant scopes** (no dependencies): use sentinel check:
   ```javascript
   if ($[0] === Symbol.for('react.memo_cache_sentinel')) {
       $[0] = computedValue;
   }
   const result = $[0];
   ```

4. **Control flow**: `ReactiveTerminal` variants emit corresponding JS:
   - `If` -> `if (test) { ... } else { ... }`
   - `Switch` -> `switch (test) { case ...: ... }`
   - `For/While/DoWhile` -> loop constructs
   - `Try` -> `try { ... } catch { ... }`
   - `Return` -> `return value;`

5. **Instructions**: Each instruction becomes a JS expression/statement:
   - `BinaryExpression` -> `left op right`
   - `CallExpression` -> `callee(args)`
   - `PropertyLoad` -> `object.property`
   - `JsxExpression` -> `<Tag props>{children}</Tag>`
   - etc.

6. **Early returns in scopes**: Special handling for `ReactiveScope.early_return_value`

7. **Gating**: If `GatingConfig` is set, wrap output in feature flag check

**Depends on:** All RF optimization passes (Gap 25 in reactive-scopes.md)

---

### Gap 2: Import Insertion

**Upstream:** `src/Entrypoint/Imports.ts`
**Current state:** Nothing implemented.
**What's needed:**

Insert the compiler runtime import at the top of the file:

- `import { c as _c } from 'react/compiler-runtime'`
- Only insert if at least one function was compiled
- Handle existing imports (don't duplicate)
- Target-specific import:
  - React 19: `react/compiler-runtime`
  - React 17/18: custom runtime package
- Handle ESM vs CJS module format
- If using gating, also import the gating function

Implementation approach:
- Track whether any compilation happened during the file transform
- If yes, prepend the import statement to the output
- Use text insertion at the top of the file (edit-based approach)

**Depends on:** Gap 1 (codegen must run first to know if anything was compiled)

---

### Gap 3: Source Map Generation

**Upstream:** Not directly ported (Babel handles source maps differently)
**Current state:** Nothing implemented.
**What's needed:**

Generate source maps for the edit-based codegen:

- Track character offsets of all edits (original span -> compiled span)
- Use `oxc_sourcemap` to build the mapping
- Compose with any existing source maps (e.g., if the file was already transformed by TypeScript)
- Map compiled code back to original source locations
- Handle the `_c()` import insertion offset

This is lower priority than getting correct output. Can be deferred.

**Depends on:** Gap 1, Gap 2
