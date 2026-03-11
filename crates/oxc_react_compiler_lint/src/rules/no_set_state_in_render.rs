#![allow(dead_code)]
//! # no-set-state-in-render
//!
//! Disallows unconditional `setState` calls in component render bodies.
//! Calling setState during render causes infinite re-render loops.
//! setState calls inside callbacks or event handlers are fine.

use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_diagnostics::OxcDiagnostic;
use oxc_syntax::scope::ScopeFlags;

use crate::utils::hook_detection::{is_component_name, is_hook_name, is_set_state_call};

/// Check for setState calls during render.
pub fn check_no_set_state_in_render<'a>(program: &Program<'a>) -> Vec<OxcDiagnostic> {
    let mut visitor = NoSetStateInRenderVisitor {
        diagnostics: Vec::new(),
        context_stack: Vec::new(),
    };
    visitor.visit_program(program);
    visitor.diagnostics
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RenderContext {
    /// Top-level of a component/hook body — setState here is an error.
    RenderPhase,
    /// Inside a callback, event handler, or non-component function — setState is OK.
    Nested,
}

struct NoSetStateInRenderVisitor {
    diagnostics: Vec<OxcDiagnostic>,
    context_stack: Vec<RenderContext>,
}

impl NoSetStateInRenderVisitor {
    fn is_in_render_phase(&self) -> bool {
        self.context_stack
            .last()
            .is_some_and(|ctx| *ctx == RenderContext::RenderPhase)
    }

    fn push_function(&mut self, name: Option<&str>) {
        let kind = if self.context_stack.is_empty() {
            // Top-level function.
            if name.is_some_and(|n| is_component_name(n) || is_hook_name(n)) {
                RenderContext::RenderPhase
            } else {
                RenderContext::Nested
            }
        } else {
            // Any nested function means we're in a callback/handler — setState is OK.
            RenderContext::Nested
        };
        self.context_stack.push(kind);
    }

    fn pop_function(&mut self) {
        self.context_stack.pop();
    }

    fn function_name_from_id<'b>(id: &'b BindingPattern<'_>) -> Option<&'b str> {
        match id {
            BindingPattern::BindingIdentifier(ident) => Some(ident.name.as_str()),
            _ => None,
        }
    }
}

impl<'a> Visit<'a> for NoSetStateInRenderVisitor {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        let name = it.id.as_ref().map(|id| id.name.as_str());
        self.push_function(name);
        if let Some(body) = &it.body {
            self.visit_function_body(body);
        }
        self.pop_function();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.push_function(None);
        self.visit_function_body(&it.body);
        self.pop_function();
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let Some(init) = &it.init {
            match init {
                Expression::ArrowFunctionExpression(arrow) => {
                    let name = Self::function_name_from_id(&it.id);
                    self.push_function(name);
                    self.visit_function_body(&arrow.body);
                    self.pop_function();
                    return;
                }
                Expression::FunctionExpression(func) => {
                    let name = func
                        .id
                        .as_ref()
                        .map(|id| id.name.as_str())
                        .or_else(|| Self::function_name_from_id(&it.id));
                    self.push_function(name);
                    if let Some(body) = &func.body {
                        self.visit_function_body(body);
                    }
                    self.pop_function();
                    return;
                }
                _ => {}
            }
        }
        walk::walk_variable_declarator(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.is_in_render_phase() && is_set_state_call(it) {
            self.diagnostics.push(
                OxcDiagnostic::warn("setState called during render will cause an infinite loop.")
                    .with_label(it.span),
            );
        }
        walk::walk_call_expression(self, it);
    }
}
