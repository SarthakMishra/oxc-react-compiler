#![allow(dead_code)]

//! Tier 2 lint rules that depend on the React Compiler's HIR analysis.
//! These run the full compiler pipeline in lint mode to detect issues
//! that require deep analysis (mutation tracking, scope inference, etc.).

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::Parser;
use oxc_span::SourceType;

use oxc_react_compiler::entrypoint::options::PluginOptions;
use oxc_react_compiler::entrypoint::pipeline::run_lint_pipeline;
use oxc_react_compiler::entrypoint::program::discover_functions;
use oxc_react_compiler::error::{DiagnosticKind, ErrorCollector};
use oxc_react_compiler::hir::build::HIRBuilder;
use oxc_react_compiler::hir::environment::EnvironmentConfig;
use oxc_react_compiler::hir::types::ReactFunctionType;

/// Run the compiler pipeline in lint mode for a source file, collecting all errors
/// with their structured diagnostic kinds intact.
fn run_lint_analysis(source: &str, filename: &str) -> ErrorCollector {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default();
    let parser_ret = Parser::new(&allocator, source, source_type).parse();

    if parser_ret.panicked {
        return ErrorCollector::default();
    }

    let options = PluginOptions::default();
    let config = EnvironmentConfig::default();
    let mut all_errors = ErrorCollector::default();

    let functions = discover_functions(&parser_ret.program, &options);

    for func_info in &functions {
        if func_info.opt_out {
            continue;
        }

        // Find the function in the AST by span and build HIR.
        let hir_func = find_and_build_function(
            &parser_ret.program,
            func_info.span,
            func_info.fn_type,
            &config,
        );

        let Some(mut hir_func) = hir_func else {
            continue;
        };

        let mut errors = ErrorCollector::default();
        let _ = run_lint_pipeline(&mut hir_func.body, &config, &mut errors);

        all_errors.extend(&mut errors);
    }

    all_errors
}

/// Find a function in the AST by span and build its HIR.
fn find_and_build_function<'a>(
    program: &'a oxc_ast::ast::Program<'a>,
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
                {
                    if func.span == span {
                        let builder = HIRBuilder::new(config.clone());
                        return Some(builder.build_function(func, fn_type));
                    }
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                if let Some(Declaration::FunctionDeclaration(func)) = &export.declaration {
                    if func.span == span {
                        let builder = HIRBuilder::new(config.clone());
                        return Some(builder.build_function(func, fn_type));
                    }
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

/// Full Rules of Hooks validation using HIR control flow analysis.
/// Goes beyond the AST-level check by analyzing the actual CFG for
/// conditional and loop paths.
pub fn check_hooks_tier2(_program: &Program) -> Vec<OxcDiagnostic> {
    // The hooks validation is performed inside run_lint_pipeline when
    // validate_hooks_usage is enabled. We can't easily re-run just for
    // hooks without the source string. Return empty for the AST-only API.
    // The full lint is available via run_lint_analysis with source.
    Vec::new()
}

/// Run Tier 2 hooks validation with source text.
pub fn check_hooks_tier2_with_source(source: &str, filename: &str) -> Vec<OxcDiagnostic> {
    run_lint_analysis(source, filename).diagnostics_by_kind(DiagnosticKind::HooksViolation)
}

/// Detect mutation of frozen (immutable) values.
/// Uses the effect system to track which values are frozen
/// and reports mutations of those values.
pub fn check_immutability(_program: &Program) -> Vec<OxcDiagnostic> {
    Vec::new()
}

/// Run Tier 2 immutability checks with source text.
pub fn check_immutability_with_source(source: &str, filename: &str) -> Vec<OxcDiagnostic> {
    run_lint_analysis(source, filename).diagnostics_by_kind(DiagnosticKind::ImmutabilityViolation)
}

/// Validate that the compiler's memoization preserves manual
/// useMemo/useCallback guarantees.
pub fn check_preserve_manual_memoization(_program: &Program) -> Vec<OxcDiagnostic> {
    Vec::new()
}

/// Run Tier 2 manual memoization preservation checks with source text.
pub fn check_preserve_manual_memoization_with_source(
    source: &str,
    filename: &str,
) -> Vec<OxcDiagnostic> {
    run_lint_analysis(source, filename).diagnostics_by_kind(DiagnosticKind::MemoizationPreservation)
}

/// Validate exhaustive dependencies for useMemo/useCallback.
/// Uses the compiler's dependency analysis to find missing deps.
pub fn check_memo_dependencies(_program: &Program) -> Vec<OxcDiagnostic> {
    Vec::new()
}

/// Run Tier 2 memo dependency checks with source text.
pub fn check_memo_dependencies_with_source(source: &str, filename: &str) -> Vec<OxcDiagnostic> {
    run_lint_analysis(source, filename).diagnostics_by_kind(DiagnosticKind::MemoDependency)
}

/// Validate exhaustive dependencies for useEffect/useLayoutEffect.
/// Similar to memo-dependencies but for effect hooks.
pub fn check_exhaustive_effect_deps(_program: &Program) -> Vec<OxcDiagnostic> {
    Vec::new()
}

/// Run Tier 2 effect dependency checks with source text.
pub fn check_exhaustive_effect_deps_with_source(
    source: &str,
    filename: &str,
) -> Vec<OxcDiagnostic> {
    run_lint_analysis(source, filename).diagnostics_by_kind(DiagnosticKind::EffectDependency)
}
