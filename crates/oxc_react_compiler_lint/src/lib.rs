pub mod rules;
pub mod utils;

use oxc_ast::ast::Program;
use oxc_diagnostics::OxcDiagnostic;

use rules::{
    no_jsx_in_try::check_no_jsx_in_try, no_ref_access_in_render::check_no_ref_access_in_render,
    no_set_state_in_effects::check_no_set_state_in_effects,
    no_set_state_in_render::check_no_set_state_in_render, rules_of_hooks::check_rules_of_hooks,
};

/// Run all Tier 1 lint rules on the given program and return any diagnostics found.
pub fn run_lint_rules<'a>(program: &Program<'a>) -> Vec<OxcDiagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_rules_of_hooks(program));
    diagnostics.extend(check_no_jsx_in_try(program));
    diagnostics.extend(check_no_ref_access_in_render(program));
    diagnostics.extend(check_no_set_state_in_render(program));
    diagnostics.extend(check_no_set_state_in_effects(program));

    diagnostics
}
