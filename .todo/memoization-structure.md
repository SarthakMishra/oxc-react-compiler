# Memoization Structure Differences

> **Priority**: MEDIUM (structural correctness -- ~605 fixtures)
> **Impact**: ~605 divergences, moving pass rate significantly when combined with TS stripping

## Problem Statement

When both our compiler and Babel memoize a function, our output differs structurally in four ways:

1. **Temp variable explosion** -- HIR SSA form leaks into codegen. Simple expressions like `useRef(null)` get decomposed into multiple temporaries: `const t32 = useRef; const t33 = null; const t34 = t32(t33)` instead of Babel's `const t0 = useRef(null)`.

2. **JSX lowering** -- We emit `_jsx("div", {...})` function calls while Babel preserves `<div>...</div>` JSX syntax in its output. This is a fundamental codegen difference.

3. **Cache slot count mismatches** -- Our `_c(N)` often has a different N than Babel's, because we create more temporaries and/or have different scope boundaries.

4. **Scope boundary differences** -- Different reactive scope merging/splitting decisions lead to different cache slot groupings.

Issues 1 and 3 are tightly coupled (fewer temps = fewer cache slots). Issues 2 and 4 are somewhat independent.

## Files to Modify

### Temp Variable Inlining
- **`crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`** -- add inline pass or modify codegen to collapse SSA chains
- Potentially new file: **`crates/oxc_react_compiler/src/optimization/inline_temporaries.rs`** -- post-RF pass to inline trivial SSA chains

### JSX Preservation
- **`crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`** -- lines 325-348, modify JSX codegen to emit JSX syntax instead of `_jsx()` calls
- **`crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`** -- line 920, update import header to not include jsx-runtime imports when preserving JSX

### Scope Merging
- **`crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`** -- review merge heuristics vs upstream
- **`crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`** -- review prune decisions vs upstream

## Implementation Plan

### Gap 1: Temp Variable Inlining Pass

**Upstream:** Babel's codegen never sees raw SSA temps -- its IR-to-code translation directly inlines simple expressions. The relevant upstream logic is spread across `CodegenReactiveFunction.ts` and `PrintReactiveFunction.ts`.
**Current state:** Our codegen in `codegen.rs` faithfully emits every SSA temporary as a separate `const` declaration. The `prune_temporary_lvalues` pass (Pass 9.5) removes some trivial temps but not all.
**What's needed:**
- Implement a temp-inlining pass that runs after `rename_variables` (Pass 59) but before codegen
- For each temporary `tN` that is: (a) assigned exactly once, (b) used exactly once, (c) the use immediately follows the definition with no intervening side effects -- inline the RHS expression into the use site
- Example: `const t0 = useRef; const t1 = null; const t2 = t0(t1)` collapses to `const t2 = useRef(null)`
- This pass operates on the `ReactiveFunction` tree, walking instructions and maintaining a use-count map
- Alternatively, this can be done during codegen itself (peephole inlining) by buffering pending simple assignments and substituting when the temp is referenced
**Depends on:** None

### Gap 2: JSX Syntax Preservation in Codegen

**Upstream:** `CodegenReactiveFunction.ts` emits JSX syntax directly (`<div>`, `<Component>`, `<>{...}</>`)
**Current state:** `codegen.rs` lines 325-348 emit `_jsx("div", { ... })` and `_jsxs(_Fragment, { children: [...] })` function call syntax
**What's needed:**
- Modify the `InstructionValue::JsxExpression` arm in `codegen_instruction()` to emit JSX syntax:
  - `_jsx("div", { className: x })` becomes `<div className={x} />`
  - `_jsx(Component, { prop: val })` becomes `<Component prop={val} />`
  - `_jsxs("div", { children: [a, b] })` becomes `<div>{a}{b}</div>`
  - `_jsx(_Fragment, { children: x })` becomes `<>{x}</>`
- Handle self-closing vs open/close tags (self-closing when no children)
- Handle spread props: `_jsx("div", { ...props })` becomes `<div {...props} />`
- Handle string children vs expression children
- Remove the `jsx-runtime` import from the generated import header (line ~920) since JSX syntax doesn't need it
- Keep the `_c` import from `react/compiler-runtime`
**Depends on:** None (independent of Gap 1)

### Gap 3: Cache Slot Count Alignment

**Upstream:** Babel counts cache slots based on reactive scope outputs + dependencies
**Current state:** Our `count_cache_slots()` function counts slots based on our scope structure, which may differ from Babel's due to different scope boundaries and extra temporaries
**What's needed:**
- After Gap 1 (temp inlining) is done, re-measure cache slot divergences -- many may be resolved by having fewer temps
- Compare `count_cache_slots` logic with upstream's `getScopeCount` in `CodegenReactiveFunction.ts`
- Verify that scope outputs and declarations match upstream's expectations
- This gap may be fully resolved by Gap 1 + Gap 4, or may require targeted fixes
**Depends on:** Gap 1 (temp inlining reduces slot count)

### Gap 4: Scope Merging/Splitting Heuristic Review

**Upstream:** `MergeOverlappingReactiveScopes.ts`, `PruneNonEscapingScopes.ts`, `PruneAlwaysInvalidatingScopes.ts`
**Current state:** We have `merge_scopes.rs` and `prune_scopes.rs` implementing these passes, but the heuristics may diverge in edge cases
**What's needed:**
- Audit `merge_overlapping_reactive_scopes_hir` against upstream `MergeOverlappingReactiveScopes.ts` -- check the merge condition (do we merge scopes whose ranges overlap the same way Babel does?)
- Audit `merge_reactive_scopes_that_invalidate_together` against upstream -- check the "invalidate together" heuristic
- Audit scope terminal construction (`build_reactive_scope_terminals_hir`) -- verify we create scope boundaries at the same points
- Fix any divergences found
- Re-measure after fixes
**Depends on:** None (can be done in parallel with Gap 1)

### Gap 5: Test Normalization for JSX (alternative to Gap 2)

**Upstream:** N/A (test infrastructure)
**Current state:** The `normalize_output()` function in conformance tests does not normalize JSX vs `_jsx()` calls
**What's needed:**
- If Gap 2 (JSX preservation) proves too complex for an initial pass, an alternative is to normalize BOTH our `_jsx()` output and Babel's `<div>` JSX syntax to a common form in the test comparison layer
- This could use `oxc_codegen` (from the TS stripping work) to parse-print both outputs, which would normalize JSX to a consistent form
- However, this is a workaround, not a fix -- the compiler should ideally emit JSX syntax to match Babel
- Only pursue this if Gap 2 is blocked
**Depends on:** TS stripping Gap 1 (oxc_codegen dependency)

## Measurement Strategy

After each gap, run conformance and measure:
```bash
cargo test conformance -- --nocapture 2>&1 | tail -5
```

Expected progression (approximate, overlapping categories):
- After Gap 1 (temp inlining): ~150-200 new passes (many structural mismatches due to temps)
- After Gap 2 (JSX preservation): ~100-150 additional passes
- After Gap 3+4 (cache slots + scopes): ~50-100 additional passes
- Total from this category: ~400-500 new passes (some fixtures have multiple issues)

## Risks and Notes

- **Temp inlining correctness**: Must verify that inlined expressions maintain the same evaluation order. Only inline pure expressions or expressions where order doesn't matter.
- **JSX edge cases**: Self-closing elements, boolean attributes (`<div disabled />`), computed property names in JSX, namespace attributes (`xml:lang`).
- **Overlap with Category 1**: Some of the 605 fixtures in this category may also be over-memoized (Category 1). Fixing structure alone won't make them pass -- they need bail-out heuristics too. The actual net gain from this category may be ~300-400 fixtures.
- **Scope merging audit scope**: The merge/prune passes are among the most complex in the compiler. A full audit requires careful line-by-line comparison with upstream TypeScript.
