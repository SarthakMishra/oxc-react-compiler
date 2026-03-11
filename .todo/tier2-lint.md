# Tier 2 Lint Rules

> Lint rules that require the full compiler pipeline (HIR, effect system, reactive scopes)
> to detect issues that cannot be found with AST-level analysis alone.

**Current state:** All 5 rules are stubbed in `crates/oxc_react_compiler_lint/src/rules/tier2.rs`. The `_with_source` variants call `run_lint_analysis` which runs the pipeline, and filtering now uses structured `DiagnosticKind` categories instead of string matching (Gap 6 completed). The `_program`-only variants return empty vectors.

---

### Gap 1: check_hooks_tier2 -- Full Rules of Hooks with CFG Analysis

**Upstream:** `compiler/packages/eslint-plugin-react-compiler/src/rules/ReactCompilerRule.ts` (uses compiler's `validateHooksUsage`)
**Current state:** The `check_hooks_tier2` function returns `Vec::new()`. The `_with_source` variant does string filtering on "hook" in diagnostic messages.
**What's needed:**
- The pipeline's `validate_hooks_usage` pass already runs during `run_lint_pipeline` and produces diagnostics
- Wire up structured error categories from `ErrorCollector` instead of string matching
- The `_program`-only API cannot work without re-parsing; either remove it or document that `_with_source` is the primary API
- Add test cases: hooks in conditionals, hooks in loops, hooks after early returns, hooks in nested functions
**Depends on:** None

### Gap 2: check_immutability -- Mutation of Frozen Values

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/Validation/ValidateNoImpurityDuringRender.ts`
**Current state:** Returns `Vec::new()`. The effect system tracks mutations, but the validation pass may not emit structured diagnostics for "mutation of frozen value" specifically.
**What's needed:**
- Verify that the pipeline's validation passes emit diagnostics for mutating frozen/immutable values
- If not, add a validation pass that checks the effect system's mutation records against the freeze points
- Wire the diagnostics through `run_lint_pipeline` with a specific `ErrorCategory`
- Filter by category in `check_immutability_with_source` instead of string matching
- Test cases: mutating a prop, mutating a value after it's been captured by a reactive scope, mutating a ref during render
**Depends on:** None

### Gap 3: check_preserve_manual_memoization

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/Validation/ValidatePreservedManualMemoization.ts`
**Current state:** Returns `Vec::new()`.
**What's needed:**
- Port `ValidatePreservedManualMemoization` logic: verify that the compiler's automatic memoization is at least as good as manual `useMemo`/`useCallback`
- This checks that every manually memoized value still has a reactive scope in the compiler output
- If the compiler would drop a manual memoization (e.g., the value is not reactive), emit a diagnostic
- Requires the reactive scope analysis to be complete before this validation runs
- Test cases: `useMemo` with non-reactive deps, `useCallback` that the compiler prunes, nested memoization
**Depends on:** None

### Gap 4: check_memo_dependencies -- Exhaustive useMemo/useCallback Deps

**Upstream:** `compiler/packages/eslint-plugin-react-compiler/src/rules/ReactCompilerRule.ts` (exhaustive deps mode)
**Current state:** Returns `Vec::new()`.
**What's needed:**
- Use the compiler's dependency analysis (from `propagate_scope_dependencies`) to determine the correct dependency set for each `useMemo`/`useCallback`
- Compare against the user-provided dependency array
- Emit diagnostics for missing or extraneous dependencies
- Include autofix suggestions (the correct dependency array) in the diagnostic
- Test cases: missing dep, extra dep, computed dep (obj.prop), stable deps (setState), ref deps
**Depends on:** None

### Gap 5: check_exhaustive_effect_deps -- Exhaustive useEffect Deps

**Upstream:** Same as Gap 4 but for `useEffect`/`useLayoutEffect`
**Current state:** Returns `Vec::new()`.
**What's needed:**
- Same approach as Gap 4 but scoped to effect hooks
- Special handling for refs (refs in effect deps are usually unnecessary since ref.current is not reactive)
- Special handling for cleanup functions
- Include autofix suggestions
- Test cases: missing dep in useEffect, ref.current in deps, dispatch/setState in deps (stable), cleanup referencing stale values
**Depends on:** None

### Gap 6: Structured Error Categories for Lint Filtering ✅

~~**Upstream:** N/A (upstream ESLint plugin uses the compiler's error severity system)~~
~~**Current state:** `check_*_with_source` functions filter diagnostics by string matching (`msg.contains("hook")`, etc.), which is fragile and will miss or mis-categorize diagnostics.~~

**Completed**: Added a `DiagnosticKind` enum with 17 variants to `CompilerError`, tagged all 14 validation passes with the correct kind, and updated `tier2.rs` to filter by kind instead of string matching. This makes lint filtering robust against diagnostic message changes.
