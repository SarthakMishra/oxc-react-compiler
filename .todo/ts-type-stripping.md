# TS Type Stripping in Conformance Tests

> **Priority**: HIGH (quick win -- ~138 fixtures, easiest category)
> **Impact**: ~138 divergences resolved, moving pass rate from 230/1717 to ~368/1717 (~21.4%)

## Problem Statement

Our compiler output retains TypeScript type annotations (parameter types, return types, generic annotations, type assertions) while Babel strips them during compilation. This causes ~138 fixture comparisons to fail purely due to type annotation presence, not due to any behavioral difference in memoization.

Example divergence:
```typescript
// Our output:
function Component(props: Props): JSX.Element {
  const x: number = props.count;

// Babel output:
function Component(props) {
  const x = props.count;
```

The fix belongs in the **test normalization layer**, not in the compiler itself. The compiler should preserve type annotations (they are useful for downstream tools). Only the conformance comparison needs to strip them.

## Files to Modify

1. **`crates/oxc_react_compiler/Cargo.toml`** -- add `oxc_codegen` dependency (dev-dependency only)
2. **`Cargo.toml` (workspace)** -- add `oxc_codegen = "0.117"` to workspace dependencies
3. **`crates/oxc_react_compiler/tests/conformance_tests.rs`** -- update `normalize_output()` to strip TS types

## Implementation Plan

### Gap 1: Add oxc_codegen as a dev-dependency

**Upstream:** N/A (test infrastructure only)
**Current state:** `oxc_codegen` is not in the dependency tree. All `oxc_*` crates are pinned to 0.117.
**What's needed:**
- Add `oxc_codegen = "0.117"` to `[workspace.dependencies]` in root `Cargo.toml`
- Add `oxc_codegen.workspace = true` to `[dev-dependencies]` in `crates/oxc_react_compiler/Cargo.toml`
- Verify it compiles: `cargo check --tests`
**Depends on:** None

### Gap 2: Implement parse-print roundtrip for type stripping

**Upstream:** N/A (test infrastructure only)
**Current state:** `normalize_output()` does whitespace normalization and cache name normalization but does not strip TS types.
**What's needed:**
- Add a `strip_typescript_types(code: &str) -> String` function in `conformance_tests.rs`
- Implementation: parse with `oxc_parser` using `SourceType::tsx()`, then print with `oxc_codegen::Codegen` which strips TS-only syntax
- Call `strip_typescript_types()` on BOTH our output and the expected output inside `normalize_output()` (or just before the tokenize/compare step)
- The parse-print roundtrip should use `.with_typescript(true)` on the source type so the parser accepts TS syntax, and the codegen should emit JS-only output
**Depends on:** Gap 1

### Gap 3: Verify and measure impact

**Upstream:** N/A
**Current state:** 230/1717 pass
**What's needed:**
- Run conformance tests: `cargo test conformance -- --nocapture`
- Verify pass rate increases by ~138 (to ~368/1717)
- If the increase is less than expected, inspect remaining failures to understand if some TS fixtures have other divergences overlapping with Category 1 or 2
- Update `tests/conformance/known-failures.txt` to remove newly-passing fixtures
**Depends on:** Gap 2

## Risks and Notes

- **oxc_codegen formatting**: The parse-print roundtrip will also normalize whitespace and formatting, which may help or hurt comparison accuracy. The existing tokenizer already handles whitespace, so this should be neutral or positive.
- **JSX preservation**: Check whether `oxc_codegen` preserves JSX syntax or lowers it to `React.createElement`. If it preserves JSX, this roundtrip could also help with Category 2's JSX normalization. If it lowers JSX, we need to be careful to only apply the roundtrip for TS stripping purposes, or configure the codegen to preserve JSX.
- **Parse errors**: Some fixtures may have intentionally invalid TS that oxc_parser rejects. The `strip_typescript_types` function should fall back to returning the original code if parsing fails.
- **Performance**: Adding a parse-print roundtrip per fixture adds ~1ms per fixture. With ~1717 fixtures this adds ~2s to the conformance suite, which is acceptable.
