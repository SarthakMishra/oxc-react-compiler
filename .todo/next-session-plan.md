# Next Session Plan

> Written: 2026-03-20 (post Phase 106). Self-contained -- no prior session context needed.

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
.journal/                           -- implementation history (106 phases)
```

### Current Metrics (2026-03-20, post Phase 106)

| Metric | Value |
|--------|-------|
| Render equivalence | 96% (24/25 benchmark fixtures produce correct HTML) |
| Conformance | 445/1717 (25.9% of upstream test fixtures match exactly) |
| E2E transform coverage | 95-100% across 4 real-world Vite projects |
| Rust tests | 196 passing, 0 panics on all 1717 fixtures |

### Conformance Breakdown (445 passing, 1272 failing)

| Category | Count | Description |
|----------|-------|-------------|
| both compile, slots DIFFER | ~622 | Scope/memoization divergence (biggest bucket) |
| both compile, slots MATCH | ~240 | Output format only (easiest to fix) |
| we bail, they compile | ~174 | False bail-outs |
| we compile, they don't | ~147 | We over-compile (usually fine) |
| both no memo, format diff | ~89 | Format-only |
| silent bail-outs | 28 | Compile but 0 scopes, no error |

---

## CRITICAL ARCHITECTURE NOTES (READ BEFORE MAKING ANY CHANGES)

### 1. `effective_range` vs `mutable_range` -- DO NOT CHANGE

File: `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`

Scope grouping uses `effective_range = max(mutable_range.end, last_use + 1)` instead of upstream's pure `mutable_range`. This compensates for our BFS mutation propagation producing narrower ranges than upstream's full abstract interpreter.

**Switching to narrow `mutable_range` has been tried 4 TIMES and ALWAYS causes render to drop from 96% to ~36%.** DO NOT attempt narrowing ranges without first porting upstream's full abstract interpreter state machine.

### 2. `collect_all_scope_declarations` is load-bearing

File: `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`

This function pre-declares ALL scope output variables at function level. Removing it causes render to drop from 96% to 24%.

### 3. Validation relaxation causes regressions

Attempted relaxing 3 different validators. ALL caused conformance drops because we compile programs incorrectly without proper scope inference. DO NOT relax validation without fixing scope inference first.

### 4. Build and test commands

```bash
cargo test                          # All Rust tests (196 tests)
cargo test --test conformance_tests -- --nocapture  # Conformance (1717 fixtures)
cargo insta test --accept           # Update snapshots after codegen changes
cd napi/react-compiler && npx @napi-rs/cli build --release  # Rebuild NAPI
cd benchmarks && npm run render:compare   # Render comparison
cd benchmarks && npm run e2e:quick        # E2E Vite builds
cargo fmt && cargo clippy           # Format check (pre-commit hook)
```

---

## Priority 1: Fix Non-Render Function Detection (~40 fixtures)

**Impact:** +26 to +40 conformance
**Risk:** LOW-MEDIUM
**Files:** See detailed plan in `.todo/validation-gaps.md` under "Shared Root Cause"

### The Problem

Both `validate_no_ref_access_in_render.rs` (14 false positives) and `validate_locals_not_reassigned_after_render.rs` (26 false positives) need to determine whether a FunctionExpression executes during render or after. The current detection is broken — `render_only_fns` is always empty.

### The Fix

Create a shared `validation/function_context.rs` with `collect_post_render_fn_ids()`:

1. **Invert the approach**: identify post-render functions (passed to hooks, JSX event props, returned) instead of trying to identify render-only ones
2. **Seed from known post-render sites**: any `use*` hook argument, JSX `onXxx`/`ref` props, return values
3. **Alias propagation**: follow LoadLocal/StoreLocal chains to resolve FE IDs
4. **Transitive fixpoint**: if post-render FE A calls named FE B, B is also post-render

Then update both validators:
- **Reassignment**: Only flag FEs in `post_render_ids` (invert current "flag everything except render-only")
- **Ref access**: Skip FEs in `post_render_ids` (same semantics, wider set)

### Implementation Steps

1. Create `crates/oxc_react_compiler/src/validation/function_context.rs`
2. Add `pub mod function_context;` to `validation/mod.rs`
3. Rewrite `validate_locals_not_reassigned_after_render.rs` to use shared utility
4. Update `validate_no_ref_access_in_render.rs` to use shared utility
5. Test: `cargo test --test conformance_tests -- --nocapture`
6. Update `known-failures.txt`

### Key Design Decisions

- Non-hook utility functions (`invoke`, `foo`) are NOT post-render — upstream treats them as synchronous render-time callers
- ALL `use*` hook arguments are post-render (not just `useEffect`)
- Return values escape and are post-render
- Async functions remain handled separately (always post-render)

### Testing

Run conformance and check:
- The 26 reassignment fixtures should now pass (or at least most of them)
- The 14 ref-access fixtures should now pass (or at least some)
- No `error.*` fixtures should newly fail (or at most 1-2 acceptable regressions)

---

## Priority 2: Canvas-Sidebar Render Divergence

**Impact:** 96% → 100% render equivalence
**Risk:** LOW

**Current state:** 1/25 benchmark fixtures shows render divergence. Phase 101 notes "9 undeclared temps" as the symptom.

**Investigation:**
1. `cd benchmarks && npm run render:compare-verbose` (see exact HTML diff)
2. `cd benchmarks && npm run babel:diff` (see compiled code diff)
3. Look at `benchmarks/fixtures/canvas-sidebar/` for source
4. Fix declaration collection in `codegen.rs`

---

## Priority 3: Named Variable Preservation (~56 remaining fixtures)

**Impact:** Up to +56 conformance
**Risk:** MEDIUM

Phase 106 fixed 8 fixtures via `is_last_assignment_in_scope` in `prune_scopes.rs`. The remaining ~56 fixtures have temp names from codegen inlining of function expressions and intermediate values. Requires broader study of upstream's `CodegenReactiveFunction.ts` naming logic.

---

## Priority 4: Optional Chaining (15 fixtures)

**Impact:** +15 conformance
**Risk:** MEDIUM (structural HIR change)

Add `optional: bool` to `CallExpression`, `MethodCall`, `PropertyLoad`, `ComputedLoad` in HIR types. Propagate from OXC AST during lowering. Use in codegen.

---

## Do NOT Attempt

- Scope inference changes (Gap 11: under-memoization, Gap 7: over-memoization)
- Validation relaxation (Gap 5a: memoization preservation)
- Narrowing `mutable_range`
- These require porting upstream's full abstract interpreter and have a proven track record of causing severe regressions.

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
| Frozen value validation | `crates/oxc_react_compiler/src/validation/validate_frozen_values.rs` |
| Ref access validation | `crates/oxc_react_compiler/src/validation/validate_no_ref_access_in_render.rs` |
| Reassignment validation | `crates/oxc_react_compiler/src/validation/validate_locals_not_reassigned_after_render.rs` |
| Conformance test runner | `crates/oxc_react_compiler/tests/conformance_tests.rs` |
| Known failures | `tests/conformance/known-failures.txt` |
