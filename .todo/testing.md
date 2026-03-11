# Testing Gaps

> The current test suite has 6 tests that verify the compiler doesn't crash, but none
> verify that it actually transforms code correctly.

---

## Gap 1: Upstream Fixture Test Harness

**Upstream:** `packages/babel-plugin-react-compiler/src/__tests__/fixtures/`
(thousands of fixture files with expected output)
**Current state:** No fixture test infrastructure exists. The 6 tests in
`tests/snapshot_tests.rs` all assert on `transformed: false` because the pipeline
doesn't work end-to-end.
**What's needed:**
- Download or vendored subset of upstream fixture files (input JS + expected output)
- Build a test harness that:
  1. Compiles each input with `compile_program`
  2. Compares output against expected (or generates snapshots for review)
  3. Tracks pass/fail rates to measure upstream compatibility
- Start with the simplest fixtures (basic components, simple hooks) and expand
- Pin to a specific upstream commit (see `UPSTREAM_VERSION.md`)
**Depends on:** Gap 1-4 in pipeline.md (end-to-end compilation must work first)

---

## Gap 2: End-to-End Snapshot Tests

**Current state:** `tests/snapshot_tests.rs` uses insta but all snapshots are of
the original source (untransformed).
**What's needed:**
- Add test cases that exercise the full pipeline:
  - Simple component with props -> should produce `_c(N)` + conditional blocks
  - Hook with useState -> should memoize derived values
  - Component with multiple JSX elements -> should produce multiple scopes
  - Component with conditional rendering -> should handle if/else scopes
- Use insta snapshots so output changes are reviewable
**Depends on:** Gap 1-4 in pipeline.md

---

## Gap 3: Per-Pass Unit Tests

**Current state:** No unit tests for individual passes (inference, reactive scopes, etc.)
**What's needed:**
- Test `infer_reactive_places` with hand-crafted HIR
- Test `infer_reactive_scope_variables` with known mutable ranges
- Test `propagate_scope_dependencies_hir` with known scope structures
- Test `codegen_function` with hand-crafted ReactiveFunction trees
- Test `build_reactive_function` with known CFG shapes
**Depends on:** None (can be written against the existing pass functions)
