#![allow(dead_code)]

use oxc_ast::ast::*;
use oxc_span::Span;

use super::options::{CompilationMode, PluginOptions};
use crate::hir::globals::{is_component_name, is_hook_name};
use crate::hir::types::ReactFunctionType;

/// A function discovered in the AST that should be compiled.
#[derive(Debug)]
pub struct DiscoveredFunction {
    /// The name of the function (if it has one)
    pub name: Option<String>,
    /// Whether this is a Component, Hook, or Other
    pub fn_type: ReactFunctionType,
    /// The source span of the function
    pub span: Span,
    /// Whether the function has a "use no memo" directive
    pub opt_out: bool,
}

/// Walk the program to find functions that should be compiled.
pub fn discover_functions<'a>(
    program: &'a Program<'a>,
    options: &PluginOptions,
) -> Vec<DiscoveredFunction> {
    let mut functions = Vec::new();

    for stmt in &program.body {
        discover_in_statement(stmt, options, &mut functions);
    }

    functions
}

fn discover_in_statement<'a>(
    stmt: &'a Statement<'a>,
    options: &PluginOptions,
    functions: &mut Vec<DiscoveredFunction>,
) {
    match stmt {
        Statement::FunctionDeclaration(func) => {
            if let Some(id) = func.id.as_ref() {
                let name = id.name.to_string();
                let fn_type = classify_function_name(&name);

                if should_compile(
                    &name,
                    fn_type,
                    func.body.as_ref().map(|b| b.directives.as_slice()),
                    options,
                ) {
                    let opt_out =
                        has_opt_out_directive(func.body.as_ref().map(|b| b.directives.as_slice()));
                    functions.push(DiscoveredFunction {
                        name: Some(name),
                        fn_type,
                        span: func.span,
                        opt_out,
                    });
                }
            }
        }
        Statement::ExportDefaultDeclaration(export) => match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
                let name = func.id.as_ref().map(|id| id.name.to_string());
                let fn_type = name
                    .as_deref()
                    .map(classify_function_name)
                    .unwrap_or(ReactFunctionType::Component);

                if should_compile_default_export(name.as_deref(), fn_type, options) {
                    let opt_out =
                        has_opt_out_directive(func.body.as_ref().map(|b| b.directives.as_slice()));
                    functions.push(DiscoveredFunction {
                        name,
                        fn_type,
                        span: func.span,
                        opt_out,
                    });
                }
            }
            _ => {}
        },
        Statement::ExportNamedDeclaration(export) => {
            if let Some(decl) = &export.declaration {
                discover_in_declaration(decl, options, functions);
            }
        }
        Statement::VariableDeclaration(decl) => {
            discover_in_variable_declaration(decl, options, functions);
        }
        _ => {}
    }
}

fn discover_in_declaration<'a>(
    decl: &'a Declaration<'a>,
    options: &PluginOptions,
    functions: &mut Vec<DiscoveredFunction>,
) {
    match decl {
        Declaration::FunctionDeclaration(func) => {
            if let Some(id) = func.id.as_ref() {
                let name = id.name.to_string();
                let fn_type = classify_function_name(&name);

                if should_compile(
                    &name,
                    fn_type,
                    func.body.as_ref().map(|b| b.directives.as_slice()),
                    options,
                ) {
                    let opt_out =
                        has_opt_out_directive(func.body.as_ref().map(|b| b.directives.as_slice()));
                    functions.push(DiscoveredFunction {
                        name: Some(name),
                        fn_type,
                        span: func.span,
                        opt_out,
                    });
                }
            }
        }
        Declaration::VariableDeclaration(decl) => {
            discover_in_variable_declaration(decl, options, functions);
        }
        _ => {}
    }
}

fn discover_in_variable_declaration<'a>(
    decl: &'a VariableDeclaration<'a>,
    options: &PluginOptions,
    functions: &mut Vec<DiscoveredFunction>,
) {
    for declarator in &decl.declarations {
        if let Some(init) = &declarator.init {
            if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                let name = id.name.to_string();
                let fn_type = classify_function_name(&name);

                // Check if initializer is a function expression or arrow
                let (is_function, span) = match init.without_parentheses() {
                    Expression::ArrowFunctionExpression(arrow) => (true, arrow.span),
                    Expression::FunctionExpression(func) => (true, func.span),
                    // Handle React.forwardRef(() => ...) and React.memo(() => ...)
                    Expression::CallExpression(call) => {
                        if is_react_wrapper_call(call) {
                            (true, call.span)
                        } else {
                            (false, Span::default())
                        }
                    }
                    _ => (false, Span::default()),
                };

                if is_function && should_compile(&name, fn_type, None, options) {
                    functions.push(DiscoveredFunction {
                        name: Some(name),
                        fn_type,
                        span,
                        opt_out: false,
                    });
                }
            }
        }
    }
}

/// Classify a function name as Component, Hook, or Other
fn classify_function_name(name: &str) -> ReactFunctionType {
    if is_hook_name(name) {
        ReactFunctionType::Hook
    } else if is_component_name(name) {
        ReactFunctionType::Component
    } else {
        ReactFunctionType::Other
    }
}

/// Check if a call expression is React.forwardRef/memo/lazy
fn is_react_wrapper_call(call: &CallExpression<'_>) -> bool {
    match &call.callee {
        Expression::StaticMemberExpression(member) => {
            if let Expression::Identifier(obj) = &member.object {
                obj.name == "React"
                    && matches!(
                        member.property.name.as_str(),
                        "forwardRef" | "memo" | "lazy"
                    )
            } else {
                false
            }
        }
        Expression::Identifier(id) => {
            matches!(id.name.as_str(), "forwardRef" | "memo" | "lazy")
        }
        _ => false,
    }
}

/// Decide if a function should be compiled based on options
fn should_compile(
    _name: &str,
    fn_type: ReactFunctionType,
    directives: Option<&[Directive<'_>]>,
    options: &PluginOptions,
) -> bool {
    // Check for opt-out
    if has_opt_out_directive(directives) {
        return false;
    }

    match options.compilation_mode {
        CompilationMode::All => true,
        CompilationMode::Infer => {
            // Infer mode: compile components and hooks
            matches!(
                fn_type,
                ReactFunctionType::Component | ReactFunctionType::Hook
            )
        }
        CompilationMode::Syntax => {
            // Only compile functions with "use memo" directive
            has_memo_directive(directives)
        }
        CompilationMode::Annotation => has_memo_directive(directives),
    }
}

fn should_compile_default_export(
    name: Option<&str>,
    _fn_type: ReactFunctionType,
    options: &PluginOptions,
) -> bool {
    match options.compilation_mode {
        CompilationMode::All => true,
        CompilationMode::Infer => {
            // Default exports that look like components
            name.map_or(true, |n| {
                matches!(
                    classify_function_name(n),
                    ReactFunctionType::Component | ReactFunctionType::Hook
                )
            })
        }
        _ => false,
    }
}

fn has_opt_out_directive(directives: Option<&[Directive<'_>]>) -> bool {
    directives.map_or(false, |dirs| {
        dirs.iter().any(|d| d.directive.as_str() == "use no memo")
    })
}

fn has_memo_directive(directives: Option<&[Directive<'_>]>) -> bool {
    directives.map_or(false, |dirs| {
        dirs.iter().any(|d| d.directive.as_str() == "use memo")
    })
}
