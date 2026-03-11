#![allow(dead_code)]
use napi_derive::napi;

#[napi(object)]
pub struct TransformResult {
    pub code: String,
    pub transformed: bool,
}

#[napi(object)]
pub struct TransformOptions {
    pub compilation_mode: Option<String>,
    pub output_mode: Option<String>,
}

#[napi]
pub fn transform_react_file(
    source: String,
    filename: String,
    options: Option<TransformOptions>,
) -> TransformResult {
    let plugin_options = match options {
        Some(opts) => {
            let mut po = oxc_react_compiler::PluginOptions::default();
            if let Some(mode) = opts.compilation_mode.as_deref() {
                po.compilation_mode = match mode {
                    "all" => oxc_react_compiler::entrypoint::options::CompilationMode::All,
                    "syntax" => oxc_react_compiler::entrypoint::options::CompilationMode::Syntax,
                    "annotation" => {
                        oxc_react_compiler::entrypoint::options::CompilationMode::Annotation
                    }
                    _ => oxc_react_compiler::entrypoint::options::CompilationMode::Infer,
                };
            }
            po
        }
        None => oxc_react_compiler::PluginOptions::default(),
    };

    let result = oxc_react_compiler::compile_program(&source, &filename, &plugin_options);

    TransformResult {
        code: result.code,
        transformed: result.transformed,
    }
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
    let allocator = oxc_allocator::Allocator::default();
    let source_type = oxc_span::SourceType::from_path(&filename).unwrap_or_default();
    let parser_ret = oxc_parser::Parser::new(&allocator, &source, source_type).parse();

    if parser_ret.panicked {
        return LintResult {
            diagnostics: vec![],
        };
    }

    let oxc_diagnostics = oxc_react_compiler_lint::run_lint_rules(&parser_ret.program);

    let diagnostics = oxc_diagnostics
        .into_iter()
        .map(|d| {
            let (start, end) = d
                .labels
                .as_ref()
                .and_then(|labels| labels.first())
                .map(|label| {
                    let s = label.offset() as u32;
                    let e = s + label.len() as u32;
                    (s, e)
                })
                .unwrap_or((0, 0));
            LintDiagnostic {
                message: d.message.to_string(),
                start,
                end,
            }
        })
        .collect();

    LintResult { diagnostics }
}
