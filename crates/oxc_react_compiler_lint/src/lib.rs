pub mod rules;
pub mod utils;

use oxc_ast::ast::Program;
use oxc_diagnostics::OxcDiagnostic;

use rules::{
    incompatible_library::check_incompatible_library,
    no_capitalized_calls::check_no_capitalized_calls,
    no_deriving_state_in_effects::check_no_deriving_state_in_effects,
    no_jsx_in_try::check_no_jsx_in_try, no_ref_access_in_render::check_no_ref_access_in_render,
    no_set_state_in_effects::check_no_set_state_in_effects,
    no_set_state_in_render::check_no_set_state_in_render, purity::check_purity,
    rules_of_hooks::check_rules_of_hooks, static_components::check_static_components,
    use_memo_validation::check_use_memo_validation,
};

/// Run all lint rules on the given program and return any diagnostics found.
pub fn run_lint_rules<'a>(program: &Program<'a>) -> Vec<OxcDiagnostic> {
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

    diagnostics
}
