# Lint Rule Gaps

> Tier 1 rules (AST-only) are complete. Tier 2 rules (requiring compiler analysis)
> are all stubs that return empty vectors.

---

## Gap 1: Tier 2 Lint Rules

**Upstream:** `packages/eslint-plugin-react-compiler/src/rules/` (various files)
**Current state:** `crates/oxc_react_compiler_lint/src/rules/tier2.rs` has 5 functions
that all return `Vec::new()`:
- `check_hooks_tier2` -- full Rules of Hooks via HIR CFG analysis
- `check_immutability` -- detect mutation of frozen values via effect system
- `check_preserve_manual_memoization` -- validate useMemo/useCallback preservation
- `check_memo_dependencies` -- validate exhaustive deps for useMemo/useCallback
- `check_exhaustive_effect_deps` -- validate exhaustive deps for useEffect

**What's needed:**
Each function needs to:
1. Parse the program with oxc_parser
2. Discover functions via `discover_functions`
3. For each function, build HIR and run pipeline in lint mode
4. Extract the relevant validation errors from the error collector
5. Convert `CompilerError` to `OxcDiagnostic`

**Depends on:** Pipeline must work end-to-end (pipeline.md gaps 1-4), specifically
the inference passes must produce correct effects for the lint rules to report
meaningful diagnostics.
