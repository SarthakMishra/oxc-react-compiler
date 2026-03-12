#![allow(dead_code)]
//! # purity
//!
//! Detects calls to known impure functions during render. Functions like
//! `Math.random()`, `Date.now()`, and `crypto.getRandomValues()` produce
//! non-deterministic results and can break React's rendering model.

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;

/// Known impure member expressions: (object, property).
const IMPURE_MEMBER_CALLS: &[(&str, &str)] = &[
    ("Math", "random"),
    ("Date", "now"),
    ("crypto", "getRandomValues"),
    ("crypto", "randomUUID"),
    ("performance", "now"),
];

/// Known impure global function calls.
const IMPURE_GLOBAL_CALLS: &[&str] = &["fetch", "setTimeout", "setInterval", "queueMicrotask"];

/// Check for impure function calls that should not appear in render.
pub fn check_purity(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    let mut visitor = PurityVisitor { diagnostics: Vec::new() };
    visitor.visit_program(program);
    visitor.diagnostics
}

struct PurityVisitor {
    diagnostics: Vec<OxcDiagnostic>,
}

impl<'a> Visit<'a> for PurityVisitor {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        // Check member expression calls like Math.random()
        if let Expression::StaticMemberExpression(member) = &it.callee
            && let Expression::Identifier(obj) = &member.object
        {
            let obj_name = obj.name.as_str();
            let prop_name = member.property.name.as_str();
            for (impure_obj, impure_prop) in IMPURE_MEMBER_CALLS {
                if obj_name == *impure_obj && prop_name == *impure_prop {
                    self.diagnostics.push(
                            OxcDiagnostic::warn(format!(
                                "{obj_name}.{prop_name}() is impure and should not be called during render."
                            ))
                            .with_label(it.span),
                        );
                }
            }
        }

        // Check global function calls like fetch()
        if let Expression::Identifier(ident) = &it.callee {
            let name = ident.name.as_str();
            if IMPURE_GLOBAL_CALLS.contains(&name) {
                self.diagnostics.push(
                    OxcDiagnostic::warn(format!(
                        "{name}() is impure and should not be called during render."
                    ))
                    .with_label(it.span),
                );
            }
        }

        walk::walk_call_expression(self, it);
    }
}
