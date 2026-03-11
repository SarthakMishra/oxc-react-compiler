use oxc_react_compiler::compile_program;
use oxc_react_compiler::PluginOptions;

#[test]
fn test_simple_component() {
    let source = r#"
function Counter({ count }) {
    return <div>{count}</div>;
}
"#;
    let result = compile_program(source, "test.tsx", &PluginOptions::default());
    // The compiler currently returns source unchanged (transformed: false)
    // since the full pipeline isn't wired end-to-end yet.
    // This test verifies the pipeline doesn't crash.
    assert!(!result.code.is_empty());
    insta::assert_snapshot!("simple_component", result.code);
}

#[test]
fn test_hook_component() {
    let source = r#"
function useCounter() {
    const [count, setCount] = useState(0);
    return { count, increment: () => setCount(count + 1) };
}
"#;
    let result = compile_program(source, "test.tsx", &PluginOptions::default());
    assert!(!result.code.is_empty());
    insta::assert_snapshot!("hook_component", result.code);
}

#[test]
fn test_no_memo_directive() {
    let source = r#"
function NoMemo() {
    "use no memo";
    return <div>hello</div>;
}
"#;
    let result = compile_program(source, "test.tsx", &PluginOptions::default());
    // Function with "use no memo" should not be compiled
    assert!(!result.transformed);
}

#[test]
fn test_empty_file() {
    let result = compile_program("", "test.tsx", &PluginOptions::default());
    assert!(!result.transformed);
}

#[test]
fn test_non_component() {
    let source = "const x = 1 + 2;";
    let result = compile_program(source, "test.ts", &PluginOptions::default());
    assert!(!result.transformed);
}

#[test]
fn test_compilation_mode_all() {
    let source = r#"
function helper(x) {
    return x * 2;
}
"#;
    let mut options = PluginOptions::default();
    options.compilation_mode = oxc_react_compiler::entrypoint::options::CompilationMode::All;
    let result = compile_program(source, "test.ts", &options);
    assert!(!result.code.is_empty());
}
