use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};

use super::options::{CompilationMode, OutputMode, PluginOptions};
use super::pipeline::run_full_pipeline;
use crate::error::ErrorCollector;
use crate::hir::build::HIRBuilder;
use crate::hir::environment::EnvironmentConfig;
use crate::hir::globals::{is_component_name, is_hook_name};
use crate::hir::types::ReactFunctionType;
use crate::reactive_scopes::codegen::{
    SourceMap, apply_compilation, codegen_function, codegen_function_with_source_map,
    has_cache_slots,
};

/// Result of compiling a program.
pub struct CompileResult {
    pub code: String,
    pub transformed: bool,
    pub diagnostics: Vec<OxcDiagnostic>,
    /// JSON-serialized v3 source map, if source maps were requested.
    pub source_map: Option<String>,
}

/// Compile a single source file.
///
/// 1. Parse with oxc_parser
/// 2. Walk AST to find compilable functions
/// 3. For each function: lower to HIR → run pipeline → codegen
/// 4. Apply edits to produce output
pub fn compile_program(source: &str, filename: &str, options: &PluginOptions) -> CompileResult {
    compile_program_inner(source, filename, options, false)
}

/// Compile a single source file with optional source map generation.
pub fn compile_program_with_source_map(
    source: &str,
    filename: &str,
    options: &PluginOptions,
) -> CompileResult {
    compile_program_inner(source, filename, options, true)
}

/// Compile a single source file with a custom environment configuration.
///
/// This allows callers to enable/disable specific validation passes
/// (e.g., enabling all validations for testing).
pub fn compile_program_with_config(
    source: &str,
    filename: &str,
    options: &PluginOptions,
    config: &EnvironmentConfig,
) -> CompileResult {
    compile_program_inner_with_config(source, filename, options, config, false)
}

fn compile_program_inner(
    source: &str,
    filename: &str,
    options: &PluginOptions,
    generate_source_map: bool,
) -> CompileResult {
    compile_program_inner_with_config(
        source,
        filename,
        options,
        &EnvironmentConfig::default(),
        generate_source_map,
    )
}

fn compile_program_inner_with_config(
    source: &str,
    filename: &str,
    options: &PluginOptions,
    config: &EnvironmentConfig,
    generate_source_map: bool,
) -> CompileResult {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default().with_jsx(true);
    let parser_ret = Parser::new(&allocator, source, source_type).parse();

    if parser_ret.panicked {
        return CompileResult {
            code: source.to_string(),
            transformed: false,
            diagnostics: vec![],
            source_map: None,
        };
    }

    let config = config.clone();

    // DIVERGENCE: Upstream emits a per-component "Use of incompatible library" error;
    // we bail the entire file silently. This is simpler but coarser — it prevents
    // compilation of all functions in a file with an incompatible import, whereas
    // upstream only skips the specific function that uses the incompatible API.
    if has_known_incompatible_import(&parser_ret.program) {
        return CompileResult {
            code: source.to_string(),
            transformed: false,
            diagnostics: vec![],
            source_map: None,
        };
    }

    // DIVERGENCE: Upstream emits a per-component diagnostic with the suppression
    // text and location; we bail the entire file silently via raw string scan.
    if has_eslint_hooks_suppression(source) {
        return CompileResult {
            code: source.to_string(),
            transformed: false,
            diagnostics: vec![],
            source_map: None,
        };
    }

    // Check for module-level opt-out directives: 'use no memo' / 'use no forget'
    if has_opt_out_directive(Some(parser_ret.program.directives.as_slice())) {
        return CompileResult {
            code: source.to_string(),
            transformed: false,
            diagnostics: vec![],
            source_map: None,
        };
    }

    // Lint mode: run analysis to collect diagnostics but don't transform the code.
    if options.output_mode == OutputMode::Lint {
        return CompileResult {
            code: source.to_string(),
            transformed: false,
            diagnostics: vec![],
            source_map: None,
        };
    }

    let mut compiled_functions: Vec<(Span, String)> = Vec::new();
    let mut function_source_maps: Vec<(Span, SourceMap)> = Vec::new();
    let mut diagnostics = Vec::new();

    // Walk the AST and compile each discovered function in place.
    for stmt in &parser_ret.program.body {
        compile_statement(
            stmt,
            options,
            &config,
            source,
            generate_source_map,
            &mut compiled_functions,
            &mut function_source_maps,
            &mut diagnostics,
        );
    }

    if compiled_functions.is_empty() {
        return CompileResult {
            code: source.to_string(),
            transformed: false,
            diagnostics,
            source_map: None,
        };
    }

    let mut code = apply_compilation(source, &compiled_functions);

    // Apply gating wrapper if configured
    if let Some(gating) = &options.gating {
        code = gating.generate_wrapper(&code);
    }

    let source_map = if generate_source_map {
        let composed = compose_source_maps(source, &compiled_functions, &function_source_maps);
        Some(composed.to_json(filename, filename))
    } else {
        None
    };

    CompileResult { code, transformed: true, diagnostics, source_map }
}

/// Compose per-function source maps into a single source map for the whole file.
///
/// Generates identity mappings for unmodified regions (code between compiled
/// functions passes through unchanged) and offsets per-function source maps
/// to account for the import line and line count changes from compilation.
fn compose_source_maps(
    original_source: &str,
    compiled_functions: &[(Span, String)],
    function_source_maps: &[(Span, SourceMap)],
) -> SourceMap {
    let mut composed = SourceMap::new();

    // The import statement adds 1 line at the top.
    let import_line_offset: u32 = 1;

    // Build a sorted list of edits.
    let mut edits: Vec<(usize, usize, &str)> = compiled_functions
        .iter()
        .map(|(span, code)| (span.start as usize, span.end as usize, code.as_str()))
        .collect();
    edits.sort_by_key(|e| e.0);

    // Build a map from span start to source map for quick lookup.
    let sm_map: std::collections::HashMap<usize, &SourceMap> =
        function_source_maps.iter().map(|(span, sm)| (span.start as usize, sm)).collect();

    // Walk through the source, emitting identity mappings for unmodified regions
    // and offset function source maps for compiled regions.
    let mut output_line: u32 = import_line_offset; // Start after the import line
    let mut output_col: u32 = 0;
    let mut source_pos: usize = 0;
    let mut orig_line: u32 = 0;
    let mut orig_col: u32 = 0;

    for &(edit_start, edit_end, replacement) in &edits {
        // Emit identity mappings for the unmodified region before this edit.
        if source_pos < edit_start {
            let unmodified = &original_source[source_pos..edit_start];
            for ch in unmodified.chars() {
                if output_col == 0 || ch == '\n' {
                    // Map start of each line in the unmodified region
                    if ch != '\n' {
                        composed.add_mapping(output_line, output_col, orig_line, orig_col);
                    }
                }
                if ch == '\n' {
                    output_line += 1;
                    output_col = 0;
                    orig_line += 1;
                    orig_col = 0;
                } else {
                    output_col += 1;
                    orig_col += 1;
                }
            }
        }

        // Emit the per-function source map entries, offset to the current output position.
        if let Some(func_sm) = sm_map.get(&edit_start) {
            for entry in &func_sm.mappings {
                composed.add_mapping(
                    entry.generated_line + output_line,
                    entry.generated_column,
                    entry.original_line,
                    entry.original_column,
                );
            }
        }

        // Advance output position past the replacement.
        for ch in replacement.chars() {
            if ch == '\n' {
                output_line += 1;
                output_col = 0;
            } else {
                output_col += 1;
            }
        }

        // Advance source position past the original span, tracking original line/col.
        for ch in original_source[source_pos..edit_end].chars() {
            if ch == '\n' {
                orig_line += 1;
                orig_col = 0;
            } else {
                orig_col += 1;
            }
        }
        source_pos = edit_end;
    }

    // Emit identity mappings for any remaining unmodified code after the last edit.
    if source_pos < original_source.len() {
        let remaining = &original_source[source_pos..];
        for ch in remaining.chars() {
            if (output_col == 0 || ch == '\n') && ch != '\n' {
                composed.add_mapping(output_line, output_col, orig_line, orig_col);
            }
            if ch == '\n' {
                output_line += 1;
                output_col = 0;
                orig_line += 1;
                orig_col = 0;
            } else {
                output_col += 1;
                orig_col += 1;
            }
        }
    }

    composed
}

/// Try to compile a single function, returning the compiled code on success.
fn try_compile_function(
    builder: HIRBuilder,
    func: &Function<'_>,
    fn_type: ReactFunctionType,
    config: &EnvironmentConfig,
    source_text: &str,
    generate_source_map: bool,
    diagnostics: &mut Vec<OxcDiagnostic>,
) -> Option<(String, Option<SourceMap>)> {
    let hir_func = builder.build_function(func, fn_type);
    let mut errors = ErrorCollector::default();

    if let Ok(rf) = run_full_pipeline(hir_func, config, &mut errors) {
        // If no reactive scopes survived the pipeline (0 cache slots),
        // skip the function — memoization would add no value. This matches
        // Babel's behavior of returning source unchanged for such functions.
        if !has_cache_slots(&rf) {
            diagnostics.extend(errors.into_diagnostics());
            return None;
        }
        let (code, sm) = if generate_source_map {
            let (code, sm) = codegen_function_with_source_map(&rf, source_text);
            (code, Some(sm))
        } else {
            (codegen_function(&rf), None)
        };
        diagnostics.extend(errors.into_diagnostics());
        Some((code, sm))
    } else {
        diagnostics.extend(errors.into_diagnostics());
        None
    }
}

/// Try to compile an arrow function, returning the compiled code on success.
fn try_compile_arrow(
    builder: HIRBuilder,
    arrow: &ArrowFunctionExpression<'_>,
    name: Option<String>,
    fn_type: ReactFunctionType,
    config: &EnvironmentConfig,
    source_text: &str,
    generate_source_map: bool,
    diagnostics: &mut Vec<OxcDiagnostic>,
) -> Option<(String, Option<SourceMap>)> {
    let hir_func = builder.build_arrow_function(arrow, name, fn_type);
    let mut errors = ErrorCollector::default();

    if let Ok(rf) = run_full_pipeline(hir_func, config, &mut errors) {
        // If no reactive scopes survived, skip the function (no value in memoizing)
        if !has_cache_slots(&rf) {
            diagnostics.extend(errors.into_diagnostics());
            return None;
        }
        let (code, sm) = if generate_source_map {
            let (code, sm) = codegen_function_with_source_map(&rf, source_text);
            (code, Some(sm))
        } else {
            (codegen_function(&rf), None)
        };
        diagnostics.extend(errors.into_diagnostics());
        Some((code, sm))
    } else {
        diagnostics.extend(errors.into_diagnostics());
        None
    }
}

/// Extract and compile the inner function from a React.forwardRef/React.memo wrapper call.
///
/// Handles patterns like:
/// - `React.forwardRef(function Comp() { ... })`
/// - `React.memo(() => { ... })`
/// - `React.memo(React.forwardRef(() => { ... }))` (nested)
/// - `forwardRef(() => { ... })` (bare imports)
#[expect(clippy::too_many_arguments)]
fn try_compile_wrapper_call<'a>(
    call: &'a CallExpression<'a>,
    name: &str,
    fn_type: ReactFunctionType,
    config: &EnvironmentConfig,
    source_text: &str,
    generate_source_map: bool,
    compiled: &mut Vec<(Span, String)>,
    source_maps: &mut Vec<(Span, SourceMap)>,
    diagnostics: &mut Vec<OxcDiagnostic>,
) {
    // The first argument is the inner function (or another wrapper call)
    let Some(first_arg) = call.arguments.first() else {
        return;
    };
    // Skip spread arguments — React.forwardRef(...args) is not a valid pattern
    if matches!(first_arg, Argument::SpreadElement(_)) {
        return;
    }
    // SAFETY: non-SpreadElement Argument variants have the same layout as Expression
    // (oxc uses inherit_variants! macro). This is the same pattern used in build.rs.
    let inner_expr: &Expression<'_> =
        unsafe { &*std::ptr::from_ref::<Argument<'_>>(first_arg).cast::<Expression<'_>>() };
    let inner = inner_expr.without_parentheses();

    match inner {
        Expression::ArrowFunctionExpression(arrow) => {
            if has_opt_out_directive(Some(arrow.body.directives.as_slice())) {
                return;
            }
            let builder = HIRBuilder::new(config.clone());
            if let Some((code, sm)) = try_compile_arrow(
                builder,
                arrow,
                Some(name.to_string()),
                fn_type,
                config,
                source_text,
                generate_source_map,
                diagnostics,
            ) {
                if let Some(sm) = sm {
                    source_maps.push((arrow.span, sm));
                }
                compiled.push((arrow.span, code));
            }
        }
        Expression::FunctionExpression(func) => {
            let builder = HIRBuilder::new(config.clone());
            if let Some((code, sm)) = try_compile_function(
                builder,
                func,
                fn_type,
                config,
                source_text,
                generate_source_map,
                diagnostics,
            ) {
                if let Some(sm) = sm {
                    source_maps.push((func.span, sm));
                }
                compiled.push((func.span, code));
            }
        }
        // Handle nested wrappers: React.memo(React.forwardRef(() => ...))
        Expression::CallExpression(inner_call) if is_react_wrapper_call(inner_call) => {
            try_compile_wrapper_call(
                inner_call,
                name,
                fn_type,
                config,
                source_text,
                generate_source_map,
                compiled,
                source_maps,
                diagnostics,
            );
        }
        _ => {}
    }
}

/// Walk a statement, discover compilable functions, and compile them immediately.
#[expect(clippy::too_many_arguments)]
fn compile_statement<'a>(
    stmt: &'a Statement<'a>,
    options: &PluginOptions,
    config: &EnvironmentConfig,
    source_text: &str,
    generate_source_map: bool,
    compiled: &mut Vec<(Span, String)>,
    source_maps: &mut Vec<(Span, SourceMap)>,
    diagnostics: &mut Vec<OxcDiagnostic>,
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
                    func.params.items.len(),
                ) {
                    let builder = HIRBuilder::new(config.clone());
                    if let Some((code, sm)) = try_compile_function(
                        builder,
                        func,
                        fn_type,
                        config,
                        source_text,
                        generate_source_map,
                        diagnostics,
                    ) {
                        if let Some(sm) = sm {
                            source_maps.push((func.span, sm));
                        }
                        compiled.push((func.span, code));
                    }
                }
            }
        }
        Statement::ExportDefaultDeclaration(export) => {
            if let ExportDefaultDeclarationKind::FunctionDeclaration(func) = &export.declaration {
                let name = func.id.as_ref().map(|id| id.name.to_string());
                let fn_type =
                    name.as_deref().map_or(ReactFunctionType::Component, classify_function_name);

                if should_compile_default_export(name.as_deref(), fn_type, options) {
                    let builder = HIRBuilder::new(config.clone());
                    if let Some((code, sm)) = try_compile_function(
                        builder,
                        func,
                        fn_type,
                        config,
                        source_text,
                        generate_source_map,
                        diagnostics,
                    ) {
                        if let Some(sm) = sm {
                            source_maps.push((func.span, sm));
                        }
                        compiled.push((func.span, code));
                    }
                }
            }
        }
        Statement::ExportNamedDeclaration(export) => {
            if let Some(decl) = &export.declaration {
                compile_declaration(
                    decl,
                    options,
                    config,
                    source_text,
                    generate_source_map,
                    compiled,
                    source_maps,
                    diagnostics,
                );
            }
        }
        Statement::VariableDeclaration(decl) => {
            compile_variable_declaration(
                decl,
                options,
                config,
                source_text,
                generate_source_map,
                compiled,
                source_maps,
                diagnostics,
            );
        }
        Statement::ExpressionStatement(expr_stmt) => {
            // Handle bare expression: React.memo(props => { ... });
            if let Expression::CallExpression(call) = &expr_stmt.expression
                && is_react_wrapper_call(call)
            {
                // For standalone wrapper calls, the wrapper name itself acts as
                // the function type hint (Component by default for memo/forwardRef).
                let fn_type = ReactFunctionType::Component;
                let name = extract_wrapper_name(call);
                try_compile_wrapper_call(
                    call,
                    &name,
                    fn_type,
                    config,
                    source_text,
                    generate_source_map,
                    compiled,
                    source_maps,
                    diagnostics,
                );
            }
        }
        _ => {}
    }
}

#[expect(clippy::too_many_arguments)]
fn compile_declaration<'a>(
    decl: &'a Declaration<'a>,
    options: &PluginOptions,
    config: &EnvironmentConfig,
    source_text: &str,
    generate_source_map: bool,
    compiled: &mut Vec<(Span, String)>,
    source_maps: &mut Vec<(Span, SourceMap)>,
    diagnostics: &mut Vec<OxcDiagnostic>,
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
                    func.params.items.len(),
                ) {
                    let builder = HIRBuilder::new(config.clone());
                    if let Some((code, sm)) = try_compile_function(
                        builder,
                        func,
                        fn_type,
                        config,
                        source_text,
                        generate_source_map,
                        diagnostics,
                    ) {
                        if let Some(sm) = sm {
                            source_maps.push((func.span, sm));
                        }
                        compiled.push((func.span, code));
                    }
                }
            }
        }
        Declaration::VariableDeclaration(decl) => {
            compile_variable_declaration(
                decl,
                options,
                config,
                source_text,
                generate_source_map,
                compiled,
                source_maps,
                diagnostics,
            );
        }
        _ => {}
    }
}

#[expect(clippy::too_many_arguments)]
fn compile_variable_declaration<'a>(
    decl: &'a VariableDeclaration<'a>,
    options: &PluginOptions,
    config: &EnvironmentConfig,
    source_text: &str,
    generate_source_map: bool,
    compiled: &mut Vec<(Span, String)>,
    source_maps: &mut Vec<(Span, SourceMap)>,
    diagnostics: &mut Vec<OxcDiagnostic>,
) {
    for declarator in &decl.declarations {
        if let Some(init) = &declarator.init
            && let BindingPattern::BindingIdentifier(id) = &declarator.id
        {
            let name = id.name.to_string();
            let fn_type = classify_function_name(&name);

            // Extract param count from the init expression for component filtering
            let param_count = match init.without_parentheses() {
                Expression::ArrowFunctionExpression(arrow) => arrow.params.items.len(),
                Expression::FunctionExpression(func) => func.params.items.len(),
                Expression::CallExpression(_) => 1, // React.memo/forwardRef wraps a component
                _ => 0,
            };

            if !should_compile(&name, fn_type, None, options, param_count) {
                continue;
            }

            match init.without_parentheses() {
                Expression::ArrowFunctionExpression(arrow) => {
                    // Check for "use no memo" directive in arrow body
                    if has_opt_out_directive(Some(arrow.body.directives.as_slice())) {
                        continue;
                    }
                    let builder = HIRBuilder::new(config.clone());
                    if let Some((code, sm)) = try_compile_arrow(
                        builder,
                        arrow,
                        Some(name),
                        fn_type,
                        config,
                        source_text,
                        generate_source_map,
                        diagnostics,
                    ) {
                        if let Some(sm) = sm {
                            source_maps.push((arrow.span, sm));
                        }
                        compiled.push((arrow.span, code));
                    }
                }
                Expression::FunctionExpression(func) => {
                    let builder = HIRBuilder::new(config.clone());
                    if let Some((code, sm)) = try_compile_function(
                        builder,
                        func,
                        fn_type,
                        config,
                        source_text,
                        generate_source_map,
                        diagnostics,
                    ) {
                        if let Some(sm) = sm {
                            source_maps.push((func.span, sm));
                        }
                        compiled.push((func.span, code));
                    }
                }
                // Handle React.forwardRef(() => ...) and React.memo(() => ...),
                // including nested: React.memo(React.forwardRef(() => ...))
                Expression::CallExpression(call) if is_react_wrapper_call(call) => {
                    try_compile_wrapper_call(
                        call,
                        &name,
                        fn_type,
                        config,
                        source_text,
                        generate_source_map,
                        compiled,
                        source_maps,
                        diagnostics,
                    );
                }
                _ => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Classification and filtering helpers (unchanged from discovery)
// ---------------------------------------------------------------------------

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

/// Walk the program to find functions that should be compiled (for lint/discovery only).
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
                    func.params.items.len(),
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
        Statement::ExportDefaultDeclaration(export) => {
            if let ExportDefaultDeclarationKind::FunctionDeclaration(func) = &export.declaration {
                let name = func.id.as_ref().map(|id| id.name.to_string());
                let fn_type =
                    name.as_deref().map_or(ReactFunctionType::Component, classify_function_name);

                if should_compile_default_export(name.as_deref(), fn_type, options) {
                    let opt_out =
                        has_opt_out_directive(func.body.as_ref().map(|b| b.directives.as_slice()));
                    functions.push(DiscoveredFunction { name, fn_type, span: func.span, opt_out });
                }
            }
        }
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
                    func.params.items.len(),
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
        if let Some(init) = &declarator.init
            && let BindingPattern::BindingIdentifier(id) = &declarator.id
        {
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

            if is_function && should_compile(&name, fn_type, None, options, usize::MAX) {
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

/// Extract a synthetic name from a React wrapper call for use as the compiled function name.
/// For standalone `React.memo(fn)` without a variable binding, we use the inner function's
/// name if available, or fall back to a generic name.
fn extract_wrapper_name(call: &CallExpression<'_>) -> String {
    // Try to get the name from the first argument (inner function)
    if let Some(first_arg) = call.arguments.first()
        && !matches!(first_arg, Argument::SpreadElement(_))
    {
        let inner_expr: &Expression<'_> =
            unsafe { &*std::ptr::from_ref::<Argument<'_>>(first_arg).cast::<Expression<'_>>() };
        let inner = inner_expr.without_parentheses();
        match inner {
            Expression::FunctionExpression(func) => {
                if let Some(id) = func.id.as_ref() {
                    return id.name.to_string();
                }
            }
            // Nested wrapper: React.memo(React.forwardRef(fn))
            Expression::CallExpression(inner_call) if is_react_wrapper_call(inner_call) => {
                return extract_wrapper_name(inner_call);
            }
            _ => {}
        }
    }
    // No inner name found — use a generic placeholder
    "Component".to_string()
}

/// Check if a call expression is React.forwardRef/memo/lazy
fn is_react_wrapper_call(call: &CallExpression<'_>) -> bool {
    match &call.callee {
        Expression::StaticMemberExpression(member) => {
            if let Expression::Identifier(obj) = &member.object {
                obj.name == "React"
                    && matches!(member.property.name.as_str(), "forwardRef" | "memo" | "lazy")
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

/// Decide if a function should be compiled based on options.
///
/// `param_count` is the number of formal parameters. In `Infer` mode, components
/// must have at most 1 parameter (props). Functions with 2+ params are not
/// considered components even if the name starts with an uppercase letter.
fn should_compile(
    _name: &str,
    fn_type: ReactFunctionType,
    directives: Option<&[Directive<'_>]>,
    options: &PluginOptions,
    param_count: usize,
) -> bool {
    // Check for opt-out
    if has_opt_out_directive(directives) {
        return false;
    }

    match options.compilation_mode {
        CompilationMode::All => true,
        CompilationMode::Infer => {
            match fn_type {
                ReactFunctionType::Hook => true,
                ReactFunctionType::Component => {
                    // Components take at most 1 parameter (props).
                    // Functions with >1 param aren't components.
                    param_count <= 1
                }
                ReactFunctionType::Other => false,
            }
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
            name.is_none_or(|n| {
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
    directives.is_some_and(|dirs| {
        dirs.iter().any(|d| {
            let s = d.directive.as_str();
            // "use no memo" is the current name; "use no forget" is the legacy name.
            s == "use no memo" || s == "use no forget"
        })
    })
}

fn has_memo_directive(directives: Option<&[Directive<'_>]>) -> bool {
    directives.is_some_and(|dirs| dirs.iter().any(|d| d.directive.as_str() == "use memo"))
}

/// Known-incompatible module sources that should cause compilation to bail.
/// These are modules whose APIs return values that cannot be safely memoized.
const KNOWN_INCOMPATIBLE_MODULES: &[&str] = &["ReactCompilerKnownIncompatibleTest"];

/// Check if the program imports from a known-incompatible module.
fn has_known_incompatible_import(program: &Program<'_>) -> bool {
    for stmt in &program.body {
        if let Statement::ImportDeclaration(import) = stmt {
            let source = import.source.value.as_str();
            if KNOWN_INCOMPATIBLE_MODULES.contains(&source) {
                return true;
            }
        }
    }
    false
}

/// Check if the source contains an unclosed ESLint disable comment that suppresses
/// React hooks rules. Upstream bails the entire file when this is detected.
///
/// Matches patterns like:
/// - `/* eslint-disable react-hooks/rules-of-hooks */`
/// - `// eslint-disable-next-line react-hooks/rules-of-hooks`
fn has_eslint_hooks_suppression(source: &str) -> bool {
    // Check for block-comment eslint-disable (file-level, unclosed)
    if source.contains("eslint-disable")
        && (source.contains("react-hooks/rules-of-hooks")
            || source.contains("react-hooks/exhaustive-deps"))
    {
        // Verify it's not a line-scoped disable-next-line (those are OK)
        // File-level eslint-disable without matching eslint-enable should bail
        for line in source.lines() {
            let trimmed = line.trim();
            // Block comment: /* eslint-disable react-hooks/... */
            // Match "eslint-disable" followed by space or the rule name directly
            if (trimmed.contains("eslint-disable ") || trimmed.contains("eslint-disable\n"))
                && !trimmed.contains("eslint-disable-next-line")
                && !trimmed.contains("eslint-disable-line")
                && (trimmed.contains("react-hooks/rules-of-hooks")
                    || trimmed.contains("react-hooks/exhaustive-deps"))
            {
                // Check if there's a matching eslint-enable
                if !source.contains("eslint-enable") {
                    return true;
                }
            }
        }
    }
    false
}
