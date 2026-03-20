# Codegen Emission Gaps

Issues in `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` and `crates/oxc_react_compiler/src/reactive_scopes/build_reactive_function.rs`.

Completed: Gaps 1-5, 6b, 7, 8, 9, 9b, 11, 13, 14. Remaining: Gap 6 (ternary reconstruction, P4), Gap 12 (named variable preservation), Gap 15 (1 render divergence).

---

## Gap 6: Ternary Expression Reconstruction

**Priority:** P4 -- functionally correct but produces `if/else` instead of `?:` for expression-position ternaries

**Current state:** `Terminal::Ternary` is converted to `ReactiveTerminal::If` and emitted as `if/else`. This is functionally correct but diverges from upstream output form. The `result: Option<Place>` field (which indicates expression-position ternaries that should emit `test ? consequent : alternate`) is ignored.

**What's needed:**
- Preserve the `result` place when converting `Terminal::Ternary` to `ReactiveTerminal::If` (or create a `ReactiveTerminal::Ternary` variant)
- In codegen, when a ternary has a result place, emit conditional expression form
- When no result place (statement position), keep `if/else` form

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Depends on:** None

---

## Gap 12: Named Variable Preservation

**Priority:** P2 -- ~80+ conformance fixtures diverge because we use temp names where upstream preserves original names

**Current state:** Our codegen assigns temporary names (`t0`, `t1`, ...) to intermediate values. The upstream compiler preserves original variable names from the source when possible (e.g., `const x = ...` instead of `const t0 = ...`). After normalization, temp names are canonicalized, but the divergence appears when the expected output uses a named variable and we use a temp in its place.

**What's needed:**
- Investigate how upstream `CodegenReactiveFunction.ts` decides when to use original names vs temps
- Preserve the original identifier name from the HIR when emitting declarations
- Likely requires carrying the original name through the reactive scope tree

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Depends on:** None

---

## Gap 15: Remaining Render Divergence (1 fixture)

**Priority:** P3 -- minor visual difference in E2E benchmark

**Current state:** 1 of 25 benchmark fixtures shows render divergence (down from 3). The 24/25 render equivalence rate (96%) represents significant progress. The remaining fixture likely has a scope/memoization root cause rather than a pure codegen bug.

**What's needed:**
- Identify which fixture still diverges and diff OXC vs Babel compiled output
- Likely a symptom of scope inference issues rather than codegen

**Depends on:** Possibly scope inference fixes (Gap 7/11 in scope-inference.md)
