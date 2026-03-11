# Test Coverage Gaps

> Gaps identified by comparing our test infrastructure against the upstream
> babel-plugin-react-compiler test suite. Ordered by implementation dependency
> (most independent first, most complex last).

Last updated: 2026-03-11

---

### Gap 1: Config parsing and option construction tests

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/__tests__/parseConfigPragma-test.ts`, `compiler/packages/babel-plugin-react-compiler/src/__tests__/envConfig-test.ts`
**Current state:** Zero tests for `PluginOptions`, `CompilationMode`, `OutputMode`, `GatingConfig`, `SourceFilter`, `PanicThreshold`, or the NAPI `TransformOptions` -> `PluginOptions` conversion path. All option construction is exercised only indirectly through fixture tests.
**What's needed:**
- Inline `#[cfg(test)]` module in `crates/oxc_react_compiler/src/entrypoint/options.rs`:
  - `PluginOptions::default()` returns expected values
  - `PluginOptions::from_map()` with valid keys, unknown keys, empty map
  - `CompilationMode` string matching: "all", "syntax", "annotation", "infer", unknown -> Infer
  - `OutputMode` string matching: "ssr", "lint", "client", unknown -> Client
  - `ReactTarget` string matching: "17", "react17", "18", "react18", unknown -> React19
  - `PanicThreshold` string matching: "all", "ALL_ERRORS", "none", "NONE", unknown -> CriticalErrors
  - `GatingConfig::generate_wrapper()` produces correct import + if-guard
  - `SourceFilter` include/exclude construction
- NAPI conversion tests in `napi/react-compiler/src/lib.rs` (or a test file):
  - `TransformOptions { compilation_mode: Some("all"), .. }` produces `CompilationMode::All`
  - `TransformOptions { gating_import_source: Some(..), gating_function_name: Some(..) }` produces `Some(GatingConfig { .. })`
  - `TransformOptions` with all `None` fields produces `PluginOptions::default()`
  - `output_mode` field is passed through (currently not wired -- this may surface a bug)
- Pragma directive parsing tests if/when we support `"use memo"` / `"use no memo"` directives
**Depends on:** None -- fully independent, good starter task

---

### Gap 2: Error diagnostic fixture tests

**Upstream:** ~390 fixtures prefixed `error.*` in `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler/`
**Current state:** Zero tests that verify the compiler reports correct diagnostics. All 17 `DiagnosticKind` variants are untested for correct emission. Error paths are exercised only by the upstream conformance runner's panic detection.
**What's needed:**
- Create `crates/oxc_react_compiler/tests/fixtures/errors/` directory
- Create fixture files (one per diagnostic scenario), each a `.js` file with code that should trigger a specific error:
  - `hooks-violation-conditional.js` -- conditional hook call -> `DiagnosticKind::HooksViolation`
  - `hooks-violation-loop.js` -- hook in loop -> `DiagnosticKind::HooksViolation`
  - `immutability-mutation-of-frozen.js` -> `DiagnosticKind::ImmutabilityViolation`
  - `ref-access-in-render.js` -> `DiagnosticKind::RefAccessInRender`
  - `set-state-in-render.js` -> `DiagnosticKind::SetStateInRender`
  - `set-state-in-effects.js` -> `DiagnosticKind::SetStateInEffects`
  - `jsx-in-try.js` -> `DiagnosticKind::JsxInTry`
  - `capitalized-calls.js` -> `DiagnosticKind::CapitalizedCalls`
  - `context-variable-lvalue.js` -> `DiagnosticKind::ContextVariableLvalues`
  - `memo-dependency-missing.js` -> `DiagnosticKind::MemoDependency`
  - `effect-dependency-missing.js` -> `DiagnosticKind::EffectDependency`
  - `memoization-preservation.js` -> `DiagnosticKind::MemoizationPreservation`
  - `derived-computation-in-effect.js` -> `DiagnosticKind::DerivedComputationsInEffects`
  - `locals-reassigned-after-render.js` -> `DiagnosticKind::LocalsReassignedAfterRender`
  - `use-memo-validation.js` -> `DiagnosticKind::UseMemoValidation`
  - `invariant-violation.js` -> `DiagnosticKind::InvariantViolation`
  - `invalid-react-general.js` -> `DiagnosticKind::Other` (InvalidReact category)
  - `invalid-js-general.js` -> `DiagnosticKind::Other` (InvalidJS category)
  - `todo-unsupported.js` -> ErrorCategory::Todo
  - Multiple errors in one file -> verify all are collected
- Create `crates/oxc_react_compiler/tests/error_diagnostic_tests.rs`:
  - Walk `fixtures/errors/` directory
  - For each fixture: parse, compile, collect `CompilerError` list
  - Snapshot the diagnostic output with insta (kind, category, message, span offsets)
  - Assert specific `DiagnosticKind` variant is present
- Expand to 50+ fixtures as validation passes mature
**Depends on:** None -- independent of other gaps, but requires that the validation passes actually emit errors (some may be stubs today)

---

### Gap 3: Post-codegen output validation

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/__tests__/validateNoUseBeforeDefine.ts` runs ESLint `no-use-before-define` on every compiled output
**Current state:** No validation that our codegen output is syntactically valid JavaScript or that variables are declared before use. Codegen bugs silently produce broken JS.
**What's needed:**
- Create `crates/oxc_react_compiler/src/validation/codegen_validator.rs` (or a test-only module):
  - `validate_codegen_output(code: &str) -> Vec<ValidationError>` function
  - Parse with `oxc_parser` -- any parse errors are validation failures
  - Use `oxc_semantic` to build a semantic analysis of the output
  - Check: all referenced identifiers have a binding (no use-before-define, no undeclared vars)
  - Check: no duplicate declarations in the same scope
  - Return structured `ValidationError` with message and location
- Integrate into existing test runners:
  - `fixture_tests.rs`: after each fixture compiles, run `validate_codegen_output()` on the result
  - `conformance_tests.rs`: add validation to the ~1263 upstream fixture runs
  - `snapshot_tests.rs`: validate each snapshot output
- Fail the test if any validation error is found (not just log it)
**Depends on:** None -- can be built independently. Uses `oxc_parser` and `oxc_semantic` which are already workspace dependencies.

---

### Gap 4: E2E dual-mode rendering tests

**Upstream:** 5 `.e2e.js` files in `compiler/packages/babel-plugin-react-compiler/src/__tests__/e2e/` using dual Jest configs (`__FORGET__ = false` for baseline, `__FORGET__ = true` for compiled)
**Current state:** We have no tests that render React components and compare compiled vs uncompiled behavior in a real DOM environment.
**What's needed:**
- Create `tests/e2e/` directory at project root
- Set up a Node.js test environment:
  - `package.json` with vitest, @testing-library/react, jsdom, react, react-dom
  - `vitest.config.ts` with jsdom environment
- Create test helper `tests/e2e/helpers/compile.ts`:
  - Imports `transformReactFile` from `napi/react-compiler/index.js`
  - `compileAndEval(source: string)` -- compiles, then evaluates in a sandboxed module context
  - `evalOriginal(source: string)` -- evaluates source directly
- Create test files:
  - `tests/e2e/basic-component.test.tsx` -- renders a simple component, asserts same DOM output
  - `tests/e2e/state-update.test.tsx` -- triggers state updates, asserts same behavior
  - `tests/e2e/event-handler.test.tsx` -- clicks, asserts same side effects
  - `tests/e2e/conditional-render.test.tsx` -- conditional branches, asserts same output
  - `tests/e2e/list-render.test.tsx` -- .map() rendering, asserts same output
- Each test:
  1. Defines a React component as a string
  2. Renders the original source
  3. Compiles with `transformReactFile()`, renders the compiled source
  4. Asserts DOM output is identical
  5. Optionally asserts fewer re-renders (using React profiler or render counting)
- CI integration: add to the test matrix, runs after `cargo test` and NAPI build
**Depends on:** NAPI binding must be working (it is). Codegen must produce runnable JS (partially working -- blocked on memoization correctness from Priority 2 items).

---

### Gap 5: Sprout-equivalent runtime evaluation

**Upstream:** `compiler/packages/snap/src/sprout/` system that evaluates original and compiled code side-by-side, comparing return values, console output, and exceptions for each fixture
**Current state:** We have no runtime evaluation of compiler output. The conformance runner only checks for panics and textual output match, not semantic correctness.
**What's needed:**
- Create `tests/sprout/` directory at project root
- Core evaluation infrastructure:
  - `tests/sprout/package.json` with dependencies: vitest, @testing-library/react, jsdom, react, react-dom
  - `tests/sprout/shared-runtime.ts` -- port of upstream `shared-runtime.ts` providing `mutate()`, `identity()`, `makeObject()`, `throwInput()`, `Stringify`, `ValidateMemoization` etc.
  - `tests/sprout/evaluator.ts`:
    - Takes a fixture file path
    - Compiles with `transformReactFile()` from NAPI binding
    - Checks if source exports `FIXTURE_ENTRYPOINT = { fn, params, sequentialRenders? }`
    - If yes: evaluates `fn(...params)` on BOTH original and compiled code
    - If `sequentialRenders`: renders the component multiple times with different props
    - Captures: return values, console.log output, thrown exceptions
    - Compares all three between original and compiled
  - `tests/sprout/sandbox.ts` -- sandboxed JS evaluation environment (vm module or similar)
- Phases of rollout:
  - Phase A: pure function fixtures (no React rendering) -- call `fn(...params)`, compare return values
  - Phase B: component fixtures with `sequentialRenders` -- use @testing-library/react to render
  - Phase C: full upstream fixture sweep -- run evaluator on all ~1263 upstream fixtures that export `FIXTURE_ENTRYPOINT`
- Integration with snapshot output:
  - Append `### Eval output` section to insta snapshots (matching upstream format)
  - Track pass/fail rate as a "semantic correctness score" alongside the existing conformance score
- This is the most important test gap -- it is the ONLY way to catch semantic bugs where the compiler produces syntactically valid but behaviorally wrong output
**Depends on:** Gap 3 (post-codegen validation) to ensure output is parseable JS before evaluation. Gap 4 (E2E dual-mode) shares infrastructure. NAPI binding must be working (it is).

---

## Summary

| Gap | Name | Depends on | Scope |
|-----|------|------------|-------|
| 1 | Config parsing tests | None | ~20 unit tests |
| 2 | Error diagnostic fixtures | None | ~20 fixtures + test runner |
| 3 | Post-codegen validation | None | 1 module + integration |
| 4 | E2E dual-mode tests | NAPI, memoization | 5 test files + setup |
| 5 | Sprout runtime evaluation | Gap 3, Gap 4 infra | evaluator + shared-runtime + fixture sweep |
