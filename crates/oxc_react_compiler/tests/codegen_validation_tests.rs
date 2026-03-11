use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_react_compiler::{PluginOptions, compile_program};
use oxc_span::SourceType;

/// Validate that compiled output is parseable JavaScript/TypeScript.
fn validate_codegen_output(code: &str) -> Vec<String> {
    if code.is_empty() {
        return Vec::new();
    }
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let ret = Parser::new(&allocator, code, source_type).parse();
    ret.errors.iter().map(|e| e.message.to_string()).collect()
}

/// Compile source and validate the output is parseable.
fn compile_and_validate(source: &str) -> (bool, Vec<String>) {
    let result = compile_program(source, "test.tsx", &PluginOptions::default());
    if !result.transformed {
        return (false, Vec::new());
    }
    let parse_errors = validate_codegen_output(&result.code);
    (result.transformed, parse_errors)
}

// ---------------------------------------------------------------------------
// Simple components should produce parseable output
// ---------------------------------------------------------------------------

#[test]
fn codegen_valid_simple_component() {
    let (transformed, errors) = compile_and_validate(
        r#"
function Counter({ count }) {
    return <div>{count}</div>;
}
"#,
    );
    assert!(transformed);
    assert!(errors.is_empty(), "codegen output should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_hook() {
    let (transformed, errors) = compile_and_validate(
        r#"
function useToggle(initial) {
    const [value, setValue] = useState(initial);
    return [value, () => setValue(!value)];
}
"#,
    );
    assert!(transformed);
    assert!(errors.is_empty(), "hook codegen should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_conditional() {
    let (transformed, errors) = compile_and_validate(
        r#"
function Toggle({ isOn }) {
    if (isOn) {
        return <div>ON</div>;
    }
    return <div>OFF</div>;
}
"#,
    );
    assert!(transformed);
    assert!(errors.is_empty(), "conditional codegen should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_derived_value() {
    let (transformed, errors) = compile_and_validate(
        r#"
function Display({ items }) {
    const count = items.length;
    const label = count > 0 ? "Has items" : "Empty";
    return <div>{label}: {count}</div>;
}
"#,
    );
    assert!(transformed);
    assert!(errors.is_empty(), "derived value codegen should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_export() {
    let (transformed, errors) = compile_and_validate(
        r#"
export function App({ title }) {
    return <h1>{title}</h1>;
}
"#,
    );
    assert!(transformed);
    assert!(errors.is_empty(), "exported component codegen should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_export_default() {
    let (transformed, errors) = compile_and_validate(
        r#"
export default function Page({ content }) {
    return <main>{content}</main>;
}
"#,
    );
    assert!(transformed);
    assert!(errors.is_empty(), "export default codegen should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_multiple_components() {
    let (transformed, errors) = compile_and_validate(
        r#"
function Header({ title }) {
    return <h1>{title}</h1>;
}
function Footer({ text }) {
    return <footer>{text}</footer>;
}
"#,
    );
    assert!(transformed);
    assert!(errors.is_empty(), "multiple components codegen should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_jsx_children() {
    let (transformed, errors) = compile_and_validate(
        r#"
function Layout({ children }) {
    return <div className="container">{children}</div>;
}
"#,
    );
    assert!(transformed);
    assert!(errors.is_empty(), "JSX children codegen should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_arrow_component() {
    let (transformed, errors) = compile_and_validate(
        r#"
const Button = ({ onClick, label }) => {
    return <button onClick={onClick}>{label}</button>;
};
"#,
    );
    assert!(transformed);
    assert!(errors.is_empty(), "arrow component codegen should be parseable: {errors:?}");
}

// ---------------------------------------------------------------------------
// Non-transformed inputs should produce no errors
// ---------------------------------------------------------------------------

#[test]
fn codegen_empty_input() {
    let (transformed, errors) = compile_and_validate("");
    assert!(!transformed);
    assert!(errors.is_empty());
}

#[test]
fn codegen_non_component() {
    let (transformed, errors) = compile_and_validate("const x = 1 + 2;");
    assert!(!transformed);
    assert!(errors.is_empty());
}
