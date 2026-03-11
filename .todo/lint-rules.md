# Oxlint Rules

> Lint rules for React compiler diagnostics, split into Tier 1 (standalone) and Tier 2 (compiler-dependent).
> Upstream: `eslint-plugin-react-compiler`
> Rust modules: `crates/oxc_react_compiler_lint/src/rules/*.rs`

---

## Tier 1: Standalone AST Rules

These use `oxc_ast` visitors and `oxc_semantic` only. No compiler dependency.

### Gap 1: no-jsx-in-try

**Upstream:** Validation category `ErrorBoundaries`
**Current state:** `rules/no_jsx_in_try.rs` exists as a stub.
**What's needed:**

- Walk AST for `TryStatement` nodes
- Check if any `JSXElement` or `JSXFragment` is inside the try block body
- Report diagnostic: "JSX expressions inside try/catch blocks are not supported. Use Error Boundaries instead."
- Autofix: none (semantic change required)

**Depends on:** None

---

### Gap 2: use-memo-validation

**Upstream:** Validation categories `UseMemo`, `VoidUseMemo`
**Current state:** No file exists yet (listed in crate structure but not created).
**What's needed:**

- Detect `useMemo(() => { ... })` where callback has no return statement
- Detect `useMemo(async () => ...)` — async callbacks
- Detect `useMemo()` with no callback argument
- Detect `useCallback()` with same issues
- Report appropriate diagnostics

**Depends on:** None

---

### Gap 3: no-capitalized-calls

**Upstream:** Validation category `CapitalizedCalls`
**Current state:** No file exists yet.
**What's needed:**

- Detect `CallExpression` where callee is an identifier starting with uppercase
- Exclude known safe PascalCase calls (e.g., `React.createElement`)
- Report: "Component-like functions should be rendered as JSX, not called directly"

**Depends on:** None

---

### Gap 4: purity

**Upstream:** Validation category `Purity`
**Current state:** No file exists yet (listed in crate structure).
**What's needed:**

- Detect known impure function calls in render: `Math.random()`, `Date.now()`, `crypto.randomUUID()`
- Detect `new Date()` without arguments
- Report: "Impure function call in render. This value will be different every render."

**Depends on:** None

---

### Gap 5: incompatible-library

**Upstream:** Validation category `IncompatibleLibrary`
**Current state:** No file exists yet.
**What's needed:**

- Detect imports from blocklisted libraries that are incompatible with React Compiler
- Configurable blocklist
- Report: "Import from '{library}' is incompatible with React Compiler"

**Depends on:** None

---

### Gap 6: static-components

**Upstream:** Validation category `StaticComponents`
**Current state:** No file exists yet (listed in crate structure).
**What's needed:**

- Detect component definitions inside render functions
- Pattern: `function Parent() { function Child() { ... } return <Child /> }`
- Also arrow function components: `const Child = () => ...` inside render
- Report: "Component '{name}' is defined inside another component. Move it outside to prevent state loss."

**Depends on:** Shared utils (hook_detection.rs for component name detection)

---

### Gap 7: no-set-state-in-render

**Upstream:** Validation category `RenderSetState`
**Current state:** `rules/no_set_state_in_render.rs` exists as a stub.
**What's needed:**

- Detect `useState()` return patterns: `const [state, setState] = useState(...)`
- Track setter functions through the scope
- Detect unconditional calls to setters in the render body (outside event handlers, effects, callbacks)
- Report: "Unconditional setState call in render will cause infinite re-renders"

**Depends on:** `oxc_semantic` for scope analysis

---

### Gap 8: no-set-state-in-effects

**Upstream:** Validation category `EffectSetState`
**Current state:** `rules/no_set_state_in_effects.rs` exists as a stub.
**What's needed:**

- Detect `useEffect(() => { setState(value); })` — synchronous setState in effect callback
- Track setter functions through the scope
- Only flag direct (non-conditional) setState calls in effect body
- Report: "Synchronous setState in useEffect. Consider using a ref or moving logic to an event handler."

**Depends on:** `oxc_semantic` for scope analysis, shared utils for effect hook detection

---

### Gap 9: no-ref-access-in-render

**Upstream:** Validation category `Refs`
**Current state:** `rules/no_ref_access_in_render.rs` exists as a stub.
**What's needed:**

- Detect `useRef()` return values
- Track ref objects through the scope
- Detect `.current` property access on refs during render (outside effects, event handlers)
- Report: "Accessing ref.current during render is not safe"
- Simplified version (no compiler analysis) — catches common patterns

**Depends on:** `oxc_semantic` for scope analysis

---

### Gap 10: no-deriving-state-in-effects

**Upstream:** Validation category `EffectDerivationsOfState`
**Current state:** No file exists yet.
**What's needed:**

- Detect pattern: `useEffect(() => setState(f(dep)), [dep])`
- This is derived state that should be computed with `useMemo` instead
- Track state setters and dependency arrays
- Report: "Derived state in effect. Use useMemo instead of useEffect + setState."

**Depends on:** `oxc_semantic` for scope analysis

---

### Gap 11: globals

**Upstream:** Validation category `Globals`
**Current state:** No file exists yet.
**What's needed:**

- Detect mutation of module-scope variables inside render functions
- Pattern: `let count = 0; function Component() { count++; ... }`
- Track module-scope variable declarations
- Detect writes to them inside component/hook bodies
- Report: "Mutating module-scope variable '{name}' in render is not safe"

**Depends on:** `oxc_semantic` for scope analysis

---

## Tier 2: Compiler-Dependent Rules

These require running the compiler pipeline (in lint mode) and analyzing HIR/effect information.

### Gap 12: hooks (Tier 2)

**Upstream:** Validation category `Hooks`
**Current state:** `rules/rules_of_hooks.rs` exists as a stub (Tier 1 version).
**What's needed:**

Full Rules of Hooks with compiler analysis:

- Conditional hook calls detected via CFG analysis (not just AST patterns)
- Hooks inside loops detected via loop terminals
- Hooks after early returns
- First-class hooks (hooks stored in variables, called dynamically)
- Uses the compiler's HIR and control flow graph for precise analysis

**Depends on:** Compiler core in lint mode (pipeline.md Gap 3)

---

### Gap 13: immutability (Tier 2)

**Upstream:** Validation category `Immutability`
**Current state:** No file exists yet.
**What's needed:**

- Detect mutation of frozen values using compiler's effect analysis
- Props mutation, state mutation, hook return value mutation
- Uses `InferMutationAliasingEffects` results
- Reports precise mutation chains

**Depends on:** Compiler core in lint mode

---

### Gap 14: preserve-manual-memoization (Tier 2)

**Upstream:** Validation category `PreserveManualMemo`
**Current state:** No file exists yet.
**What's needed:**

- Report whether manual useMemo/useCallback would be preserved by the compiler
- Uses `ValidatePreservedManualMemoization` pass results
- Reports if compiler memoization is less stable than manual

**Depends on:** Compiler core in lint mode

---

### Gap 15: memo-dependencies (Tier 2)

**Upstream:** Validation category `MemoDependencies`
**Current state:** No file exists yet.
**What's needed:**

- Report missing/extra dependencies in useMemo/useCallback dep arrays
- Uses compiler's reactive scope dependency analysis
- Provides autofix: rewrite dependency array to match compiler-inferred deps
- More precise than eslint-plugin-react-hooks (uses compiler analysis, not AST heuristics)

**Depends on:** Compiler core in lint mode

---

### Gap 16: exhaustive-effect-deps (Tier 2)

**Upstream:** Validation category `EffectExhaustiveDependencies`
**Current state:** No file exists yet.
**What's needed:**

- Report missing/extra dependencies in useEffect dep arrays
- Uses compiler's reactive scope dependency analysis
- Provides autofix
- Distinct from memo-dependencies because effect deps have different semantics

**Depends on:** Compiler core in lint mode

---

## Shared Utilities

### Gap 17: hook_detection.rs Expansion

**Upstream:** Various detection utilities used across rules
**Current state:** `utils/hook_detection.rs` exists as a stub.
**What's needed:**

- `is_hook_call(name: &str) -> bool` — starts with "use" + uppercase
- `is_component_name(name: &str) -> bool` — starts with uppercase, not all caps
- `is_effect_hook(name: &str) -> bool` — useEffect, useLayoutEffect, useInsertionEffect
- `is_state_hook(name: &str) -> bool` — useState, useReducer
- `is_ref_hook(name: &str) -> bool` — useRef
- `is_memo_hook(name: &str) -> bool` — useMemo, useCallback
- `get_hook_name_from_call(call: &CallExpression) -> Option<&str>`
- `is_inside_callback(node, semantic) -> bool` — check if we're inside an event handler or effect callback
- `is_inside_effect_callback(node, semantic) -> bool` — check if inside useEffect/useLayoutEffect callback

**Depends on:** None
