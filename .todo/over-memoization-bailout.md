# Over-Memoization Bail-Out Heuristics

> **Priority**: P3 (supports both upstream-error and compiled-no-memo categories)
> **Impact**: Cross-cutting -- validation accuracy affects ~50 remaining upstream-error + some compiled-no-memo fixtures
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
- Key areas: frozen mutation (8 remaining), exhaustive deps (2 remaining), scope reassignment (2 remaining), ref access (6), hook identity (2), setState (3), other (29 uncategorized)
**Depends on:** Gap 2 (completed)

### Gap 4: Zero-Scope Bail-Out ✅

~~**Upstream:** In `compileFn` in `CompilationPipeline.ts`, after scope construction, Babel checks if there are zero reactive scopes.~~

**Completed**: Zero-scope bail-out implemented. Functions with no reactive scopes return original source unchanged. Added +90 fixtures to conformance.

### Gap 5: Mutation Aliasing Bail-Out [IN PROGRESS]

**Upstream:** `InferMutationAliasingRanges.ts`, `InferReactivePlaces.ts` -- when values escape into unknown functions or global scope, Babel marks them as non-cacheable

**Completed milestones:**

1. **Graph-based BFS mutation propagation** (2026-03-14) ✅: `infer_mutation_aliasing_ranges.rs` rewritten from flat one-level alias traversal (~315 lines) to graph-based BFS algorithm (~715 lines) matching upstream `InferMutationAliasingRanges.ts`. Directed `AliasingGraph` with typed edges (Alias, Capture, MaybeAlias, CreatedFrom), BFS with temporal index guards, phi-node back-edge handling via `pending_phis` map.

2. **refine_effects() / applyEffect phase** (2026-03-14) ✅: `infer_mutation_aliasing_effects.rs` implements upstream `applyEffect()` logic. Apply effects resolved with and without function signatures. CreateFrom/Capture/Assign/MutateConditionally/Mutate effects refined based on value kinds via `AbstractHeap.value_kind()`. Function signature wiring complete -- effects are applied through `FunctionSignature` when available, falling back to conservative defaults. +15 fixtures (370 -> 385/1717).

3. **Pre-freeze infrastructure for component params** (2026-03-14) ✅: `pre_freeze_params()` in `infer_mutation_aliasing_effects.rs` seeds component parameters as frozen values in the abstract heap at function entry. This is groundwork for replacing the validator's name-based freeze tracking with aliasing-pass-derived freeze state. No immediate fixture gains, but enables future Gap 5b work.

**What remains:**
- **Full fixpoint abstract interpretation**: Upstream runs `InferMutableRanges` as a fixpoint loop, iterating until no new mutations are discovered. Our implementation is single-pass. Some fixtures require multiple iterations to propagate mutations through deep alias chains.
- **Function signature inference for user-defined functions**: When a user-defined function is called, upstream infers its signature from the function body (parameter effects, return type). Our implementation falls back to the conservative "mutate all arguments" path for functions without pre-existing signatures. Built-in/hook signatures are resolved correctly.
- **Return-value freezing**: Upstream freezes return values of certain function calls (e.g., hook returns) at the call site, preventing downstream mutations from being attributed to the caller's scope. Our freeze tracking is partial (see `validate_no_mutation_after_freeze.rs`).
- When aliasing analysis determines a function has no safely-cacheable values, the reactive scope construction should produce zero scopes, which triggers the zero-scope bail-out
- Implementation files: `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs`, `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_effects.rs`
**Depends on:** Gap 4 (completed)

### Gap 5b: False-Positive Validation Bail-Outs (~162 fixtures)

**Upstream:** Various validation passes in `babel-plugin-react-compiler/src/Validation/`
**Current state:** Name-based freeze tracking in `validate_no_mutation_after_freeze.rs` is too aggressive -- it freezes values that upstream's alias analysis (which uses the abstract heap and mutable ranges) would not freeze. Similarly, `validate_locals_not_reassigned_after_render.rs` and `validate_no_capitalized_calls.rs` produce false positives.
**What's needed:**
- **Frozen-mutation (~40% of false bail-outs):** The validator uses name-based tracking to determine frozen status. Upstream uses `InferMutableRanges` output (mutable range end < scope start = frozen). Fixing requires either: (a) wire the aliasing pass output (mutable ranges) into the validator, replacing name-based heuristics, or (b) make `refine_effects` output precise enough that downstream validators can query it directly.
- **Locals-reassigned-after-render (~22%):** Our validator flags reassignments that upstream allows because upstream's scope analysis is more precise about what constitutes "after render."
- **Capitalized-calls (~12%):** SSA resolution of aliased capitalized function names is incomplete; some non-capitalized calls are flagged as capitalized.
- **Other (~26%):** Various validator false positives across remaining validation passes.
- Key architectural insight: fixing frozen-mutation false positives requires coordinated changes across both `infer_mutation_aliasing_effects.rs` (to produce mutable-range data) and the validator (to consume it instead of name-based tracking).
**Depends on:** Gap 5 (mutation aliasing -- mutable range output needed)

### Gap 5c: Codegen Structure Divergences (~63 fixtures)

**Upstream:** `CodegenReactiveFunction.ts`
**Current state:** ~63 fixtures in known-failures have matching cache slot counts (`_c(N)` where N matches upstream) but the generated code within those slots differs in variable naming, scope structure, or declaration ordering.
**What's needed:**
- Audit codegen output for these fixtures to identify patterns (likely temp variable naming, scope nesting, or declaration ordering)
- May require improvements to temp inlining (Gap 1 in memoization-structure.md), scope structure in codegen, or declaration ordering logic
**Depends on:** Gap 1 (temp variable inlining), Gap 3 (cache slot alignment)

### Gap 6: "Too Simple" Function Detection

**Upstream:** Functions with no reactive inputs (no props/state/context usage) produce zero scopes naturally
**Current state:** May already work via zero-scope bail-out for many cases
**What's needed:**
- After Gaps 4+5 are implemented, check if "too simple" functions naturally produce zero scopes
- If not, investigate why -- may indicate over-eagerness in reactive place inference
- Common patterns: `function helper() { return 42; }`, `function format(x) { return x.toString(); }`
**Depends on:** Gap 4 (completed), Gap 5

### Gap 7: Remove Legacy `last_use_map` / `creation_map` After Upstream Pass Alignment

**Upstream:** `InferMutationAliasingRanges.ts` does not export `last_use_map` or `creation_map` -- these are artifacts of the previous flat implementation that leaked into downstream passes
**Current state:** The graph-based BFS rewrite (Gap 5) retained `last_use_map` and `creation_map` for backward compatibility because `infer_reactive_places.rs` and other downstream code depends on them. `last_use_map` records the last instruction ID at which each identifier is read; `creation_map` records the instruction ID where each identifier is first created. Both are computed as side-effects during the graph-building phase.
**What's needed:**
- Once `infer_reactive_places.rs` is rewritten to match upstream `InferReactivePlaces.ts`, the `last_use_map` dependency should be eliminated
- Once the mutable range output from the BFS algorithm is directly consumed by downstream passes (scope construction, pruning), `creation_map` can be removed
- This is a cleanup task, not a correctness fix -- the current code is correct but carries dead weight
**Depends on:** Gap 5 (in progress), upstream pass alignment for `InferReactivePlaces`

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
