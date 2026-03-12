//! Tier 2 lint rules that depend on the React Compiler's HIR analysis.
//!
//! These run the full compiler pipeline in lint mode to detect issues
//! that require deep analysis (mutation tracking, scope inference, etc.).

use oxc_ast::ast::Program;
use oxc_diagnostics::OxcDiagnostic;

use oxc_react_compiler::entrypoint::options::PluginOptions;
use oxc_react_compiler::entrypoint::pipeline::run_full_pipeline;
use oxc_react_compiler::entrypoint::program::discover_functions;
use oxc_react_compiler::error::{DiagnosticKind, ErrorCollector};
use oxc_react_compiler::hir::build::HIRBuilder;
use oxc_react_compiler::hir::environment::EnvironmentConfig;
use oxc_react_compiler::hir::types::ReactFunctionType;

/// Lint-mode environment config with all validation passes enabled.
fn lint_config() -> EnvironmentConfig {
    EnvironmentConfig {
        validate_exhaustive_memo_dependencies: true,
        validate_exhaustive_effect_dependencies: true,
        ..EnvironmentConfig::default()
    }
}

/// Run the compiler pipeline in lint mode on an already-parsed program,
/// collecting all errors with their structured diagnostic kinds intact.
fn run_lint_analysis(program: &Program<'_>) -> ErrorCollector {
    let options = PluginOptions::default();
    let config = lint_config();
    let mut all_errors = ErrorCollector::default();

    let functions = discover_functions(program, &options);

    for func_info in &functions {
        if func_info.opt_out {
            continue;
        }

        let hir_func = find_and_build_function(program, func_info.span, func_info.fn_type, &config);

        let Some(hir_func) = hir_func else {
            continue;
        };

        let mut errors = ErrorCollector::default();
        let _ = run_full_pipeline(hir_func, &config, &mut errors);

        all_errors.extend(&mut errors);
    }

    all_errors
}

/// Find a function in the AST by span and build its HIR.
fn find_and_build_function<'a>(
    program: &'a Program<'a>,
    span: oxc_span::Span,
    fn_type: ReactFunctionType,
    config: &EnvironmentConfig,
) -> Option<oxc_react_compiler::hir::types::HIRFunction> {
    use oxc_ast::ast::*;

    for stmt in &program.body {
        match stmt {
            Statement::FunctionDeclaration(func) if func.span == span => {
                let builder = HIRBuilder::new(config.clone());
                return Some(builder.build_function(func, fn_type));
            }
            Statement::ExportDefaultDeclaration(export) => {
                if let ExportDefaultDeclarationKind::FunctionDeclaration(func) = &export.declaration
                    && func.span == span
                {
                    let builder = HIRBuilder::new(config.clone());
                    return Some(builder.build_function(func, fn_type));
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                if let Some(Declaration::FunctionDeclaration(func)) = &export.declaration
                    && func.span == span
                {
                    let builder = HIRBuilder::new(config.clone());
                    return Some(builder.build_function(func, fn_type));
                }
            }
            Statement::VariableDeclaration(decl) => {
                for declarator in &decl.declarations {
                    if let Some(init) = &declarator.init {
                        match init.without_parentheses() {
                            Expression::ArrowFunctionExpression(arrow) if arrow.span == span => {
                                let name =
                                    if let BindingPattern::BindingIdentifier(id) = &declarator.id {
                                        Some(id.name.to_string())
                                    } else {
                                        None
                                    };
                                let builder = HIRBuilder::new(config.clone());
                                return Some(builder.build_arrow_function(arrow, name, fn_type));
                            }
                            Expression::FunctionExpression(func) if func.span == span => {
                                let builder = HIRBuilder::new(config.clone());
                                return Some(builder.build_function(func, fn_type));
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Run all Tier 2 rules on the given program and return diagnostics.
///
/// This runs the full compiler pipeline (HIR → SSA → inference → reactive scopes → validation)
/// and collects diagnostics from all validation passes.
pub fn run_tier2_rules(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    run_lint_analysis(program).into_diagnostics()
}

/// Full Rules of Hooks validation using HIR control flow analysis.
pub fn check_hooks_tier2(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    run_lint_analysis(program).diagnostics_by_kind(DiagnosticKind::HooksViolation)
}

/// Detect mutation of frozen (immutable) values.
pub fn check_immutability(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    run_lint_analysis(program).diagnostics_by_kind(DiagnosticKind::ImmutabilityViolation)
}

/// Validate that the compiler's memoization preserves manual useMemo/useCallback guarantees.
pub fn check_preserve_manual_memoization(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    run_lint_analysis(program).diagnostics_by_kind(DiagnosticKind::MemoizationPreservation)
}

/// Validate exhaustive dependencies for useMemo/useCallback.
pub fn check_memo_dependencies(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    run_lint_analysis(program).diagnostics_by_kind(DiagnosticKind::MemoDependency)
}

/// Validate exhaustive dependencies for useEffect/useLayoutEffect.
pub fn check_exhaustive_effect_deps(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    run_lint_analysis(program).diagnostics_by_kind(DiagnosticKind::EffectDependency)
}
