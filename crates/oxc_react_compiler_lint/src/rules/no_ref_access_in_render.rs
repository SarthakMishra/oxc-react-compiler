#![allow(dead_code)]
//! # no-ref-access-in-render
//!
//! Disallows accessing `.current` on ref values during render. Refs should only
//! be read/written inside effects or event handlers.

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;

use crate::utils::hook_detection::{is_component_name, is_hook_name, is_use_ref_call};

/// Check for `.current` accesses on ref values during render.
pub fn check_no_ref_access_in_render(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    let mut visitor =
        NoRefAccessInRenderVisitor { diagnostics: Vec::new(), context_stack: Vec::new() };
    visitor.visit_program(program);
    visitor.diagnostics
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RenderContext {
    /// Inside a component/hook body at the top level (render phase).
    RenderPhase,
    /// Inside a callback/nested function (not render phase).
    Nested,
}

#[derive(Clone)]
struct ContextFrame {
    kind: RenderContext,
    /// Names of variables known to hold refs (from `useRef()` calls).
    ref_names: FxHashSet<String>,
}

struct NoRefAccessInRenderVisitor {
    diagnostics: Vec<OxcDiagnostic>,
    context_stack: Vec<ContextFrame>,
}

impl NoRefAccessInRenderVisitor {
    fn current_context(&self) -> Option<&ContextFrame> {
        self.context_stack.last()
    }

    fn is_in_render_phase(&self) -> bool {
        self.current_context().is_some_and(|f| f.kind == RenderContext::RenderPhase)
    }

    /// Check if a name is a known ref in any enclosing render-phase context.
    fn is_known_ref(&self, name: &str) -> bool {
        for frame in self.context_stack.iter().rev() {
            if frame.ref_names.contains(name) {
                return true;
            }
            // Stop searching if we leave a render-phase context.
            if frame.kind != RenderContext::RenderPhase {
                break;
            }
        }
        false
    }

    fn push_function(&mut self, name: Option<&str>) {
        let kind = if self.context_stack.is_empty() {
            // Top-level function — component or hook?
            if name.is_some_and(|n| is_component_name(n) || is_hook_name(n)) {
                RenderContext::RenderPhase
            } else {
                RenderContext::Nested
            }
        } else {
            // Nested function is always "not render phase".
            RenderContext::Nested
        };
        self.context_stack.push(ContextFrame { kind, ref_names: FxHashSet::default() });
    }

    fn pop_function(&mut self) {
        self.context_stack.pop();
    }

    fn register_ref(&mut self, name: String) {
        if let Some(frame) = self.context_stack.last_mut() {
            frame.ref_names.insert(name);
        }
    }

    fn function_name_from_id<'b>(id: &'b BindingPattern<'_>) -> Option<&'b str> {
        match id {
            BindingPattern::BindingIdentifier(ident) => Some(ident.name.as_str()),
            _ => None,
        }
    }
}

impl<'a> Visit<'a> for NoRefAccessInRenderVisitor {
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
        // Check for `const Foo = () => { ... }` or `const Foo = function() { ... }`
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

        // Track `const myRef = useRef(...)`.
        if let Some(init) = &it.init
            && let Expression::CallExpression(call) = init
            && is_use_ref_call(call)
            && let Some(name) = Self::function_name_from_id(&it.id)
        {
            self.register_ref(name.to_string());
        }

        walk::walk_variable_declarator(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        // Check for `ref.current` access during render.
        if self.is_in_render_phase()
            && it.property.name.as_str() == "current"
            && let Expression::Identifier(ident) = &it.object
            && self.is_known_ref(ident.name.as_str())
        {
            self.diagnostics.push(
                OxcDiagnostic::warn("Accessing ref.current during render can lead to bugs.")
                    .with_label(it.span),
            );
        }
        walk::walk_static_member_expression(self, it);
    }
}
