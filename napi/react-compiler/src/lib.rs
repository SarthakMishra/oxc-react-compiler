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
