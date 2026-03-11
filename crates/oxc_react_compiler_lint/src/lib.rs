#![allow(
    clippy::needless_pass_by_value,
    clippy::only_used_in_recursion,
    clippy::match_same_arms
)]

pub mod rules;
pub mod utils;

use oxc_ast::ast::Program;
use oxc_diagnostics::OxcDiagnostic;

use rules::{
    globals::check_globals, incompatible_library::check_incompatible_library,
    no_capitalized_calls::check_no_capitalized_calls,
    no_deriving_state_in_effects::check_no_deriving_state_in_effects,
    no_jsx_in_try::check_no_jsx_in_try, no_ref_access_in_render::check_no_ref_access_in_render,
    no_set_state_in_effects::check_no_set_state_in_effects,
    no_set_state_in_render::check_no_set_state_in_render, purity::check_purity,
    rules_of_hooks::check_rules_of_hooks, static_components::check_static_components,
    use_memo_validation::check_use_memo_validation,
};

/// Run Tier 1 (AST-level) lint rules on the given program.
pub fn run_lint_rules(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    let mut diagnostics = Vec::new();

    // Core rules
    diagnostics.extend(check_rules_of_hooks(program));
    diagnostics.extend(check_no_jsx_in_try(program));
    diagnostics.extend(check_no_ref_access_in_render(program));
    diagnostics.extend(check_no_set_state_in_render(program));
    diagnostics.extend(check_no_set_state_in_effects(program));

    // Additional rules
    diagnostics.extend(check_use_memo_validation(program));
    diagnostics.extend(check_no_capitalized_calls(program));
    diagnostics.extend(check_purity(program));
    diagnostics.extend(check_incompatible_library(program));
    diagnostics.extend(check_static_components(program));
    diagnostics.extend(check_no_deriving_state_in_effects(program));
    diagnostics.extend(check_globals(program));

    diagnostics
}

/// Run all lint rules (Tier 1 + Tier 2) on the given program.
///
/// Tier 2 rules run the full compiler pipeline (HIR, SSA, mutation analysis,
/// reactive scopes) to detect issues that require deep analysis. This is
/// more expensive than `run_lint_rules` but catches more issues.
pub fn run_all_lint_rules(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    let mut diagnostics = run_lint_rules(program);
    diagnostics.extend(rules::tier2::run_tier2_rules(program));
    diagnostics
}
