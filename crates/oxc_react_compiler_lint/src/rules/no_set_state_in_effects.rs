#![allow(dead_code)]
//! # no-set-state-in-effects
//!
//! Disallows direct `setState` calls in effect hook callbacks.
//! setState inside nested callbacks/promises within effects is OK (e.g. in a
//! `.then()` callback or an event listener).

use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_diagnostics::OxcDiagnostic;
use oxc_syntax::scope::ScopeFlags;

use crate::utils::hook_detection::{is_effect_hook_call, is_set_state_call};

/// Check for setState calls directly inside effect callbacks.
pub fn check_no_set_state_in_effects<'a>(program: &Program<'a>) -> Vec<OxcDiagnostic> {
    let mut visitor = NoSetStateInEffectsVisitor {
        diagnostics: Vec::new(),
        effect_depth: 0,
        nested_fn_depth: 0,
    };
    visitor.visit_program(program);
    visitor.diagnostics
}

struct NoSetStateInEffectsVisitor {
    diagnostics: Vec<OxcDiagnostic>,
    /// How many effect callback levels deep we are.
    effect_depth: u32,
    /// Nested function depth within an effect callback. When > 0, setState is OK.
    nested_fn_depth: u32,
}

impl NoSetStateInEffectsVisitor {
    /// Returns `true` if we are directly inside an effect callback (not in a
    /// nested function within it).
    fn is_direct_in_effect(&self) -> bool {
        self.effect_depth > 0 && self.nested_fn_depth == 0
    }
}

impl<'a> Visit<'a> for NoSetStateInEffectsVisitor {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        // Check if this is an effect hook call: useEffect(() => { ... })
        if is_effect_hook_call(it) {
            // The first argument is the effect callback.
            if let Some(first_arg) = it.arguments.first() {
                self.effect_depth += 1;
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
                self.effect_depth -= 1;

                // Visit remaining arguments (dependency array) normally.
                for arg in it.arguments.iter().skip(1) {
                    self.visit_argument(arg);
                }

                // Visit the callee.
                self.visit_expression(&it.callee);
                return;
            }
        }

        // Check if this is a setState call directly in an effect.
        if self.is_direct_in_effect() && is_set_state_call(it) {
            self.diagnostics.push(
                OxcDiagnostic::warn(
                    "Avoid calling setState directly in effect callbacks. This often indicates a missing dependency or a need to restructure the code.",
                )
                .with_label(it.span),
            );
        }

        walk::walk_call_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        if self.effect_depth > 0 {
            self.nested_fn_depth += 1;
        }
        if let Some(body) = &it.body {
            self.visit_function_body(body);
        }
        if self.effect_depth > 0 {
            self.nested_fn_depth -= 1;
        }
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if self.effect_depth > 0 {
            self.nested_fn_depth += 1;
        }
        self.visit_function_body(&it.body);
        if self.effect_depth > 0 {
            self.nested_fn_depth -= 1;
        }
    }
}
