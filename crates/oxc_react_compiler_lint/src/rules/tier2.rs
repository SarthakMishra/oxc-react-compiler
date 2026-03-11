#![allow(dead_code)]

//! Tier 2 lint rules that depend on the React Compiler's HIR analysis.
//! These run the full compiler pipeline in lint mode to detect issues
//! that require deep analysis (mutation tracking, scope inference, etc.).

use oxc_ast::ast::Program;
use oxc_diagnostics::OxcDiagnostic;

/// Full Rules of Hooks validation using HIR control flow analysis.
/// Goes beyond the AST-level check by analyzing the actual CFG for
/// conditional and loop paths.
pub fn check_hooks_tier2(_program: &Program) -> Vec<OxcDiagnostic> {
    // TODO: Run compiler in lint mode, extract hooks validation diagnostics
    // This would use:
    // 1. Parse -> discover functions -> build HIR
    // 2. Run validate_hooks_usage on the HIR CFG
    // 3. Convert compiler errors to OxcDiagnostics
    Vec::new()
}

/// Detect mutation of frozen (immutable) values.
/// Uses the effect system to track which values are frozen
/// and reports mutations of those values.
pub fn check_immutability(_program: &Program) -> Vec<OxcDiagnostic> {
    // TODO: Run compiler through mutation analysis, check for
    // AliasingEffect::MutateFrozen effects
    Vec::new()
}

/// Validate that the compiler's memoization preserves manual
/// useMemo/useCallback guarantees.
pub fn check_preserve_manual_memoization(_program: &Program) -> Vec<OxcDiagnostic> {
    // TODO: Run compiler through reactive scope analysis,
    // check StartMemoize/FinishMemoize pairs against computed scopes
    Vec::new()
}

/// Validate exhaustive dependencies for useMemo/useCallback.
/// Uses the compiler's dependency analysis to find missing deps.
pub fn check_memo_dependencies(_program: &Program) -> Vec<OxcDiagnostic> {
    // TODO: Run compiler through scope dependency analysis,
    // compare computed deps against declared deps
    Vec::new()
}

/// Validate exhaustive dependencies for useEffect/useLayoutEffect.
/// Similar to memo-dependencies but for effect hooks.
pub fn check_exhaustive_effect_deps(_program: &Program) -> Vec<OxcDiagnostic> {
    // TODO: Same as memo_dependencies but for effect hooks
    Vec::new()
}
