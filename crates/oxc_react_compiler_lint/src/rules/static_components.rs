#![allow(dead_code)]
//! # static-components
//!
//! Detects component definitions inside other component or function bodies.
//! Defining a component inline causes React to unmount and remount it on every
//! render, which destroys all state and DOM.

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;
use oxc_syntax::scope::ScopeFlags;

use crate::utils::hook_detection::is_component_name;

/// Check for component definitions inside render functions.
pub fn check_static_components<'a>(program: &Program<'a>) -> Vec<OxcDiagnostic> {
    let mut visitor = StaticComponentsVisitor { diagnostics: Vec::new(), component_depth: 0 };
    visitor.visit_program(program);
    visitor.diagnostics
}

struct StaticComponentsVisitor {
    diagnostics: Vec<OxcDiagnostic>,
    /// Nesting depth of component function definitions.
    component_depth: u32,
}

impl<'a> Visit<'a> for StaticComponentsVisitor {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        let name = it.id.as_ref().map(|id| id.name.as_str()).unwrap_or("");
        let is_component = is_component_name(name);

        if is_component && self.component_depth > 0 {
            self.diagnostics.push(
                OxcDiagnostic::warn(format!(
                    "Component \"{}\" is defined inside another component. \
                     Move it outside to avoid remounting on every render.",
                    name
                ))
                .with_label(it.span),
            );
        }

        if is_component {
            self.component_depth += 1;
        }
        if let Some(body) = &it.body {
            self.visit_function_body(body);
        }
        if is_component {
            self.component_depth -= 1;
        }
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        // Check for const MyComponent = () => { ... } or const MyComponent = function() { ... }
        if self.component_depth > 0 {
            if let BindingPattern::BindingIdentifier(ident) = &it.id {
                let name = ident.name.as_str();
                if is_component_name(name) {
                    if let Some(init) = &it.init {
                        let is_fn = matches!(
                            init,
                            Expression::ArrowFunctionExpression(_)
                                | Expression::FunctionExpression(_)
                        );
                        if is_fn {
                            self.diagnostics.push(
                                OxcDiagnostic::warn(format!(
                                    "Component \"{}\" is defined inside another component. \
                                     Move it outside to avoid remounting on every render.",
                                    name
                                ))
                                .with_label(it.span),
                            );
                        }
                    }
                }
            }
        }

        walk::walk_variable_declarator(self, it);
    }
}
