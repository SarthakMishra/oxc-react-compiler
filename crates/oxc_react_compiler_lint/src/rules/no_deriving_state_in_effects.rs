#![allow(dead_code)]
//! # no-deriving-state-in-effects
//!
//! Detects patterns where state is derived from props or other state inside
//! effect callbacks. This is an anti-pattern because the derived value should
//! be computed during render (e.g., with `useMemo`), not in an effect.

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;

use crate::utils::hook_detection::{get_callee_name, is_effect_hook_call, is_set_state_call};

/// Check for derived state computations inside effect callbacks.
pub fn check_no_deriving_state_in_effects<'a>(program: &Program<'a>) -> Vec<OxcDiagnostic> {
    let mut visitor =
        NoDerivedStateInEffectsVisitor { diagnostics: Vec::new(), in_effect_callback: false };
    visitor.visit_program(program);
    visitor.diagnostics
}

struct NoDerivedStateInEffectsVisitor {
    diagnostics: Vec<OxcDiagnostic>,
    /// Whether we are currently inside an effect callback at the top level.
    in_effect_callback: bool,
}

impl<'a> Visit<'a> for NoDerivedStateInEffectsVisitor {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_effect_hook_call(it) {
            // Visit the first argument (callback) with flag set
            if let Some(first_arg) = it.arguments.first() {
                let prev = self.in_effect_callback;
                self.in_effect_callback = true;
                match first_arg {
                    Argument::ArrowFunctionExpression(arrow) => {
                        self.visit_function_body(&arrow.body);
                    }
                    Argument::FunctionExpression(func) => {
                        if let Some(body) = &func.body {
                            self.visit_function_body(body);
                        }
                    }
                    _ => {
                        self.visit_argument(first_arg);
                    }
                }
                self.in_effect_callback = prev;

                // Visit remaining arguments normally
                for arg in it.arguments.iter().skip(1) {
                    self.visit_argument(arg);
                }
                self.visit_expression(&it.callee);
                return;
            }
        }

        // Check for setState calls inside effect
        if self.in_effect_callback && is_set_state_call(it) {
            let hook_name = get_callee_name(it).unwrap_or("setState");
            self.diagnostics.push(
                OxcDiagnostic::warn(format!(
                    "\"{}\" is called inside an effect callback. If this derives state from \
                     props or other state, compute the value during render instead (e.g., with useMemo).",
                    hook_name
                ))
                .with_label(it.span),
            );
        }

        walk::walk_call_expression(self, it);
    }
}
