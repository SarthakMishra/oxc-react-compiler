# Render Equivalence and Correctness Bugs

> Fix compiler output bugs that cause runtime behavior to diverge from the upstream Babel plugin.

**Priority:** P1 (Must Build Now) -- these are correctness issues where the compiled output is semantically wrong or incomplete.

---

## Recently Fixed (Context for Future Work)

These four bugs were fixed in commits `8a13bc8` and `0550f08`, improving the score from 0.000 → 0.24:

### Fixed Bug 1 — Terminal Lowering (`build_reactive_function.rs`)

When converting the HIR CFG into a flat `ReactiveBlock`, six terminal types — `Ternary`, `Logical`, `Optional`, `Sequence`, `Branch`, `MaybeThrow` — all hit a catch-all `break`, silently dropping all code reachable from those terminals. Each now has an explicit match arm:
- **Ternary** → lowered to `ReactiveTerminal::If` (same structure, just a different CFG representation)
- **Logical** → right-side block is inlined (short-circuit not needed; React compiler evaluates both paths for memoization)
- **Optional** → consequent block inlined
- **Sequence** → all sub-blocks processed in order
- **Branch** → lowered to `ReactiveTerminal::If` (Branch has no fallthrough, so it `break`s after)
- **MaybeThrow** → follows the continuation path

The same set of terminals was also missing from `build_scope_block_only` (the scope-aware variant), which got matching treatment.

### Fixed Bug 2 — Parameter Destructuring Hoisting (`codegen.rs`)

When a component like `function Badge({ status })` is compiled, the HIR produces a `Destructure` instruction (`const { status } = t0`) that appears *after* reactive scope guards. But scope guards reference `status` in their dependency checks — so the variable was used before being declared. Before generating the body, codegen now:
1. Collects all parameter names from `rf.params`
2. Scans the body for `Destructure` instructions whose source is a parameter
3. Hoists those to the top of the function output
4. Registers the destructured names via `collect_pattern_names` so later `DeclareLocal` instructions skip re-declaration

A new `codegen_block_skip_hoisted` function processes the body while skipping already-hoisted indices.

### Fixed Bug 3 — HIR Body CFG Traversal (`codegen.rs`)

`codegen_hir_body` (used for nested function expressions) iterated `hir.blocks` in storage order — not CFG order. This produced duplicate blocks, wrong ordering, and missed ternary/logical lowering. Replaced with `build_reactive_block_from_hir(hir, hir.entry)` — the same CFG-to-reactive conversion used for top-level functions — then fed through the standard `codegen_block` pipeline. The old `codegen_hir_terminal` function was deleted entirely.

### Fixed Bug 4 — Fallthrough Duplication (`build_reactive_function.rs`)

When an `If` or `Ternary` terminal has a fallthrough block, both the consequent and alternate branches would `Goto` the fallthrough. `build_reactive_block` would then follow those `Goto`s and inline the fallthrough code into *both* branches, duplicating it. Introduced `build_reactive_block_until(hir, start, stop_at)` — when a `Goto` targets `stop_at`, it stops instead of following. All `If` and `Ternary` arms now pass `Some(fallthrough)` as the stop block.

---

## Current State

Render equivalence score: **0.24** (6/25 test cases match upstream across 16 benchmark fixtures).

**Passing fixtures (6/25 matching test cases):**
- `simple-counter`: 1/1
- `todo-list`: 1/1
- `status-badge`: 4/4 (all sub-tests pass)

**Failing categories across the remaining fixtures:**

### Category A: Truncated/Incomplete Output (Critical)

The `availability-schedule` fixture is the most visible example:
- OXC output: 115 lines (truncated mid-function, no memoization at all)
- Babel output: 387 lines (complete with 31 cache slots, 14 scope blocks)
- Diff analysis: `semantic_difference / bug / "Babel memoizes but OXC does not"`
- The OXC output stops after destructuring props and calling `useReducer`, emitting no cache logic
- `cacheSize: 0, sentinelChecks: 0, scopeBlocks: 0` vs Babel's `cacheSize: 31, scopeBlocks: 14`

This likely indicates a panic/bailout in the compilation pipeline for large/complex components (switch statements, `useReducer`, computed property access patterns). The `catch_unwind` in the NAPI layer silently swallows the error and returns partial output.

`multi-step-form` has a similar pattern (also an L-tier fixture that may timeout or segfault).

### Category B: Unresolved Temporary Variables (Phi-Node Resolution)

Multiple fixtures produce output containing unresolved temporary variable names like `t25`, `t59`, `t75`, `t16`, `t123` that should have been resolved to their actual values during codegen. These appear in:
- Ternary expressions: `condition ? t25 : t26` instead of the actual expressions
- Logical expressions: `x && t59` instead of the resolved value
- JSX attribute values: `<div className={t75}>` instead of the computed expression

This indicates the phi-node resolution in `leave_ssa.rs` or the codegen temporary variable elimination is not fully handling all branch merge points. The upstream compiler resolves all temporaries before codegen.

**Upstream reference:** `src/ReactiveScopes/LeaveSSA.ts` and `src/Codegen/CodegenReactiveFunction.ts`

### Category C: JSX Attribute Name Quoting

JSX attributes with hyphens (e.g., `aria-label`, `data-testid`) are not being quoted correctly in the output. This causes parse errors in the compiled output.

**Expected:** `"aria-label": value` (as a string property in the JSX factory call)
**Actual:** `aria-label: value` (invalid JS identifier used as property name)

This is a codegen bug in the JSX attribute emission path.

**Upstream reference:** `src/Codegen/CodegenReactiveFunction.ts` (JSX attribute handling)

### Category D: Fixtures That Fail in Both Compilers

Some fixtures (`time-slot-picker`, `data-table`) require default prop values or specific setup that neither compiler handles in isolation. These are test infrastructure issues, not compiler bugs.

---

## Gap 1: Availability-Schedule Truncated Output

**Upstream:** Full pipeline -- the issue is that compilation bails out or panics partway through
**Current state:** Output is 115 lines (truncated) with zero memoization patterns
**What's needed:**
- Investigate why the availability-schedule fixture produces truncated output:
  1. Run the fixture through the compiler with panic logging enabled (not `catch_unwind`)
  2. Identify the exact pass that fails (likely during BuildHIR lowering of the `switch` statement in `scheduleReducer`, or during reactive scope inference for the complex reducer pattern)
  3. Fix the root cause
- The diff.json shows `oxc_patterns.cacheSize: 0` which means no reactive scopes were created at all -- the pipeline likely bails out very early
- Check if `scheduleReducer` (a non-component function) is being compiled when it should be skipped, or if the `AvailabilitySchedule` component compilation fails on the `useReducer` pattern with a function reference argument

**Files involved:**
- `benchmarks/fixtures/realworld/availability-schedule.tsx` (input)
- `benchmarks/snapshots/availability-schedule.oxc.js` (current broken output)
- `benchmarks/snapshots/availability-schedule.babel.js` (expected output)
- `crates/oxc_react_compiler/src/build_hir/` (likely failure point)
- `crates/oxc_react_compiler/src/pipeline.rs` (compilation orchestration)

**Depends on:** None

---

## Gap 2: Phi-Node / Temporary Variable Resolution

**Upstream:** `src/ReactiveScopes/LeaveSSA.ts`, `src/Codegen/CodegenReactiveFunction.ts`
**Current state:** Unresolved temporaries appear in output for fixtures with ternary/logical expressions
**What's needed:**
- Audit `leave_ssa.rs` to verify all phi nodes are resolved to their source values
- Check the codegen path for `InstructionValue::Phi` -- if any phi instructions survive to codegen, they produce raw temporary names
- The upstream `LeaveSSA` pass:
  1. Walks all blocks in reverse postorder
  2. For each phi node, determines which incoming value "wins" based on control flow
  3. Replaces all uses of the phi with the winning value (or emits a `let` binding if the phi merges genuinely different values)
- Verify that ternary expressions (`ConditionalExpression`) correctly resolve both branches through the phi
- Verify that logical expressions (`LogicalExpression`) handle short-circuit evaluation phi nodes
- Add test cases for:
  - Simple ternary: `const x = cond ? a : b`
  - Nested ternary: `const x = c1 ? (c2 ? a : b) : c`
  - Logical AND: `const x = a && b`
  - Logical OR: `const x = a || b`
  - Nullish coalescing: `const x = a ?? b`

**Files involved:**
- `crates/oxc_react_compiler/src/leave_ssa.rs`
- `crates/oxc_react_compiler/src/codegen.rs`
- `crates/oxc_react_compiler/tests/` (new test cases)

**Depends on:** None

---

## Gap 3: JSX Hyphenated Attribute Names

**Upstream:** `src/Codegen/CodegenReactiveFunction.ts`
**Current state:** `aria-label` emitted as bare identifier instead of quoted string
**What's needed:**
- In the codegen JSX attribute emission path, check if the attribute name contains a hyphen
- If it does, emit it as a quoted string property: `"aria-label": value`
- Also handle `data-*` attributes the same way
- The upstream codegen uses `jsxAttributeName()` which checks for this pattern
- This is a small, targeted fix in `codegen.rs`

**Files involved:**
- `crates/oxc_react_compiler/src/codegen.rs` (JSX attribute name emission)
- `crates/oxc_react_compiler/tests/` (add test case with `aria-label` and `data-testid`)

**Depends on:** None

---

## Gap 4: Multi-Step-Form Timeout/Segfault

**Current state:** The multi-step-form fixture (L-tier, complex) may timeout or segfault during compilation
**What's needed:**
- Investigate whether `multi-step-form.tsx` compilation completes or hits a timeout/OOM/stack overflow
- If it's a stack overflow: increase stack size or convert recursive algorithms to iterative
- If it's a timeout: profile to find the hot loop (likely in fixpoint iteration for scope inference)
- If it produces truncated output like availability-schedule: same root cause investigation
- The benchmark diff shows `conservative_miss` with 80 fewer cache slots, suggesting it does compile but with significantly less memoization than Babel

**Files involved:**
- `benchmarks/fixtures/realworld/multi-step-form.tsx` (input)
- `benchmarks/snapshots/multi-step-form.diff.json` (current analysis)
- Various compiler passes depending on failure point

**Depends on:** None (can be investigated independently)

---

## Gap 5: Conservative Memoization Misses

**Current state:** Most fixtures show "conservative miss" -- OXC compiles correctly but uses fewer cache slots than Babel
**What's needed:**
- This is lower priority than the above gaps (conservative = safe but suboptimal)
- The pattern across all fixtures: OXC uses N fewer cache slots (ranging from 2 to 80 fewer)
- Root causes likely include:
  - Scope merging being too aggressive (merging scopes that Babel keeps separate)
  - Dependency analysis being too conservative (treating more values as non-reactive)
  - Missing scope creation for certain expression patterns
- After Gaps 1-4 are fixed, audit the scope inference passes against upstream:
  - `infer_reactive_scopes.rs` vs `src/ReactiveScopes/InferReactiveScopeVariables.ts`
  - `propagate_dependencies.rs` vs `src/ReactiveScopes/PropagateScopeDependencies.ts`
  - `prune_scopes.rs` vs `src/ReactiveScopes/PruneNonReactiveDependencies.ts`

**Files involved:**
- `crates/oxc_react_compiler/src/` (reactive scope inference passes)
- `benchmarks/snapshots/*.diff.json` (tracking improvement)

**Depends on:** Gaps 1-4 (fix bugs first, then optimize coverage)

---

## Gap 6: Test Infrastructure for Render Equivalence

**Current state:** Render comparison exists via `benchmarks/scripts/render-compare.mjs` but results are not in CI
**What's needed:**
- After fixing the above gaps, re-run render comparison and update the equivalence score
- Consider adding render equivalence as a CI check (non-blocking initially)
- Track the score over time: current 0.24 (6/25), target 0.80+ (20/25)
- Some fixtures will never match exactly due to conservative memoization differences -- that is acceptable as long as runtime behavior is equivalent

**Files involved:**
- `benchmarks/scripts/render-compare.mjs`
- `benchmarks/scripts/analyze-correctness.mjs`

**Depends on:** Gaps 1-5

---

## Acceptance Criteria

1. `availability-schedule` compiles completely (no truncation) with memoization patterns present
2. No unresolved temporary variables (`t25`, `t59`, etc.) in any benchmark fixture output
3. Hyphenated JSX attributes are quoted correctly in output
4. `multi-step-form` compiles without timeout or segfault
5. Render equivalence score improves from 0.24 to at least 0.60 (15/25 test cases)
6. No `semantic_difference / bug` classifications in any benchmark diff.json
