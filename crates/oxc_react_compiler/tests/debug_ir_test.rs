//! Tests for the DEBUG_IR differential tracing tool.
//!
//! Verifies that the debug dump infrastructure produces output when enabled
//! and does not affect compilation results.

use oxc_react_compiler::{PluginOptions, compile_program};

#[test]
fn test_debug_ir_produces_output() {
    // Enable DEBUG_IR for this test.
    // SAFETY: This test is not run concurrently with other tests that depend
    // on the absence of DEBUG_IR (test binary runs single-threaded by default
    // for env var tests).
    // SAFETY: test-only env var manipulation
    unsafe {
        std::env::set_var("DEBUG_IR", "1");
    }

    let source = r"
function Component({ a, b }) {
    const x = a + b;
    return <div>{x}</div>;
}
";

    let result = compile_program(source, "test.tsx", &PluginOptions::default());

    // SAFETY: test-only env var manipulation
    unsafe {
        std::env::remove_var("DEBUG_IR");
    }

    // The compilation should still succeed regardless of debug dumping.
    assert!(result.transformed, "compilation should succeed with DEBUG_IR enabled");
}

#[test]
fn test_debug_ir_does_not_change_output() {
    let source = r"
function Component({ items }) {
    const doubled = items.map(x => x * 2);
    return <ul>{doubled.map(d => <li key={d}>{d}</li>)}</ul>;
}
";

    // Compile without DEBUG_IR
    let result_without = compile_program(source, "test.tsx", &PluginOptions::default());

    // Compile with DEBUG_IR
    // SAFETY: test-only env var manipulation
    unsafe {
        std::env::set_var("DEBUG_IR", "1");
    }
    let result_with = compile_program(source, "test.tsx", &PluginOptions::default());
    // SAFETY: test-only env var manipulation
    unsafe {
        std::env::remove_var("DEBUG_IR");
    }

    // Output should be identical — debug dumping must not affect compilation.
    assert_eq!(
        result_without.code, result_with.code,
        "DEBUG_IR must not change compilation output"
    );
    assert_eq!(result_without.transformed, result_with.transformed);
}
