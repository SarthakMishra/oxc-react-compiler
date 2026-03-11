#![allow(dead_code)]
//! # no-capitalized-calls
//!
//! Detects PascalCase function calls that might indicate a component being
//! called as a regular function instead of being rendered as JSX.
//! For example, `MyComponent()` should be `<MyComponent />`.

use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_diagnostics::OxcDiagnostic;

use crate::utils::hook_detection::is_component_name;

/// Check for PascalCase function calls that should be JSX.
pub fn check_no_capitalized_calls<'a>(program: &Program<'a>) -> Vec<OxcDiagnostic> {
    let mut visitor = NoCapitalizedCallsVisitor {
        diagnostics: Vec::new(),
    };
    visitor.visit_program(program);
    visitor.diagnostics
}

struct NoCapitalizedCallsVisitor {
    diagnostics: Vec<OxcDiagnostic>,
}

impl<'a> Visit<'a> for NoCapitalizedCallsVisitor {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::Identifier(ident) = &it.callee {
            let name = ident.name.as_str();
            if is_component_name(name) {
                // Exclude known globals that are PascalCase but not components
                let known_non_components = [
                    "Array",
                    "Boolean",
                    "Date",
                    "Error",
                    "Function",
                    "Map",
                    "Number",
                    "Object",
                    "Promise",
                    "Proxy",
                    "Reflect",
                    "RegExp",
                    "Set",
                    "String",
                    "Symbol",
                    "TypeError",
                    "RangeError",
                    "WeakMap",
                    "WeakSet",
                    "Intl",
                    "BigInt",
                ];
                if !known_non_components.contains(&name) {
                    self.diagnostics.push(
                        OxcDiagnostic::warn(format!(
                            "\"{}\" is called as a regular function. If this is a React component, render it as JSX: `<{} />`.",
                            name, name
                        ))
                        .with_label(it.span),
                    );
                }
            }
        }

        walk::walk_call_expression(self, it);
    }
}
