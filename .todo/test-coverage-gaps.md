# Test Coverage Gaps

> Gaps identified by comparing our test infrastructure against the upstream
> babel-plugin-react-compiler test suite. Ordered by implementation dependency
> (most independent first, most complex last).

Last updated: 2026-03-12

---

### Gap 1: Config parsing and option construction tests ✅

**Completed.** 10 unit tests in `crates/oxc_react_compiler/src/entrypoint/options.rs` covering `PluginOptions::default()`, `from_map()`, all enum variant parsing, `GatingConfig::generate_wrapper()`, and `SourceFilter` construction.

---

### Gap 2: Error diagnostic fixture tests [~]

**Upstream:** ~390 fixtures prefixed `error.*` in `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler/`

**Done so far:**
- `crates/oxc_react_compiler/tests/error_diagnostic_tests.rs` with 8 tests using inline source strings and insta snapshots
- Covers 4 DiagnosticKind variants: `HooksViolation` (conditional), `JsxInTry`, `SetStateInRender`, and the `"use no memo"` directive path
- Note: hooks-in-loop test removed due to compiler infinite loop in for-of lowering

**Remaining:**
- Expand coverage to remaining 13 DiagnosticKind variants: `ImmutabilityViolation`, `RefAccessInRender`, `SetStateInEffects`, `CapitalizedCalls`, `ContextVariableLvalues`, `MemoDependency`, `EffectDependency`, `MemoizationPreservation`, `DerivedComputationsInEffects`, `LocalsReassignedAfterRender`, `UseMemoValidation`, `InvariantViolation`, `Other`
- Optionally migrate to fixture-file approach (`fixtures/errors/*.js`) with directory walker
- Expand toward 50+ fixtures as validation passes mature

**Depends on:** Validation passes must actually emit the corresponding diagnostics (some may be stubs)

---

### Gap 3: Post-codegen output validation [~]

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/__tests__/validateNoUseBeforeDefine.ts` runs ESLint `no-use-before-define` on every compiled output

**Done so far:**
- `crates/oxc_react_compiler/tests/codegen_validation_tests.rs` with 11 tests validating compiled output is parseable JavaScript using `oxc_parser`
- Covers simple components, hooks, conditionals, exports, arrow functions, JSX children, multiple components, and edge cases

**Remaining:**
- Add `oxc_semantic`-based use-before-define checking (all referenced identifiers have a binding, no undeclared vars, no duplicate declarations)
- Integrate validation into `fixture_tests.rs`, `conformance_tests.rs`, and `snapshot_tests.rs` so every compiled output is validated automatically

**Depends on:** None

---

### Gap 4: E2E dual-mode rendering tests ✅

**Completed.** `tests/e2e/` with Vitest, esbuild JSX transform, vm-based eval, ReactDOMServer rendering. 31 tests across 5 files (basic-component, conditional-render, derived-values, list-render, jsx-attributes). Dual-mode comparisons use `it.fails` for known codegen issues — they auto-flip when bugs are fixed.

---

### Gap 5: Sprout-equivalent runtime evaluation ✅

**Completed.** `tests/e2e/sprout-eval.test.ts` with 11 tests covering pure function evaluation, mutation tracking, sequential render consistency, and dual-mode return value comparison. Shared runtime utilities (mutate, identity, makeObject, etc.) in `helpers/eval-function.ts`.

---

## Summary

| Gap | Name | Status | Scope |
|-----|------|--------|-------|
| 1 | Config parsing tests | ✅ Done | 10 unit tests |
| 2 | Error diagnostic fixtures | [~] Partial | 4/17 variants covered, expand to remaining |
| 3 | Post-codegen validation | [~] Partial | Parse validation done, use-before-define missing |
| 4 | E2E dual-mode tests | ✅ Done | 31 tests + infrastructure |
| 5 | Sprout runtime evaluation | ✅ Done | 11 tests + eval infrastructure |
