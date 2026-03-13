# Compiled No Memo -- Non-Memoization Transforms

> **Priority**: P3 (152 fixtures, moderate tractability)
> **Impact**: 152 fixtures where Babel transforms the code but emits no `_c()` memoization
> **Tractability**: MODERATE -- requires understanding what non-memo transforms Babel applies

## Problem Statement

For 152 fixtures, Babel's compiler pipeline transforms the source code but does
not add `_c()` memoization. The output differs from the input but contains no
cache slots. Our compiler either:
- Adds memoization (wrong -- we should have bailed or produced a non-memo transform)
- Returns source unchanged (wrong -- Babel did transform it)

These fixtures represent non-memoization compiler behaviors like:
- Dead code elimination (removing unreachable branches)
- Constant propagation/folding
- Arrow function extraction (outlining helper functions)
- Variable renaming / scope cleanup
- Function signature normalization

## Sub-categories (needs triage)

The 152 fixtures have not been triaged into sub-categories yet. The first step
is to analyze what transforms Babel applies to understand the distribution.

### Gap 1: DCE and Constant Propagation

**Upstream:** `ConstantPropagation.ts`, `DeadCodeElimination.ts`
**Current state:** `crates/oxc_react_compiler/src/optimization/constant_propagation.rs` and `dead_code_elimination.rs` exist but may be incomplete or not applied in the right pipeline position.
**What's needed:**
- Audit the existing DCE pass against upstream `DeadCodeElimination.ts`
- Audit constant propagation against upstream `ConstantPropagation.ts`
- Ensure these passes run even when no memoization scopes are created
- In Babel, these passes produce transformed output even for functions that get zero reactive scopes
- Our pipeline may skip codegen entirely when there are zero scopes (the zero-scope bail-out), which would prevent non-memo transforms from being emitted
**Fixture gain estimate:** Unknown until triaged
**Depends on:** Triage of the 152 fixtures

### Gap 2: Arrow Extraction

**Upstream:** `OutlineJSX.ts`, `OutlineFunctions.ts`
**Current state:** `outline_jsx.rs` and `outline_functions.rs` exist but were verified as no-ops in current tests.
**What's needed:**
- Determine if upstream's outlining passes produce output for any of the 152 fixtures
- If so, enable these passes to produce transformed output even without memoization
**Fixture gain estimate:** Unknown until triaged
**Depends on:** Triage of the 152 fixtures

## Next Steps

1. **Triage**: Run the 152 fixtures through Babel and categorize what transforms were applied
2. **Prioritize**: Focus on the sub-category with the most fixtures
3. **Implement**: Port or fix the relevant transform pass

## Risks and Notes

- The zero-scope bail-out (returning source unchanged when no reactive scopes exist) may conflict with these transforms. Babel returns *transformed* source even when there are zero memoization scopes. We return *original* source. This is a fundamental architectural difference that needs resolution.
- Some of these transforms may not be worth porting if they represent Babel-specific behavior that doesn't affect correctness.
