# Next Session Plan

> Written: 2026-03-20. Self-contained -- no prior session context needed.

## Project Overview

**oxc-react-compiler** is a native Rust port of Meta's `babel-plugin-react-compiler` using OXC. It analyzes React components/hooks, infers reactive dependencies, and wraps computations in memoization scopes using `useMemoCache`. It runs as a Vite plugin via NAPI bindings.

### Repository Layout

```
crates/oxc_react_compiler/          -- main compiler crate
  src/entrypoint/pipeline.rs        -- 60+ pass pipeline orchestration
  src/entrypoint/program.rs         -- compile_program entry point, function discovery
  src/hir/types.rs                  -- all HIR data structures (~1000 lines)
  src/hir/build.rs                  -- OXC AST -> HIR lowering (~2500 lines)
  src/hir/environment.rs            -- EnvironmentConfig (~20 flags)
  src/hir/object_shape.rs           -- ShapeRegistry, FunctionSignature
  src/hir/globals.rs                -- register_globals() for std library shapes
  src/ssa/                          -- SSA construction + phi elimination
  src/optimization/                 -- constant prop, DCE, block merge, etc.
  src/inference/
    aliasing_effects.rs             -- per-instruction effect computation
    infer_mutation_aliasing_effects.rs -- abstract interpreter + effect refinement
    infer_mutation_aliasing_ranges.rs  -- BFS mutation range propagation + annotate_last_use
    infer_reactive_places.rs        -- fixpoint reactivity propagation
    infer_types.rs                  -- forward type inference
  src/reactive_scopes/
    infer_reactive_scope_variables.rs -- scope grouping (union-find + effective_range)
    propagate_dependencies.rs       -- scope dependency/declaration computation
    build_reactive_function.rs      -- HIR CFG -> ReactiveFunction tree
    prune_scopes.rs                 -- 14 pruning sub-passes
    codegen.rs                      -- JavaScript code generation (~3000 lines)
  src/validation/                   -- 15+ validation passes
  tests/conformance_tests.rs        -- upstream fixture conformance runner

crates/oxc_react_compiler_lint/     -- standalone lint rules
napi/react-compiler/                -- Node.js NAPI binding + Vite plugin
tests/conformance/                  -- 1717 upstream test fixtures
benchmarks/                         -- render comparison, babel diff, E2E builds
.todo/                              -- backlog and gap tracking
.journal/                           -- implementation history (100+ phases)
```

### Current Metrics (2026-03-20)

| Metric | Value |
|--------|-------|
| Render equivalence | 96% (24/25 benchmark fixtures produce correct HTML) |
| Conformance | 437/1717 (25.4% of upstream test fixtures match exactly) |
| E2E transform coverage | 95-100% across 4 real-world Vite projects |
| Rust tests | 196 passing, 0 panics on all 1717 fixtures |
| Correctness score | 93.8% |

### Conformance Breakdown (437 passing, 1280 failing)

| Category | Count | Description |
|----------|-------|-------------|
| both compile, slots DIFFER | ~621 | Scope/memoization divergence (biggest bucket) |
| both compile, slots MATCH | ~242 | Output format only (easiest to fix) |
| we bail, they compile | ~170 | False bail-outs |
| we compile, they don't | ~138 | We over-compile (usually fine) |
| both no memo, format diff | ~93 | Format-only |
| silent bail-outs | 28 | Compile but 0 scopes, no error |

---

## CRITICAL ARCHITECTURE NOTES (READ BEFORE MAKING ANY CHANGES)

### 1. `effective_range` vs `mutable_range` -- DO NOT CHANGE

File: `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`

Scope grouping uses `effective_range = max(mutable_range.end, last_use + 1)` instead of upstream's pure `mutable_range`. This compensates for our BFS mutation propagation producing narrower ranges than upstream's full abstract interpreter.

**Switching to narrow `mutable_range` has been tried 4 TIMES and ALWAYS causes render to drop from 96% to ~36%.** Each attempt used different compensating passes: (1) none, (2) PropagateScopeMembership, (3) PropagateScopeMembership + JSX Capture edges, (4) full aliasing effect pipeline Steps 1-6. The root cause: upstream's abstract interpreter produces wider mutation ranges through more complete state tracking. Our `effective_range` approximation compensates.

DO NOT attempt narrowing ranges without first porting upstream's full abstract interpreter state machine from `src/Inference/InferMutationAliasingEffects.ts`.

### 2. `last_use` field on Identifier

File: `crates/oxc_react_compiler/src/hir/types.rs`

A separate `last_use: InstructionId` field tracks usage reach independently from mutation reach. Used by the `effective_range` computation. Added in Phase 97.

### 3. Validation relaxation causes regressions

Attempted relaxing 3 different validators (memoization preservation, frozen mutation, reassigned-after-render). ALL caused conformance drops because we compile programs incorrectly without proper scope inference. DO NOT relax validation without fixing scope inference first.

### 4. `collect_all_scope_declarations` is load-bearing

File: `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`

This function pre-declares ALL scope output variables at function level. Removing it causes render to drop from 96% to 24%. It was removed once by accident and had to be reverted.

### 5. Build and test commands

```bash
# Run all Rust tests (196 tests)
cargo test

# Run conformance tests only (1717 fixtures)
cargo test conformance

# Update snapshot tests after codegen changes
cargo insta test --accept

# Rebuild NAPI after Rust changes (REQUIRED for benchmark/E2E tests)
cd napi/react-compiler && npx @napi-rs/cli build --release

# Run render comparison (requires NAPI rebuild first)
cd benchmarks && npm run render:compare

# Run E2E Vite builds
cd benchmarks && npm run e2e

# Run correctness analysis
cd benchmarks && npm run correctness

# Run Babel differential comparison
cd benchmarks && npm run babel:diff

# Format check (pre-commit hook runs this)
cargo fmt
cargo clippy
```

---

## Priority 1: Production Readiness

### What's needed for real-world use

The compiler already works in production-like scenarios (95-100% E2E transform coverage across 4 Vite projects, 96% render equivalence). The main production readiness gap is the **render divergence in canvas-sidebar** (1/25 fixtures).

### Task 1A: Fix the remaining render divergence (canvas-sidebar)

**Files:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
**Impact:** 96% -> 100% render equivalence
**Risk:** LOW -- this is a single fixture with known symptoms

**Current state:** The canvas-sidebar fixture renders but has minor content differences. Phase 101 journal notes "9 undeclared temps" as the symptom -- cross-block scope dependency references aren't being properly declared.

**How to investigate:**
1. Run `cd benchmarks && npm run render:compare-verbose` to see the exact HTML diff
2. Run `cd benchmarks && npm run babel:diff` to see the compiled code diff between Babel and OXC for the canvas-sidebar fixture
3. Look at `benchmarks/fixtures/canvas-sidebar/` for the source component
4. Compare OXC compiled output vs Babel compiled output to find scope/declaration divergences
5. The issue is likely in `codegen.rs` -- specifically in how `collect_all_scope_declarations` handles cross-scope variable references

**Testing:** `cd benchmarks && npm run render:compare`

### Task 1B: Verify E2E builds still pass

After any changes, run: `cd benchmarks && npm run e2e:quick`

This tests 4 real Vite projects (each configured with the oxc-react-compiler Vite plugin). All should build and serve correctly.

---

## Priority 2: Conformance Growth (Safe Improvements)

These are improvements that can safely push the 437 conformance count higher without touching scope inference or validation relaxation. Ordered by expected impact.

### Task 2A: Named variable preservation (~80+ fixtures)

**Files:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
**Impact:** Up to +80 conformance (these are "both compile, slots MATCH" fixtures that differ only in variable naming)
**Risk:** MEDIUM -- changes to codegen can affect all output
**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`

**Current state:** Our codegen assigns temporary names (`t0`, `t1`, ...) to intermediate values. Upstream preserves original variable names from the source when possible (e.g., `const x = ...` instead of `const t0 = ...`).

**What's needed:**
- Study how upstream `CodegenReactiveFunction.ts` decides when to use original names vs temps
- Carry the original identifier name through the reactive scope tree (it may already be available on `Identifier.name`)
- In codegen, when emitting a declaration for a value that has an original source name, use that name instead of a temp
- The conformance test normalizer already handles temp renaming (`t0` -> `t0`, `t1` -> `t1`), so the comparison should be stable

**Gotcha:** The per-function temp normalization in `conformance_tests.rs` resets temp counters at `function` boundaries. Named variables would bypass this normalization entirely, which is the desired behavior.

**Testing:** `cargo test conformance` -- look for fixtures that move from "slots MATCH, format diff" to "PASS"

### Task 2B: Silent bail-outs (28 fixtures)

**Files:** Various -- depends on the bail-out cause
**Impact:** Up to +28 conformance
**Risk:** LOW -- these fixtures currently produce 0 scopes, so any fix is strictly additive

**Current state:** 28 fixtures compile but produce 0 reactive scopes and no error, while upstream compiles them successfully with memoization.

**How to investigate:**
1. Run `cargo test conformance` and look for fixtures marked "silent bail" in the output
2. For each, check what pattern causes 0 scopes -- common causes:
   - HIR lowering gap (unsupported AST node type)
   - Function discovery miss (not recognized as component/hook)
   - Early bail-out in pipeline due to missing feature
3. Group by root cause and fix the most common category first

**Testing:** `cargo test conformance`

### Task 2C: False "ref access in render" errors (14 fixtures)

**Files:** `crates/oxc_react_compiler/src/validation/validate_no_ref_access_in_render.rs`
**Impact:** Up to +14 conformance
**Risk:** LOW -- validation-only, won't affect correct compilations
**Upstream:** `src/Validation/ValidateNoRefAccessInRender.ts`

**What's needed:**
- Audit whether we correctly identify effect/callback contexts where ref access IS allowed
- Upstream allows `ref.current` inside `useEffect`/`useLayoutEffect` callbacks, event handlers, and other non-render contexts
- Compare our detection logic against upstream's

**Testing:** `cargo test conformance`

### Task 2D: False "reassigned after render" errors (16 fixtures)

**Files:** `crates/oxc_react_compiler/src/validation/validate_locals_not_reassigned_after_render.rs`
**Impact:** Up to +16 conformance
**Risk:** LOW -- validation-only
**Upstream:** `src/Validation/ValidateLocalsNotReassignedAfterRender.ts` (or related)

**What's needed:**
- Check if reassignment inside callbacks/effects is correctly allowed
- Audit against upstream validation logic

**Testing:** `cargo test conformance`

### Task 2E: False "frozen mutation" errors (~20 fixtures)

**Files:** `crates/oxc_react_compiler/src/validation/validate_frozen_values.rs`
**Impact:** Up to +20 conformance
**Risk:** LOW-MEDIUM -- recent work (SSA-versioned keys, IIFE/PrefixUpdate/hook lambda exemptions) already fixed many cases
**Upstream:** `src/Validation/ValidateFrozenValues.ts`

**What's needed:**
- Audit remaining false positives against upstream logic
- Check if mutable range computation is still too narrow for remaining cases
- Each false positive should be investigated individually to understand why our analysis disagrees with upstream

**Testing:** `cargo test conformance`

### Task 2F: Other false bail-outs (28 fixtures)

**Files:** Various validation passes
**Impact:** Up to +28 conformance
**Risk:** LOW -- each is a separate validation fix

**Breakdown from `.todo/validation-gaps.md`:**
- 8x "Cannot reassign variables declared outside of the component"
- 6x "Local variable y is assigned during render but reassigned"
- 3x "Hooks may not be referenced as normal values"
- 2x "Cannot call setState during render"
- 2x "setState is called directly inside useEffect"
- 7x other

**Testing:** `cargo test conformance`

---

## Priority 3: The Remaining Render Divergence (canvas-sidebar)

This overlaps with Task 1A above. Additional context:

**File:** `benchmarks/fixtures/canvas-sidebar/` (the source fixture)
**Symptom:** Phase 101 journal mentions "9 undeclared temps" -- scope dependency references that cross block boundaries aren't being properly hoisted/declared.
**Root cause hypothesis:** The `collect_all_scope_declarations` function in `codegen.rs` may miss variables that are used as scope dependencies but defined in a different scope or block. The function currently pre-declares ALL scope output variables at function level, but scope DEPENDENCY variables (used in `$[N] !== dep` checks) may not be included.

**Investigation approach:**
1. Extract the canvas-sidebar compiled output from both Babel and OXC
2. Diff them to find the specific variables that are undeclared
3. Trace back through the ReactiveFunction tree to find where those variables should have been declared
4. Fix the declaration collection in `codegen.rs`

---

## Priority 4: Future Architecture Work

### The Foundational Blocker: Under-memoization (404 fixtures)

**Files:**
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`
- `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_effects.rs`
- `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs`

**Impact:** Would fix ~404 under-memoized fixtures and unblock validation relaxation (another ~58 fixtures)
**Risk:** VERY HIGH -- this is the core scope inference mechanism

This is documented extensively in `.todo/scope-inference.md` (Gap 11). The root cause is that our BFS mutation propagation produces narrower ranges than upstream's full abstract interpreter. We compensate with `effective_range = max(mutable_range.end, last_use + 1)`, but this is an approximation that still leaves 404 fixtures under-memoized.

**What would be needed (NOT recommended for next session):**
1. Port upstream's full abstract interpreter state machine from `src/Inference/InferMutationAliasingEffects.ts`
2. This is a ~2000-line TypeScript file with complex state tracking (value kinds, aliasing graphs, effect refinement)
3. Only after the abstract interpreter produces sufficiently wide ranges can we switch from `effective_range` to `mutable_range`
4. This change would also unblock validation relaxation (Gap 5a: +58 fixtures)

**Prerequisites for attempting:**
- Deep understanding of upstream's `InferMutationAliasingEffects.ts` state machine
- Ability to A/B test with render comparison (`cd benchmarks && npm run render:compare`)
- Willingness to revert if render drops below 96%

### Over-memoization (175 fixtures)

**Files:**
- `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`

**Impact:** Would fix ~175 over-memoized fixtures
**Risk:** MEDIUM -- may self-resolve as side effect of under-memoization fix

This may partially resolve itself once under-memoization is fixed. Investigate AFTER the foundational scope inference work is done. Compare our scope merging and pruning against:
- `src/ReactiveScopes/MergeReactiveScopesThatInvalidateTogether.ts`
- `src/ReactiveScopes/PruneNonEscapingScopes.ts`

---

## Recommended Session Strategy

**Start with Task 2A (named variable preservation)** -- it has the highest expected conformance impact (~80 fixtures) with moderate risk and no dependency on scope inference work. The changes are localized to `codegen.rs` and the effect is purely cosmetic (correct variable names instead of temps).

**Then move to Task 2C + 2D (false ref-access and reassignment errors)** -- these are independent validation fixes worth up to +30 conformance with low risk.

**If time remains, investigate Task 1A (canvas-sidebar render divergence)** -- this is the last render correctness gap.

**Do NOT attempt:** Scope inference changes (Priority 4), validation relaxation (Gap 5a), or narrowing `mutable_range`. These require the full abstract interpreter port and have a proven track record of causing severe regressions.

---

## Quick Reference: Key File Paths

| Purpose | Path |
|---------|------|
| Pipeline orchestration | `crates/oxc_react_compiler/src/entrypoint/pipeline.rs` |
| Code generation | `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` |
| Scope grouping | `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs` |
| Scope dependencies | `crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs` |
| ReactiveFunction builder | `crates/oxc_react_compiler/src/reactive_scopes/build_reactive_function.rs` |
| Scope pruning | `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs` |
| Aliasing effects | `crates/oxc_react_compiler/src/inference/aliasing_effects.rs` |
| Effect refinement | `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_effects.rs` |
| Mutation ranges | `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs` |
| Frozen value validation | `crates/oxc_react_compiler/src/validation/validate_frozen_values.rs` |
| Ref access validation | `crates/oxc_react_compiler/src/validation/validate_no_ref_access_in_render.rs` |
| Reassignment validation | `crates/oxc_react_compiler/src/validation/validate_locals_not_reassigned_after_render.rs` |
| Memoization validation | `crates/oxc_react_compiler/src/validation/validate_preserved_manual_memoization.rs` |
| Conformance test runner | `crates/oxc_react_compiler/tests/conformance_tests.rs` |
| Known failures | `tests/conformance/known-failures.txt` |
| Backlog index | `.todo/index.md` |
| Scope inference gaps | `.todo/scope-inference.md` |
| Validation gaps | `.todo/validation-gaps.md` |
| Codegen gaps | `.todo/codegen-emission.md` |
| Journal (latest) | `.journal/001.md` (100+ phases, read from top for most recent) |
