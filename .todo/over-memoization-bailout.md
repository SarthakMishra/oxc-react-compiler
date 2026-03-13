# Over-Memoization Bail-Out Heuristics

> **Priority**: MEDIUM-HIGH (largest category -- ~710 fixtures, but hardest to implement)
> **Impact**: ~710 divergences where we memoize but Babel returns source unchanged

## Problem Statement

Our compiler adds `_c()` caching to ~710 fixtures where Babel v1.0.0 returns the source unchanged (no memoization at all). The root cause: Babel has bail-out heuristics in its pipeline that cause it to skip memoization when it detects patterns that cannot be safely cached or are too simple to benefit. Our compiler compiles everything it can reach, adding memoization unconditionally.

Babel bails out of memoization when it detects:
- **Mutation aliasing**: values that escape into unknown functions, making it unsafe to cache
- **Global/module-level assignments**: writes to variables outside the function scope
- **Ref access patterns**: `.current` access on refs that can't be safely cached
- **Too-simple functions**: functions with no reactive dependencies (nothing to memoize)
- **Hook usage patterns**: certain hook configurations that indicate bail-out
- **Validation errors**: when validation passes detect patterns that make compilation unsafe, Babel emits a diagnostic and returns the original source

The key distinction: when Babel's validation passes emit an error, the **entire function** is skipped (source returned unchanged). Our compiler currently collects errors but still attempts to compile and emit memoized output for many of these cases.

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
2. Validation passes run but only bail on `CriticalErrors` (invariant violations)
3. Compilation continues through all passes
4. Memoized output is always generated if the pipeline completes

## Files to Modify

### Error-based bail-out
- **`crates/oxc_react_compiler/src/error.rs`** -- review error severity levels and bail thresholds
- **`crates/oxc_react_compiler/src/entrypoint/pipeline.rs`** -- add bail-out checks after validation phases
- **`crates/oxc_react_compiler/src/entrypoint/program.rs`** -- handle pipeline bail-out by returning original source

### Zero-scope bail-out
- **`crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`** -- after scope pruning, detect zero-scope case
- **`crates/oxc_react_compiler/src/entrypoint/pipeline.rs`** -- propagate zero-scope signal

### Validation pass accuracy
- **`crates/oxc_react_compiler/src/validation/*.rs`** -- ensure each validation pass emits errors at the correct severity matching Babel's `CompilerError` levels
- **`crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_effects.rs`** -- mutation escape analysis accuracy
- **`crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs`** -- mutable range tracking accuracy

## Implementation Plan

### Gap 1: Categorize Bail-Out Fixtures ✅

~~**Upstream:** Various validation passes in `babel-plugin-react-compiler/src/`~~

**Completed**: Bail-out fixtures categorized into 1121 both-compile-diff, 219 babel-transforms-no-memo, and 10 our-bail-should-match. Triage done via conformance test analysis.

### Gap 2: Validation-Error Bail-Out Threshold ✅

~~**Upstream:** `CompilerError.ts` -- Babel has error severities~~

**Completed**: AllErrors threshold implemented. Pipeline bails on all validation errors, matching Babel's behavior. Added +24 fixtures to conformance.

### Gap 3: Ensure Validation Passes Emit Correct Errors

**Upstream:** Each validation pass in `babel-plugin-react-compiler/src/Validation/`
**Current state:** Our validation passes exist but may not emit errors for all the same patterns Babel flags, or may emit at wrong severity
**What's needed:**
- Audit each validation pass against its upstream counterpart:
  - `validate_hooks_usage` vs `ValidateHooksUsage.ts` -- verify all hook rule violations are caught
  - `validate_no_ref_access_in_render` vs `ValidateNoRefAccessInRender.ts` -- verify ref.current access detection
  - `validate_no_set_state_in_render` vs `ValidateNoSetStateInRender.ts`
  - `validate_no_set_state_in_effects` vs `ValidateNoSetStateInEffects.ts`
  - `validate_no_jsx_in_try` vs `ValidateNoJSXInTryStatement.ts`
  - `validate_locals_not_reassigned_after_render` vs `ValidateLocalsNotReassignedAfterRender.ts`
  - `validate_no_impure_functions_in_render` vs `ValidateNoImpureFunctionsInRender.ts`
- For each pass, verify: (a) same patterns detected, (b) same error severity emitted
- Fix any gaps -- missing pattern detection or wrong severity
**Depends on:** Gap 2 (need the error severity infrastructure first)

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
- Audit `infer_reactive_places` against upstream `InferReactivePlaces.ts`:
  - Verify that escaped values are not marked as reactive (they can't be safely cached)
- When aliasing analysis determines a function has no safely-cacheable values, the reactive scope construction should produce zero scopes, which triggers Gap 4's bail-out
**Depends on:** Gap 4 (the zero-scope bail-out mechanism)

### Gap 6: "Too Simple" Function Detection

**Upstream:** Functions with no reactive inputs (no props/state/context usage) produce zero scopes naturally
**Current state:** Unknown -- may already work via zero-scope bail-out
**What's needed:**
- After Gap 4 is implemented, check if "too simple" functions naturally produce zero scopes
- If not, investigate why -- it may indicate over-eagerness in reactive place inference
- Common patterns: `function helper() { return 42; }`, `function format(x) { return x.toString(); }`
- These should have no reactive scopes because they don't read from reactive sources
**Depends on:** Gap 4

## Measurement Strategy

After each gap, run conformance and measure:
```bash
cargo test conformance -- --nocapture 2>&1 | tail -5
```

Expected progression:
- After Gap 1 (categorization): no pass rate change (diagnostic only)
- After Gap 2 (bail-out threshold): ~100-200 new passes (fixtures where we already emit the right errors but don't bail)
- After Gap 3 (validation accuracy): ~100-200 additional passes (fixtures where we missed errors)
- After Gap 4 (zero-scope bail-out): ~100-150 additional passes (trivial functions)
- After Gap 5 (mutation aliasing): ~100-150 additional passes (escaped values)
- After Gap 6 (too simple): likely included in Gap 4's numbers
- Total from this category: ~400-600 new passes (some fixtures have overlapping issues)

Note: the full ~710 may not all be achievable because some fixtures may require very precise upstream-matching behavior that takes significant effort per-fixture.

## Risks and Notes

- **False negatives**: If we bail out too aggressively, we'll skip compiling functions that Babel does compile, creating a new category of divergences (under-memoization). Need to be precise about matching Babel's exact bail-out conditions.
- **Error severity mapping**: Babel's error severities have evolved across versions. We need to match v1.0.0 behavior specifically, not the latest main branch.
- **Validation pass completeness**: Some validation passes may have subtle gaps that cause us to miss errors Babel catches. This is the hardest part -- it requires careful line-by-line comparison with upstream TypeScript.
- **Interaction with Category 2**: Some fixtures in the 710 may also have structural differences if we did compile them. Fixing bail-out first is strategic because it removes fixtures from the comparison entirely (original source = original source, always matches).
- **Order of operations**: This category should be worked on AFTER TS stripping (Category 3) and ideally after some of the temp inlining work (Category 2, Gap 1), because the pass rate improvements compound -- TS stripping removes 138 false divergences, making it easier to see the real bail-out issues.
