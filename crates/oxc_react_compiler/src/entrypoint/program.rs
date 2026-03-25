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
use rustc_hash::FxHashSet;

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
    // DIVERGENCE: Preprocess Flow component/hook syntax to regular functions.
    // Flow's `component Foo(bar: number) {}` is equivalent to
    // `function Foo({bar}: {bar: number}) {}`. Babel converts these before
    // the React Compiler sees them; we do a lightweight text-level conversion
    // because OXC's parser doesn't support Flow component syntax.
    let preprocessed;
    let source = if has_flow_component_or_hook_syntax(source) {
        preprocessed = preprocess_flow_syntax(source);
        preprocessed.as_str()
    } else {
        source
    };

    let allocator = Allocator::default();
    // Always enable TypeScript and JSX parsing regardless of file extension.
    // Many upstream fixtures use `.js` extension but contain TypeScript or Flow
    // type annotations. OXC's TypeScript parser tolerates plain JS gracefully,
    // so enabling it unconditionally is safe and avoids silent parse failures.
    let source_type =
        SourceType::from_path(filename).unwrap_or_default().with_jsx(true).with_typescript(true);
    let parser_ret = Parser::new(&allocator, source, source_type).parse();

    if parser_ret.panicked {
        return CompileResult {
            code: source.to_string(),
            transformed: false,
            diagnostics: vec![],
            source_map: None,
        };
    }

    let mut config = config.clone();

    // Thread the panic threshold from plugin options into the environment config
    // so the pipeline can use it for bail-out decisions.
    config.bail_threshold = options.panic_threshold;

    // DIVERGENCE: Upstream emits a per-component "Use of incompatible library" diagnostic
    // but still compiles the function. We also compile (no bail) to match conformance.
    // The incompatible import check is preserved as `has_known_incompatible_import()` for
    // potential future use in diagnostics.

    // Skip files that already import from the compiler runtime.
    // These have already been compiled by React Compiler and should not
    // be double-compiled. Upstream checks for `useMemoCache` usage
    // within each function; we bail the entire file if the runtime import exists.
    if has_compiler_runtime_import(&parser_ret.program) {
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

    // DIVERGENCE: Upstream bails per-function on custom ESLint suppression rules,
    // not per-file. We no longer bail the whole file — instead we compile normally.
    // The custom suppression check is preserved for future per-function use.
    // TODO: Implement per-function suppression check in compile_function().

    // Check for module-level opt-out directives: 'use no memo' / 'use no forget'
    if !options.ignore_use_no_forget
        && has_opt_out_directive(
            Some(parser_ret.program.directives.as_slice()),
            &options.custom_opt_out_directives,
        )
    {
        return CompileResult {
            code: source.to_string(),
            transformed: false,
            diagnostics: vec![],
            source_map: None,
        };
    }

    // Null mode: skip compilation entirely, return source unchanged.
    // Upstream uses this for testing pipeline overhead without transformation.
    if options.output_mode == OutputMode::Null {
        return CompileResult {
            code: source.to_string(),
            transformed: false,
            diagnostics: vec![],
            source_map: None,
        };
    }

    // DIVERGENCE: Upstream lint mode still compiles (adds memoization) while also
    // collecting lint diagnostics. We do the same to match conformance expectations.
    // In a production lint-only context, callers can discard the transformed code.

    // Collect hook aliases: local names that alias hook imports
    // (e.g., `import { useFragment as readFragment }` → "readFragment" is a hook alias)
    config.hook_aliases = collect_hook_aliases(&parser_ret.program);

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

    let code = apply_compilation(source, &compiled_functions, options.gating.as_ref());

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
        // skip the function — memoization would add no value.
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
    options: &PluginOptions,
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
            if !options.ignore_use_no_forget
                && has_opt_out_directive(
                    Some(arrow.body.directives.as_slice()),
                    &options.custom_opt_out_directives,
                )
            {
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
                        // When gating is enabled, wrap the function in a ternary:
                        //   `const Name = gatingFn() ? compiledFn : originalFn;`
                        if let Some(gating) = &options.gating {
                            let original =
                                &source_text[func.span.start as usize..func.span.end as usize];
                            let ternary = gating.wrap_function(&code, original);
                            compiled.push((func.span, format!("const {name} = {ternary}")));
                        } else {
                            compiled.push((func.span, code));
                        }
                    }
                }
            }
        }
        Statement::ExportDefaultDeclaration(export) => {
            if let ExportDefaultDeclarationKind::FunctionDeclaration(func) = &export.declaration {
                let name = func.id.as_ref().map(|id| id.name.to_string());
                let fn_type =
                    name.as_deref().map_or(ReactFunctionType::Component, classify_function_name);
                let directives = func.body.as_ref().map(|b| b.directives.as_slice());

                if should_compile_default_export(name.as_deref(), fn_type, directives, options) {
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
                        // When gating is enabled for export default function:
                        //   `const Name = gatingFn() ? compiledFn : originalFn;\nexport default Name;`
                        // Replace the entire export-default statement span.
                        if let Some(gating) = &options.gating {
                            let original =
                                &source_text[func.span.start as usize..func.span.end as usize];
                            let ternary = gating.wrap_function(&code, original);
                            let fn_name = name.as_deref().unwrap_or("_anonymous");
                            let replacement =
                                format!("const {fn_name} = {ternary};\nexport default {fn_name};");
                            compiled.push((export.span, replacement));
                        } else {
                            compiled.push((func.span, code));
                        }
                    }
                }
            }
        }
        Statement::ExportNamedDeclaration(export) => {
            if let Some(decl) = &export.declaration {
                compile_declaration(
                    decl,
                    export.span,
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
                None,
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
        _ => {}
    }
}

#[expect(clippy::too_many_arguments)]
fn compile_declaration<'a>(
    decl: &'a Declaration<'a>,
    export_span: Span,
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
                        // When gating is enabled for `export function Foo(...)`:
                        //   `export const Foo = gatingFn() ? compiledFn : originalFn;`
                        // Replace the entire export-named statement span.
                        if let Some(gating) = &options.gating {
                            let original =
                                &source_text[func.span.start as usize..func.span.end as usize];
                            let ternary = gating.wrap_function(&code, original);
                            let replacement = format!("export const {name} = {ternary}");
                            compiled.push((export_span, replacement));
                        } else {
                            compiled.push((func.span, code));
                        }
                    }
                }
            }
        }
        Declaration::VariableDeclaration(decl) => {
            compile_variable_declaration(
                decl,
                None,
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
    _export_span: Option<Span>,
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
                    if !options.ignore_use_no_forget
                        && has_opt_out_directive(
                            Some(arrow.body.directives.as_slice()),
                            &options.custom_opt_out_directives,
                        )
                    {
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
                        // When gating is enabled for `const Foo = () => ...`:
                        //   replace the init expression with `gatingFn() ? compiledArrow : originalArrow`
                        if let Some(gating) = &options.gating {
                            let original =
                                &source_text[arrow.span.start as usize..arrow.span.end as usize];
                            let ternary = gating.wrap_function(&code, original);
                            compiled.push((arrow.span, ternary));
                        } else {
                            compiled.push((arrow.span, code));
                        }
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
                        // When gating is enabled for `const Foo = function() {...}`:
                        //   replace the init expression with `gatingFn() ? compiledFn : originalFn`
                        if let Some(gating) = &options.gating {
                            let original =
                                &source_text[func.span.start as usize..func.span.end as usize];
                            let ternary = gating.wrap_function(&code, original);
                            compiled.push((func.span, ternary));
                        } else {
                            compiled.push((func.span, code));
                        }
                    }
                }
                // Handle React.forwardRef(() => ...) and React.memo(() => ...),
                // including nested: React.memo(React.forwardRef(() => ...))
                Expression::CallExpression(call) if is_react_wrapper_call(call) => {
                    try_compile_wrapper_call(
                        call,
                        &name,
                        fn_type,
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
                    let opt_out = has_opt_out_directive(
                        func.body.as_ref().map(|b| b.directives.as_slice()),
                        &options.custom_opt_out_directives,
                    );
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
                let directives = func.body.as_ref().map(|b| b.directives.as_slice());

                if should_compile_default_export(name.as_deref(), fn_type, directives, options) {
                    let opt_out =
                        has_opt_out_directive(directives, &options.custom_opt_out_directives);
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
                    let opt_out = has_opt_out_directive(
                        func.body.as_ref().map(|b| b.directives.as_slice()),
                        &options.custom_opt_out_directives,
                    );
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
    // Check for opt-out (unless @ignoreUseNoForget is set)
    if !options.ignore_use_no_forget
        && has_opt_out_directive(directives, &options.custom_opt_out_directives)
    {
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
    directives: Option<&[Directive<'_>]>,
    options: &PluginOptions,
) -> bool {
    // Check for opt-out (unless @ignoreUseNoForget is set)
    if !options.ignore_use_no_forget
        && has_opt_out_directive(directives, &options.custom_opt_out_directives)
    {
        return false;
    }

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
        CompilationMode::Syntax | CompilationMode::Annotation => {
            // Only compile functions with "use memo" / "use forget" directive
            has_memo_directive(directives)
        }
    }
}

fn has_opt_out_directive(
    directives: Option<&[Directive<'_>]>,
    custom_directives: &[String],
) -> bool {
    directives.is_some_and(|dirs| {
        dirs.iter().any(|d| {
            let s = d.directive.as_str();
            // "use no memo" is the current name; "use no forget" is the legacy name.
            s == "use no memo" || s == "use no forget" || custom_directives.iter().any(|cd| cd == s)
        })
    })
}

fn has_memo_directive(directives: Option<&[Directive<'_>]>) -> bool {
    directives.is_some_and(|dirs| {
        dirs.iter().any(|d| {
            let s = d.directive.as_str();
            // "use memo" is the current name; "use forget" is the legacy name.
            // "use memo if(condition)" is a conditional compilation variant.
            s == "use memo"
                || s == "use forget"
                || s.starts_with("use memo if(")
                || s.starts_with("use forget if(")
        })
    })
}

/// Known-incompatible module sources whose APIs return values that cannot be
/// safely memoized. Retained for future per-function diagnostic use.
const KNOWN_INCOMPATIBLE_MODULES: &[&str] = &["ReactCompilerKnownIncompatibleTest"];

/// Collect local names that alias hook imports.
///
/// When a module uses `import { useFragment as readFragment }`, the local name
/// `readFragment` doesn't match `is_hook_name` (no `use` prefix), but it IS a hook
/// because it aliases `useFragment`. This function finds such aliases so the
/// hooks validation pass can treat them correctly.
fn collect_hook_aliases(program: &Program<'_>) -> FxHashSet<String> {
    let mut aliases = FxHashSet::default();

    for stmt in &program.body {
        if let Statement::ImportDeclaration(import) = stmt
            && let Some(specifiers) = &import.specifiers
        {
            for spec in specifiers {
                if let ImportDeclarationSpecifier::ImportSpecifier(named) = spec {
                    let imported_name = named.imported.name();
                    let local_name = named.local.name.as_str();

                    // If the imported name is a hook but the local name is not,
                    // record the local name as a hook alias.
                    if is_hook_name(imported_name.as_str()) && !is_hook_name(local_name) {
                        aliases.insert(local_name.to_string());
                    }
                }
            }
        }
    }

    aliases
}

/// Check if the program imports from a known-incompatible module.
/// Retained for future per-function diagnostic use.
#[expect(dead_code)]
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

/// Check if the program imports the compiler's memoization cache from the
/// React Compiler runtime (`c` / `_c` / `useMemoCache`).
///
/// If the file already imports the cache function, the code has already been
/// compiled and should not be double-compiled. Upstream checks for `useMemoCache`
/// usage per-function; we do a simpler file-level check on the import specifiers.
///
/// DIVERGENCE: We only bail when the specific cache import (`c` / `useMemoCache`)
/// is present, not on any import from the runtime module. This allows files that
/// import non-compiler utilities from `react/compiler-runtime` to still compile.
fn has_compiler_runtime_import(program: &Program<'_>) -> bool {
    for stmt in &program.body {
        if let Statement::ImportDeclaration(import) = stmt {
            let source = import.source.value.as_str();
            if source == "react/compiler-runtime" || source == "react-compiler-runtime" {
                // Only bail if the import specifically includes the cache function
                if let Some(specifiers) = &import.specifiers {
                    for spec in specifiers {
                        if let oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(s) = spec {
                            let imported = s.imported.name().as_str();
                            if imported == "c" || imported == "useMemoCache" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Check if the source contains any ESLint disable comment that suppresses
/// React hooks rules. Upstream bails per-component when this is detected,
/// regardless of whether there's a matching eslint-enable.
///
/// Also checks custom suppression rules when provided.
///
/// Matches patterns like:
/// - `/* eslint-disable react-hooks/rules-of-hooks */`
/// - `// eslint-disable-next-line react-hooks/rules-of-hooks`
/// - `/* eslint-disable react-hooks/exhaustive-deps */`
fn has_eslint_hooks_suppression(source: &str) -> bool {
    has_eslint_suppression_for_rules(
        source,
        &["react-hooks/rules-of-hooks", "react-hooks/exhaustive-deps"],
    )
}

/// Check if the source contains any ESLint disable comment for the given rules.
fn has_eslint_suppression_for_rules(source: &str, rules: &[&str]) -> bool {
    if !source.contains("eslint-disable") {
        return false;
    }
    // Check if any of the target rules are mentioned alongside eslint-disable
    for rule in rules {
        if source.contains(rule) {
            // Verify it's actually in an eslint-disable context (not just a random mention)
            for line in source.lines() {
                let trimmed = line.trim();
                if trimmed.contains("eslint-disable") && trimmed.contains(rule) {
                    return true;
                }
            }
        }
    }
    false
}

/// Quick check whether source contains Flow component or hook declaration syntax.
/// Used to avoid the preprocessing overhead for non-Flow files.
fn has_flow_component_or_hook_syntax(source: &str) -> bool {
    // Check for `component ` or `hook ` at the start of a line (possibly preceded by `export default ` or `export `)
    for line in source.lines() {
        let trimmed = line.trim();
        let after_export = trimmed
            .strip_prefix("export")
            .map(str::trim_start)
            .and_then(|s| s.strip_prefix("default").map(str::trim_start).or(Some(s)))
            .unwrap_or(trimmed);
        if after_export.starts_with("component ") || after_export.starts_with("hook ") {
            return true;
        }
    }
    false
}

/// Preprocess Flow component/hook syntax into standard function declarations.
///
/// Transforms:
/// - `component Foo(bar: T, baz: U) { ... }` → `function Foo({bar, baz}) { ... }`
/// - `hook useFoo(bar: T) { ... }` → `function useFoo(bar) { ... }`
/// - Handles `export`, `export default` prefixes
///
/// This is a text-level transformation, not a full parser. It handles the common
/// patterns used in upstream fixtures. Complex cases (nested generics in params,
/// render types) may not be handled perfectly.
fn preprocess_flow_syntax(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut pos = 0;

    while pos < source.len() {
        // Find the next potential component/hook declaration
        let remaining = &source[pos..];

        // Try to match at the current position
        if let Some((replacement, consumed)) = try_match_flow_decl(remaining) {
            result.push_str(&replacement);
            pos += consumed;
            // Advance chars iterator
            chars = source[pos..].chars().peekable();
        } else {
            // Copy one character
            if let Some(ch) = chars.next() {
                result.push(ch);
                pos += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    result
}

/// Try to match a Flow component or hook declaration at the start of `s`.
/// Returns (replacement_text, bytes_consumed) if matched.
fn try_match_flow_decl(s: &str) -> Option<(String, usize)> {
    // Only match at the start of a line (or start of input)
    // Check for optional `export` and `export default` prefixes
    let mut cursor = 0;
    let trimmed_start = s[cursor..].trim_start();
    let leading_ws = s.len() - trimmed_start.len();
    cursor = leading_ws;

    let mut prefix = String::new();

    if s[cursor..].starts_with("export") {
        let after_export = &s[cursor + 6..];
        if after_export.starts_with(|c: char| c.is_whitespace()) {
            prefix.push_str("export ");
            cursor += 6;
            // Skip whitespace
            while cursor < s.len()
                && s.as_bytes()[cursor].is_ascii_whitespace()
                && s.as_bytes()[cursor] != b'\n'
            {
                cursor += 1;
            }
            if s[cursor..].starts_with("default") {
                let after_default = &s[cursor + 7..];
                if after_default.starts_with(|c: char| c.is_whitespace()) {
                    prefix.push_str("default ");
                    cursor += 7;
                    while cursor < s.len()
                        && s.as_bytes()[cursor].is_ascii_whitespace()
                        && s.as_bytes()[cursor] != b'\n'
                    {
                        cursor += 1;
                    }
                }
            }
        } else {
            return None;
        }
    }

    let is_component = s[cursor..].starts_with("component ");
    let is_hook = s[cursor..].starts_with("hook ");

    if !is_component && !is_hook {
        return None;
    }

    let keyword_len = if is_component { 10 } else { 5 }; // "component " or "hook "
    cursor += keyword_len;

    // Read the name
    let name_start = cursor;
    while cursor < s.len()
        && (s.as_bytes()[cursor].is_ascii_alphanumeric()
            || s.as_bytes()[cursor] == b'_'
            || s.as_bytes()[cursor] == b'$')
    {
        cursor += 1;
    }
    let name = &s[name_start..cursor];
    if name.is_empty() {
        return None;
    }

    // Skip whitespace
    while cursor < s.len()
        && s.as_bytes()[cursor].is_ascii_whitespace()
        && s.as_bytes()[cursor] != b'\n'
    {
        cursor += 1;
    }

    // Expect '('
    if cursor >= s.len() || s.as_bytes()[cursor] != b'(' {
        return None;
    }
    cursor += 1; // skip '('

    // Read params until matching ')'
    let params_start = cursor;
    let mut depth = 1;
    while cursor < s.len() && depth > 0 {
        match s.as_bytes()[cursor] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth > 0 {
            cursor += 1;
        }
    }
    let params_text = &s[params_start..cursor];
    cursor += 1; // skip closing ')'

    // Parse params: strip Flow type annotations, extract names
    let param_names = extract_flow_param_names(params_text);

    // Build replacement
    let ws = &s[..leading_ws];
    let params_str = if is_component {
        if param_names.is_empty() {
            String::new()
        } else {
            // Wrap in destructuring: {a, b, c}
            format!("{{{}}}", param_names.join(", "))
        }
    } else {
        // Hook: keep params as-is (strip types)
        param_names.join(", ")
    };

    let replacement = format!("{ws}{prefix}function {name}({params_str})");

    Some((replacement, cursor))
}

/// Extract parameter names from a Flow parameter list, stripping type annotations.
///
/// Handles patterns like:
/// - `bar: number` → `bar`
/// - `bar: number, baz: string` → `bar`, `baz`
/// - `bar?: string` → `bar`
/// - `onClose: (isConfirmed: boolean) => void` → `onClose`
/// - Empty params → empty vec
fn extract_flow_param_names(params: &str) -> Vec<&str> {
    let trimmed = params.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut names = Vec::new();
    let bytes = trimmed.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Skip leading whitespace
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        // Read the parameter name
        let name_start = i;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
        {
            i += 1;
        }
        let name = &trimmed[name_start..i];
        if !name.is_empty() {
            names.push(name);
        }

        // Skip '?' if present (optional param)
        if i < bytes.len() && bytes[i] == b'?' {
            i += 1;
        }

        // Skip type annotation: everything up to the next top-level comma
        // Need to track depth of parens, angles, braces
        let mut depth = 0i32;
        while i < bytes.len() {
            match bytes[i] {
                b',' if depth == 0 => {
                    i += 1; // skip comma
                    break;
                }
                b'(' | b'[' | b'{' | b'<' => depth += 1,
                b')' | b']' | b'}' | b'>' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
    }

    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entrypoint::options::GatingConfig;

    #[test]
    fn gating_per_function_ternary() {
        let source = "import {Stringify} from 'shared-runtime';\nconst ErrorView = ({error, _retry}) => <Stringify error={error}></Stringify>;\n\nexport default ErrorView;";
        let opts = PluginOptions {
            gating: Some(GatingConfig {
                import_source: "ReactForgetFeatureFlag".to_string(),
                function_name: "isForgetEnabled_Fixtures".to_string(),
            }),
            ..Default::default()
        };
        let config = EnvironmentConfig::default();
        let result = compile_program_with_config(source, "test.js", &opts, &config);
        assert!(result.transformed);
        assert!(result.code.contains("isForgetEnabled_Fixtures()"));
        // Gating import should be present
        assert!(
            result
                .code
                .contains("import { isForgetEnabled_Fixtures } from \"ReactForgetFeatureFlag\"")
        );
    }

    #[test]
    fn gating_export_default_function() {
        let source = "export default function Bar(props) {\n  'use forget';\n  return <div>{props.bar}</div>;\n}";
        let opts = PluginOptions {
            compilation_mode: CompilationMode::Annotation,
            gating: Some(GatingConfig {
                import_source: "ReactForgetFeatureFlag".to_string(),
                function_name: "isForgetEnabled_Fixtures".to_string(),
            }),
            ..Default::default()
        };
        let config = EnvironmentConfig::default();
        let result = compile_program_with_config(source, "test.js", &opts, &config);
        assert!(result.transformed);
        // Should produce const declaration + ternary + separate export default
        assert!(result.code.contains("const Bar = isForgetEnabled_Fixtures()"));
        assert!(result.code.contains("export default Bar;"));
    }

    #[test]
    fn annotation_mode_export_default_with_memo_directive() {
        // Verify annotation mode compiles export-default functions with 'use forget'
        let source = "export default function Bar(props) {\n  'use forget';\n  return <div>{props.bar}</div>;\n}";
        let opts =
            PluginOptions { compilation_mode: CompilationMode::Annotation, ..Default::default() };
        let config = EnvironmentConfig::default();
        let result = compile_program_with_config(source, "test.js", &opts, &config);
        assert!(
            result.transformed,
            "annotation mode should compile 'use forget' in export-default functions"
        );
    }
}
