# Over-Memoization Bail-Out Heuristics

> **Priority**: P3 (supports both upstream-error and compiled-no-memo categories)
> **Impact**: Cross-cutting -- validation accuracy affects 96 upstream-error + some compiled-no-memo fixtures
> **Tractability**: MODERATE -- requires line-by-line comparison with upstream validation passes

## Problem Statement

Our compiler adds `_c()` caching to functions where Babel returns the source
unchanged (no memoization at all). The root causes are captured in two other
.todo files:
- [upstream-errors.md](upstream-errors.md) -- 96 fixtures where Babel rejects with validation errors
- [compiled-no-memo.md](compiled-no-memo.md) -- 152 fixtures where Babel transforms without memoization

This file tracks the cross-cutting validation infrastructure that supports both.

## Architecture Overview

Babel's bail-out flow:
1. Function enters the pipeline
2. Validation passes run (hooks usage, ref access, set-state-in-render, etc.)
3. If any validation emits a `CompilerError` with severity `InvalidReact` or higher, the function is **skipped entirely** -- original source is returned
4. If validation passes, mutation analysis runs
5. If reactive scope analysis produces zero scopes (nothing reactive), the function is **skipped** -- original source is returned
6. Otherwise, memoized output is generated

Our current flow:
1. Function enters the pipeline
2. Validation passes run and bail on `AllErrors` threshold (implemented)
3. Compilation continues through all passes
4. If zero reactive scopes, return original source (implemented)
5. Otherwise, memoized output is always generated

## Implementation Plan

### Gap 1: Categorize Bail-Out Fixtures ✅

~~**Upstream:** Various validation passes in `babel-plugin-react-compiler/src/`~~

**Completed**: Bail-out fixtures categorized into multiple sub-categories. Triage done via conformance test analysis.

### Gap 2: Validation-Error Bail-Out Threshold ✅

~~**Upstream:** `CompilerError.ts` -- Babel has error severities~~

**Completed**: AllErrors threshold implemented. Pipeline bails on all validation errors, matching Babel's behavior. Added +24 fixtures to conformance.

### Gap 3: Ensure Validation Passes Emit Correct Errors

**Upstream:** Each validation pass in `babel-plugin-react-compiler/src/Validation/`
**Current state:** Our validation passes exist and have received significant SSA resolution improvements (see Completed Work below). However, they may not emit errors for all the same patterns Babel flags.
**What's needed:**
- Systematic audit of each validation pass against its upstream counterpart
- See [upstream-errors.md](upstream-errors.md) for the per-category breakdown of remaining gaps
- Key areas: frozen mutation (26), preserve-memo (13), exhaustive deps (8), scope reassignment (6), ref aliasing (6)
**Depends on:** Gap 2 (completed)

### Gap 4: Zero-Scope Bail-Out ✅

~~**Upstream:** In `compileFn` in `CompilationPipeline.ts`, after scope construction, Babel checks if there are zero reactive scopes.~~

**Completed**: Zero-scope bail-out implemented. Functions with no reactive scopes return original source unchanged. Added +90 fixtures to conformance.

### Gap 5: Mutation Aliasing Bail-Out

**Upstream:** `InferMutableRanges.ts`, `InferReactivePlaces.ts` -- when values escape into unknown functions or global scope, Babel marks them as non-cacheable
**Current state:** `infer_mutation_aliasing_effects.rs` and `infer_mutation_aliasing_ranges.rs` exist but may not track all escape paths that Babel tracks
**What's needed:**
- Audit `infer_mutation_aliasing_effects` against upstream `InferMutableRanges.ts`:
  - Verify that function calls with mutable arguments mark the arguments as potentially mutated
  - Verify that assignments to module-level variables are detected
  - Verify that values passed to non-local functions are marked as escaped
- When aliasing analysis determines a function has no safely-cacheable values, the reactive scope construction should produce zero scopes, which triggers the zero-scope bail-out
**Depends on:** Gap 4 (completed)

### Gap 6: "Too Simple" Function Detection

**Upstream:** Functions with no reactive inputs (no props/state/context usage) produce zero scopes naturally
**Current state:** May already work via zero-scope bail-out for many cases
**What's needed:**
- After Gaps 4+5 are implemented, check if "too simple" functions naturally produce zero scopes
- If not, investigate why -- may indicate over-eagerness in reactive place inference
- Common patterns: `function helper() { return 42; }`, `function format(x) { return x.toString(); }`
**Depends on:** Gap 4 (completed), Gap 5

## Completed Validation SSA Work (2026-03-13)

The following validation improvements have been made, resolving many bail-out fixtures:

- SSA name resolution in `validate_use_memo` (+3 fixtures)
- SSA name resolution in `validate_no_set_state_in_render` (+9 fixtures)
- SSA name resolution in `validate_no_ref_access_in_render` (+15 fixtures)
- PropertyStore/PropertyLoad ref tracking (+6 fixtures)
- setState detection in useMemo callbacks (+2 fixtures)
- SSA resolution in `validate_no_impure_functions_in_render` + performance.now() (+2 fixtures)
- SSA resolution in `validate_no_derived_computations_in_effects` (+1 fixture)
- SSA resolution in `validate_exhaustive_dependencies` (correctness, no new fixtures)
- SSA resolution in `validate_no_set_state_in_effects` (correctness)
- SSA resolution in `validate_no_capitalized_calls` (+3 fixtures)
- Conditional hook method calls (+3 fixtures)
- Global hook names in SSA for conditional hook detection (+8 fixtures)
- Hooks-as-values validation (+9 fixtures)
- `validate_no_global_reassignment` pass (new)

## Risks and Notes

- **False negatives**: If we bail out too aggressively, we'll skip compiling functions that Babel does compile, creating under-memoization divergences. Need to match Babel's exact bail-out conditions.
- **Error severity mapping**: We must match v1.0.0 behavior specifically, not the latest main branch.
- **Interaction with structural fixes**: Some fixtures in the bail-out categories may also have structural differences. Fixing bail-out is strategic because it removes fixtures from comparison entirely (original source = original source, always matches).
