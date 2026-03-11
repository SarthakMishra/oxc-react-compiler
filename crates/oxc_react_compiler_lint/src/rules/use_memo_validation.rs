#![allow(dead_code)]
//! # use-memo-validation
//!
//! Validates that `useMemo` and `useCallback` are called with the correct
//! number and types of arguments. The first argument must be a function
//! expression and the second must be a dependency array.

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;

use crate::utils::hook_detection::get_callee_name;

/// Check for invalid `useMemo`/`useCallback` usage patterns.
pub fn check_use_memo_validation<'a>(program: &Program<'a>) -> Vec<OxcDiagnostic> {
    let mut visitor = UseMemoValidationVisitor { diagnostics: Vec::new() };
    visitor.visit_program(program);
    visitor.diagnostics
}

struct UseMemoValidationVisitor {
    diagnostics: Vec<OxcDiagnostic>,
}

impl<'a> Visit<'a> for UseMemoValidationVisitor {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(name) = get_callee_name(it) {
            if name == "useMemo" || name == "useCallback" {
                // Must have exactly 2 arguments
                if it.arguments.len() != 2 {
                    self.diagnostics.push(
                        OxcDiagnostic::warn(format!(
                            "\"{}\" requires exactly 2 arguments (a callback and a dependency array), but received {}.",
                            name,
                            it.arguments.len()
                        ))
                        .with_label(it.span),
                    );
                } else {
                    // First argument should be a function
                    let first = &it.arguments[0];
                    let is_fn = matches!(
                        first,
                        Argument::ArrowFunctionExpression(_) | Argument::FunctionExpression(_)
                    );
                    if !is_fn {
                        self.diagnostics.push(
                            OxcDiagnostic::warn(format!(
                                "The first argument to \"{}\" should be a function expression.",
                                name
                            ))
                            .with_label(it.span),
                        );
                    }

                    // Second argument should be an array
                    let second = &it.arguments[1];
                    let is_array = matches!(second, Argument::ArrayExpression(_));
                    if !is_array {
                        self.diagnostics.push(
                            OxcDiagnostic::warn(format!(
                                "The second argument to \"{}\" should be a dependency array.",
                                name
                            ))
                            .with_label(it.span),
                        );
                    }
                }
            }
        }

        walk::walk_call_expression(self, it);
    }
}
