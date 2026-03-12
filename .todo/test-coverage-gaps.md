# Test Coverage Gaps

> Gaps identified by comparing our test infrastructure against the upstream
> babel-plugin-react-compiler test suite. Ordered by implementation dependency
> (most independent first, most complex last).

Last updated: 2026-03-12

---

### Gap 1: Config parsing and option construction tests ✅

**Completed.** 10 unit tests in `crates/oxc_react_compiler/src/entrypoint/options.rs` covering `PluginOptions::default()`, `from_map()`, all enum variant parsing, `GatingConfig::generate_wrapper()`, and `SourceFilter` construction.

---

### Gap 2: Error diagnostic fixture tests ✅

~~**Upstream:** ~390 fixtures prefixed `error.*` in `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler/`~~

**Completed**: Expanded from 4/17 to 17/17 DiagnosticKind variant coverage with 26 tests in `crates/oxc_react_compiler/tests/error_diagnostic_tests.rs`. Added `compile_program_with_config` API and `EnvironmentConfig::all_validations_enabled()`. Most tests document current state as `[]` since validation passes don't yet detect HIR patterns; this is expected and documented -- tests will start catching diagnostics as validation passes mature.

---

### Gap 3: Post-codegen output validation ✅

~~**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/__tests__/validateNoUseBeforeDefine.ts` runs ESLint `no-use-before-define` on every compiled output~~

**Completed**: Added oxc_semantic-based use-before-define checking in `crates/oxc_react_compiler/tests/codegen_validation_tests.rs`. 6 semantic tests with insta snapshots covering unresolved references, duplicate declarations, and scope analysis. Found real codegen bugs: unresolved references for props, derived values, and temporaries. Integration into other test runners (fixture_tests, conformance_tests, snapshot_tests) remains a future enhancement but is not blocking.

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
| 2 | Error diagnostic fixtures | ✅ Done | 17/17 variants, 26 tests |
| 3 | Post-codegen validation | ✅ Done | Parse + semantic validation, 6 semantic tests |
| 4 | E2E dual-mode tests | ✅ Done | 31 tests + infrastructure |
| 5 | Sprout runtime evaluation | ✅ Done | 11 tests + eval infrastructure |
