#![allow(clippy::print_stdout, clippy::print_stderr)]

use oxc_react_compiler::{
    CompilationMode, EnvironmentConfig, PanicThreshold, PluginOptions, compile_program_with_config,
};

fn main() {
    let fixture_name = std::env::args().nth(1).unwrap_or_else(|| "simple.js".to_string());
    let path = format!("tests/conformance/upstream-fixtures/{fixture_name}");
    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Failed to read {path}: {e}");
        std::process::exit(1);
    });
    let options = PluginOptions {
        compilation_mode: CompilationMode::All,
        panic_threshold: PanicThreshold::AllErrors,
        ..Default::default()
    };
    let env_config = EnvironmentConfig::default();
    let result = compile_program_with_config(&source, &fixture_name, &options, &env_config);
    println!("Transformed: {}", result.transformed);
    if !result.diagnostics.is_empty() {
        for d in &result.diagnostics {
            println!("  DIAG: {d}");
        }
    }
    println!("---OUTPUT---");
    println!("{}", result.code);
}
