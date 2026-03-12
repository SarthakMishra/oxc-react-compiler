#![allow(dead_code)]
//! # rules-of-hooks
//!
//! Validates the Rules of Hooks:
//! 1. Hooks must only be called at the top level of a function component or custom hook.
//! 2. Hooks must not be called inside conditions, loops, or nested functions.

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;

use crate::utils::hook_detection::{is_component_name, is_hook_call, is_hook_name};

/// Check for violations of the Rules of Hooks.
pub fn check_rules_of_hooks(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    let mut visitor = RulesOfHooksVisitor {
        diagnostics: Vec::new(),
        // We start outside any component/hook context.
        context_stack: Vec::new(),
    };
    visitor.visit_program(program);
    visitor.diagnostics
}

/// Tracks what kind of function context we are in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FunctionContext {
    /// A function component or custom hook — hooks are allowed at top level.
    ComponentOrHook,
    /// Some other function (callback, helper, etc.) — hooks are not allowed.
    Other,
}

/// Tracks the nesting within a component/hook function body.
#[derive(Clone)]
struct ContextFrame {
    /// What kind of function this is.
    kind: FunctionContext,
    /// Nesting depth of control flow (if/for/while/switch/ternary) within this function.
    /// When > 0, hook calls are forbidden even in a component/hook.
    control_flow_depth: u32,
}

struct RulesOfHooksVisitor {
    diagnostics: Vec<OxcDiagnostic>,
    context_stack: Vec<ContextFrame>,
}

impl RulesOfHooksVisitor {
    fn current_context(&self) -> Option<&ContextFrame> {
        self.context_stack.last()
    }

    fn is_hook_allowed(&self) -> bool {
        match self.current_context() {
            Some(frame) => {
                frame.kind == FunctionContext::ComponentOrHook && frame.control_flow_depth == 0
            }
            // Top-level module scope — hooks not allowed.
            None => false,
        }
    }

    fn push_function(&mut self, name: Option<&str>) {
        let kind = match self.current_context() {
            // If we're already nested inside a component/hook, a nested function is "Other".
            Some(_) => {
                if let Some(n) = name {
                    if is_component_name(n) || is_hook_name(n) {
                        FunctionContext::ComponentOrHook
                    } else {
                        FunctionContext::Other
                    }
                } else {
                    FunctionContext::Other
                }
            }
            // Top-level function — check if it is a component or hook.
            None => {
                if let Some(n) = name {
                    if is_component_name(n) || is_hook_name(n) {
                        FunctionContext::ComponentOrHook
                    } else {
                        FunctionContext::Other
                    }
                } else {
                    FunctionContext::Other
                }
            }
        };
        self.context_stack.push(ContextFrame { kind, control_flow_depth: 0 });
    }

    fn pop_function(&mut self) {
        self.context_stack.pop();
    }

    fn report_conditional(&mut self, span: Span) {
        self.diagnostics.push(
            OxcDiagnostic::warn(
                "React hooks must be called at the top level. Do not call hooks inside conditions, loops, or nested functions.",
            )
            .with_label(span),
        );
    }

    fn report_not_in_component(&mut self, span: Span) {
        self.diagnostics.push(
            OxcDiagnostic::warn(
                "React hooks can only be called inside function components or custom hooks.",
            )
            .with_label(span),
        );
    }

    /// Determine the function name from a variable declarator that holds a function expression.
    /// e.g. `const MyComponent = function() { ... }` or `const useHook = () => { ... }`
    fn function_name_from_id<'b>(id: &'b BindingPattern<'_>) -> Option<&'b str> {
        match id {
            BindingPattern::BindingIdentifier(ident) => Some(ident.name.as_str()),
            _ => None,
        }
    }
}

impl<'a> Visit<'a> for RulesOfHooksVisitor {
    fn visit_function(&mut self, it: &Function<'a>, _flags: ScopeFlags) {
        let name = it.id.as_ref().map(|id| id.name.as_str());
        self.push_function(name);
        if let Some(body) = &it.body {
            self.visit_function_body(body);
        }
        self.pop_function();
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        // Arrow functions don't have names directly; they might get a name from
        // variable declarations, but we handle that case specially.
        // For standalone arrows, they are "Other" context.
        self.push_function(None);
        self.visit_function_body(&it.body);
        self.pop_function();
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        // Check for `const Foo = () => { ... }` or `const Foo = function() { ... }` patterns.
        // We need to give the arrow/function the name of the variable.
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
                    // Prefer the function's own name, fall back to the variable name.
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
        // Default walk for other cases.
        walk::walk_variable_declarator(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if is_hook_call(it) && !self.is_hook_allowed() {
            match self.current_context() {
                Some(frame) if frame.kind == FunctionContext::ComponentOrHook => {
                    // Inside a component/hook but in a conditional/loop.
                    self.report_conditional(it.span);
                }
                _ => {
                    // Not in a component/hook at all.
                    self.report_not_in_component(it.span);
                }
            }
        }
        // Continue walking into arguments (there may be nested arrow functions etc.)
        walk::walk_call_expression(self, it);
    }

    // Track control-flow nesting.
    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        if let Some(frame) = self.context_stack.last_mut() {
            frame.control_flow_depth += 1;
        }
        walk::walk_if_statement(self, it);
        if let Some(frame) = self.context_stack.last_mut() {
            frame.control_flow_depth -= 1;
        }
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        if let Some(frame) = self.context_stack.last_mut() {
            frame.control_flow_depth += 1;
        }
        walk::walk_for_statement(self, it);
        if let Some(frame) = self.context_stack.last_mut() {
            frame.control_flow_depth -= 1;
        }
    }

    fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
        if let Some(frame) = self.context_stack.last_mut() {
            frame.control_flow_depth += 1;
        }
        walk::walk_while_statement(self, it);
        if let Some(frame) = self.context_stack.last_mut() {
            frame.control_flow_depth -= 1;
        }
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        if let Some(frame) = self.context_stack.last_mut() {
            frame.control_flow_depth += 1;
        }
        walk::walk_switch_statement(self, it);
        if let Some(frame) = self.context_stack.last_mut() {
            frame.control_flow_depth -= 1;
        }
    }

    fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
        if let Some(frame) = self.context_stack.last_mut() {
            frame.control_flow_depth += 1;
        }
        walk::walk_conditional_expression(self, it);
        if let Some(frame) = self.context_stack.last_mut() {
            frame.control_flow_depth -= 1;
        }
    }
}
