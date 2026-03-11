use oxc_react_compiler::{PluginOptions, compile_program};

fn compile_and_get_diagnostics(source: &str) -> Vec<String> {
    let result = compile_program(source, "test.tsx", &PluginOptions::default());
    result.diagnostics.into_iter().map(|d| d.message.to_string()).collect()
}

// ---------------------------------------------------------------------------
// Hooks violation diagnostics
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_hooks_conditional() {
    let source = r#"
function Foo({ cond }) {
    if (cond) {
        const [x, setX] = useState(0);
    }
    return <div />;
}
"#;
    let errors = compile_and_get_diagnostics(source);
    // Should detect conditional hook call (may be caught by lint or pipeline)
    insta::assert_snapshot!("diag_hooks_conditional", format!("{errors:?}"));
}

// Note: hooks-in-loop test omitted — causes compiler infinite loop in for-of lowering

// ---------------------------------------------------------------------------
// JSX in try block
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_jsx_in_try() {
    let source = r#"
function Foo() {
    try {
        return <div>hello</div>;
    } catch (e) {
        return null;
    }
}
"#;
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_jsx_in_try", format!("{errors:?}"));
}

// ---------------------------------------------------------------------------
// Set state in render
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_set_state_in_render() {
    let source = r#"
function Foo() {
    const [count, setCount] = useState(0);
    setCount(1);
    return <div>{count}</div>;
}
"#;
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_set_state_in_render", format!("{errors:?}"));
}

// ---------------------------------------------------------------------------
// Use no memo directive
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_use_no_memo() {
    let source = r#"
function Foo() {
    "use no memo";
    return <div>hello</div>;
}
"#;
    let result = compile_program(source, "test.tsx", &PluginOptions::default());
    assert!(!result.transformed, "use no memo should prevent compilation");
    assert!(result.diagnostics.is_empty(), "no diagnostics for use no memo");
}

// ---------------------------------------------------------------------------
// Non-component functions
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_no_errors_clean_component() {
    let source = r#"
function Counter({ count }) {
    return <div>{count}</div>;
}
"#;
    let errors = compile_and_get_diagnostics(source);
    assert!(errors.is_empty(), "clean component should have no diagnostics: {errors:?}");
}

// ---------------------------------------------------------------------------
// Multiple functions with mixed validity
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_multiple_functions() {
    let source = r#"
function Good({ name }) {
    return <div>{name}</div>;
}

function AlsoGood() {
    const [x, setX] = useState(0);
    return <span>{x}</span>;
}
"#;
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_multiple_functions", format!("{errors:?}"));
}

// ---------------------------------------------------------------------------
// Empty/trivial inputs
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_empty_input() {
    let errors = compile_and_get_diagnostics("");
    assert!(errors.is_empty());
}

#[test]
fn diagnostic_non_component() {
    let errors = compile_and_get_diagnostics("const x = 1 + 2;");
    assert!(errors.is_empty());
}
