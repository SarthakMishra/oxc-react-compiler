#![allow(dead_code, clippy::cast_possible_truncation, clippy::needless_pass_by_value)]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use napi_derive::napi;

#[napi(object)]
pub struct TransformResult {
    pub code: String,
    pub transformed: bool,
    /// JSON-serialized v3 source map, if source maps are enabled.
    pub source_map: Option<String>,
}

#[napi(object)]
pub struct TransformOptions {
    pub compilation_mode: Option<String>,
    pub output_mode: Option<String>,
    /// Enable source map generation.
    pub source_map: Option<bool>,
    /// Import source for gating function (e.g., "my-flags").
    pub gating_import_source: Option<String>,
    /// Function name for gating check (e.g., "isCompilerEnabled").
    pub gating_function_name: Option<String>,
}

#[napi]
pub fn transform_react_file(
    source: String,
    filename: String,
    options: Option<TransformOptions>,
) -> TransformResult {
    let source_fallback = source.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let generate_source_map = options.as_ref().and_then(|o| o.source_map).unwrap_or(false);

        let plugin_options = match options {
            Some(opts) => {
                let mut po = oxc_react_compiler::PluginOptions::default();
                if let Some(mode) = opts.compilation_mode.as_deref() {
                    po.compilation_mode = match mode {
                        "all" => oxc_react_compiler::entrypoint::options::CompilationMode::All,
                        "syntax" => {
                            oxc_react_compiler::entrypoint::options::CompilationMode::Syntax
                        }
                        "annotation" => {
                            oxc_react_compiler::entrypoint::options::CompilationMode::Annotation
                        }
                        _ => oxc_react_compiler::entrypoint::options::CompilationMode::Infer,
                    };
                }
                if let (Some(import_source), Some(function_name)) =
                    (opts.gating_import_source, opts.gating_function_name)
                {
                    po.gating = Some(oxc_react_compiler::entrypoint::options::GatingConfig {
                        import_source,
                        function_name,
                    });
                }
                po
            }
            None => oxc_react_compiler::PluginOptions::default(),
        };

        let result = if generate_source_map {
            oxc_react_compiler::compile_program_with_source_map(&source, &filename, &plugin_options)
        } else {
            oxc_react_compiler::compile_program(&source, &filename, &plugin_options)
        };

        TransformResult {
            code: result.code,
            transformed: result.transformed,
            source_map: result.source_map,
        }
    }));

    result.unwrap_or(TransformResult {
        code: source_fallback,
        transformed: false,
        source_map: None,
    })
}

#[napi(object)]
pub struct TransformTimedResult {
    pub code: String,
    pub transformed: bool,
    pub source_map: Option<String>,
    /// Rust-side compilation time in nanoseconds (excludes NAPI marshalling).
    pub rust_compile_ns: i64,
}

#[napi]
pub fn transform_react_file_timed(
    source: String,
    filename: String,
    options: Option<TransformOptions>,
) -> TransformTimedResult {
    let source_fallback = source.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let generate_source_map = options.as_ref().and_then(|o| o.source_map).unwrap_or(false);

        let plugin_options = match options {
            Some(opts) => {
                let mut po = oxc_react_compiler::PluginOptions::default();
                if let Some(mode) = opts.compilation_mode.as_deref() {
                    po.compilation_mode = match mode {
                        "all" => oxc_react_compiler::entrypoint::options::CompilationMode::All,
                        "syntax" => {
                            oxc_react_compiler::entrypoint::options::CompilationMode::Syntax
                        }
                        "annotation" => {
                            oxc_react_compiler::entrypoint::options::CompilationMode::Annotation
                        }
                        _ => oxc_react_compiler::entrypoint::options::CompilationMode::Infer,
                    };
                }
                if let (Some(import_source), Some(function_name)) =
                    (opts.gating_import_source, opts.gating_function_name)
                {
                    po.gating = Some(oxc_react_compiler::entrypoint::options::GatingConfig {
                        import_source,
                        function_name,
                    });
                }
                po
            }
            None => oxc_react_compiler::PluginOptions::default(),
        };

        let start = std::time::Instant::now();
        let result = if generate_source_map {
            oxc_react_compiler::compile_program_with_source_map(&source, &filename, &plugin_options)
        } else {
            oxc_react_compiler::compile_program(&source, &filename, &plugin_options)
        };
        let elapsed = start.elapsed();

        TransformTimedResult {
            code: result.code,
            transformed: result.transformed,
            source_map: result.source_map,
            rust_compile_ns: elapsed.as_nanos() as i64,
        }
    }));

    result.unwrap_or(TransformTimedResult {
        code: source_fallback,
        transformed: false,
        source_map: None,
        rust_compile_ns: 0,
    })
}

#[napi(object)]
pub struct LintResult {
    pub diagnostics: Vec<LintDiagnostic>,
}

#[napi(object)]
pub struct LintDiagnostic {
    pub message: String,
    pub start: u32,
    pub end: u32,
}

#[napi]
pub fn lint_react_file(source: String, filename: String) -> LintResult {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let allocator = oxc_allocator::Allocator::default();
        let source_type = oxc_span::SourceType::from_path(&filename).unwrap_or_default();
        let parser_ret = oxc_parser::Parser::new(&allocator, &source, source_type).parse();

        if parser_ret.panicked {
            return LintResult { diagnostics: vec![] };
        }

        let oxc_diagnostics = oxc_react_compiler_lint::run_all_lint_rules(&parser_ret.program);

        let diagnostics = oxc_diagnostics
            .into_iter()
            .map(|d| {
                let (start, end) =
                    d.labels.as_ref().and_then(|labels| labels.first()).map_or((0, 0), |label| {
                        let s = label.offset() as u32;
                        let e = s + label.len() as u32;
                        (s, e)
                    });
                LintDiagnostic { message: d.message.to_string(), start, end }
            })
            .collect();

        LintResult { diagnostics }
    }));

    result.unwrap_or(LintResult { diagnostics: vec![] })
}
