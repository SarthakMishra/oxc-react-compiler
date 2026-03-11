#![allow(dead_code)]

use oxc_ast::ast::*;
use oxc_diagnostics::OxcDiagnostic;

/// Check for mutations of known global objects (Array.prototype, Object.prototype, etc.)
pub fn check_globals(program: &Program) -> Vec<OxcDiagnostic> {
    let mut diagnostics = Vec::new();

    for stmt in &program.body {
        check_statement_for_global_mutation(stmt, &mut diagnostics);
    }

    diagnostics
}

fn check_statement_for_global_mutation(stmt: &Statement, diagnostics: &mut Vec<OxcDiagnostic>) {
    if let Statement::ExpressionStatement(expr_stmt) = stmt {
        check_expr_for_global_mutation(&expr_stmt.expression, diagnostics);
    }
}

fn check_expr_for_global_mutation(expr: &Expression, diagnostics: &mut Vec<OxcDiagnostic>) {
    // Check for assignments to global properties: Array.prototype.foo = ...
    if let Expression::AssignmentExpression(assign) = expr {
        if is_global_prototype_mutation(&assign.left) {
            diagnostics.push(
                OxcDiagnostic::warn(
                    "Mutating global prototype is not compatible with the React Compiler",
                )
                .with_label(assign.span),
            );
        }
    }
}

fn is_global_prototype_mutation(target: &AssignmentTarget) -> bool {
    // Check for patterns like Array.prototype.foo, Object.prototype.bar
    match target {
        AssignmentTarget::StaticMemberExpression(member) => {
            if let Expression::StaticMemberExpression(inner) = &member.object {
                if inner.property.name == "prototype" {
                    if let Expression::Identifier(id) = &inner.object {
                        return matches!(
                            id.name.as_str(),
                            "Array"
                                | "Object"
                                | "String"
                                | "Number"
                                | "Boolean"
                                | "Function"
                                | "Symbol"
                                | "Map"
                                | "Set"
                                | "WeakMap"
                                | "WeakSet"
                        );
                    }
                }
            }
            false
        }
        _ => false,
    }
}
