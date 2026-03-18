# Codegen Emission Gaps

Issues in `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` and `crates/oxc_react_compiler/src/reactive_scopes/build_reactive_function.rs`.

Completed: Gaps 1-5, 6b, 7, 8, 9, fixture bugs. Remaining: output format divergences, render divergences, ternary reconstruction.

---

## Gap 1: Duplicate Declarations in `codegen_scope` -- COMPLETED

## Gap 2: Hook Destructuring Codegen -- COMPLETED

## Gap 3: Variable Ordering / Use-Before-Declare -- COMPLETED

## Gap 4: Assignment vs Re-declaration -- COMPLETED

## Gap 5: Logical Expression Flattening -- COMPLETED

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

## Gap 6b: ForOf/ForIn Loop Codegen -- COMPLETED

## Gap 7: availability-schedule Arithmetic -- COMPLETED

## Gap 8: canvas-sidebar Missing Return -- COMPLETED

## Gap 9b: booking-list localeCompare -- COMPLETED (fixture bug)

---

## Gap 11: Destructuring Pattern Codegen

**Priority:** P1 -- 43 conformance fixtures diverge because we emit member access instead of destructuring

**Current state:** When upstream emits `const { x, y } = t0` or `const [a, b] = t0`, we emit `const x = t0.x; const y = t0.y` (property access form). This causes 34 object-destructuring and 9 array-destructuring divergences in conformance.

**What's needed:**
- Detect when a temporary is destructured (multiple properties read from the same source in the same scope)
- Emit destructuring pattern form instead of individual property access assignments
- Handle both object `{ }` and array `[ ]` destructuring
- Handle nested destructuring and rest elements

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts` (destructuring reconstruction)
**Depends on:** None

---

## Gap 12: Named Variable Preservation

**Priority:** P1 -- ~80+ conformance fixtures diverge because we use temp names where upstream preserves original names

**Current state:** Our codegen assigns temporary names (`t0`, `t1`, ...) to intermediate values. The upstream compiler preserves original variable names from the source when possible (e.g., `const x = ...` instead of `const t0 = ...`). After normalization, temp names are canonicalized, but the divergence appears when the expected output uses a named variable and we use a temp in its place.

**What's needed:**
- Investigate how upstream `CodegenReactiveFunction.ts` decides when to use original names vs temps
- Preserve the original identifier name from the HIR when emitting declarations
- Likely requires carrying the original name through the reactive scope tree

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Depends on:** None

---

## Gap 13: Async Function Emission

**Priority:** P2 -- ~1 fixture diverges because we drop `async` keyword

**Current state:** At least 1 conformance fixture has `ours=[function]` vs `exp=[async]`, indicating we don't preserve the `async` keyword on function declarations.

**What's needed:**
- Check if `async` flag is preserved through HIR lowering
- Emit `async function` when the original was async

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Depends on:** None

---

## Gap 14: Known-Failures Housekeeping

**Priority:** P1 -- quick win

**Current state:** 2 fixtures are newly passing (should be removed from known-failures.txt) and 5 fixtures have regressed (should be added). The conformance test prints these at the end of each run.

**What's needed:**
- Remove `error.unconditional-set-state-in-render-after-loop.js` from known-failures.txt
- Remove `jsx-bracket-in-text.jsx` from known-failures.txt
- Add the 5 unexpected divergences to known-failures.txt

**Depends on:** None

---

## Gap 15: Remaining Render Divergences (3 fixtures)

**Priority:** P3 -- minor visual differences in E2E benchmarks

**Current state:** 3 of 25 benchmark fixtures show `semantic_divergence`:
1. **command-menu**: Active item class styling differs (likely conditional logic in compiled output)
2. **canvas-sidebar**: Minor content differences (possibly JSX text whitespace edge case)
3. **multi-step-form**: Shows "0/0 fields" instead of "0/1 fields" (field count logic difference)

**What's needed:**
- Diff the OXC vs Babel compiled output for each to find the specific codegen difference
- These are likely symptoms of scope/memoization issues rather than pure codegen bugs

**Depends on:** Possibly scope inference fixes (Gap 7/11 in scope-inference.md)
