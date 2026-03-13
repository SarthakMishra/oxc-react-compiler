use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_react_compiler::{PluginOptions, compile_program};
use oxc_semantic::SemanticBuilder;
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

/// Known globals that are expected to be unresolved in compiled output.
const KNOWN_GLOBALS: &[&str] = &[
    // React hooks and APIs
    "useState",
    "useEffect",
    "useRef",
    "useMemo",
    "useCallback",
    "useContext",
    "useReducer",
    "useLayoutEffect",
    "useImperativeHandle",
    "useDebugValue",
    "useDeferredValue",
    "useTransition",
    "useId",
    "useSyncExternalStore",
    "useInsertionEffect",
    "useOptimistic",
    "useFormStatus",
    "useActionState",
    "React",
    "ReactDOM",
    "Fragment",
    // Browser globals
    "console",
    "document",
    "window",
    "setTimeout",
    "setInterval",
    "clearTimeout",
    "clearInterval",
    "requestAnimationFrame",
    "fetch",
    "Promise",
    "Error",
    "Map",
    "Set",
    "Symbol",
    "JSON",
    "Math",
    "Object",
    "Array",
    "String",
    "Number",
    "Boolean",
    "Date",
    "RegExp",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "undefined",
    "NaN",
    "Infinity",
    "globalThis",
    "alert",
    "confirm",
    "prompt",
    // Common test/fixture globals
    "expensive",
    "fetchData",
    "MyContext",
    "MyComponent",
];

/// Validate that compiled output has no unexpected unresolved references.
///
/// Uses oxc_semantic to run scope analysis on the compiled output and checks
/// that all referenced identifiers either have a binding or are known globals.
fn validate_no_unresolved_refs(code: &str) -> Vec<String> {
    if code.is_empty() {
        return Vec::new();
    }
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let ret = Parser::new(&allocator, code, source_type).parse();

    if ret.panicked {
        return vec!["Failed to parse compiled output for semantic analysis".to_string()];
    }

    let semantic_ret = SemanticBuilder::new().build(&ret.program);

    let mut errors = Vec::new();
    let scoping = semantic_ret.semantic.scoping();
    let unresolved = scoping.root_unresolved_references();

    // Also collect semantic errors (e.g., duplicate declarations)
    for err in &semantic_ret.errors {
        errors.push(format!("Semantic error: {}", err.message));
    }

    for (name, _ref_ids) in unresolved {
        let name_str = name.as_str();
        // Skip known globals
        if KNOWN_GLOBALS.contains(&name_str) {
            continue;
        }
        // Skip compiler runtime import (_c) and compiler temporaries (_temp*)
        if name_str == "_c" || name_str.starts_with("_temp") {
            continue;
        }
        errors.push(format!("Unresolved reference: `{name_str}`"));
    }

    errors.sort();
    errors
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
// Parse validation: compiled output should be valid JS/TS
// ---------------------------------------------------------------------------

#[test]
fn codegen_valid_simple_component() {
    let (transformed, errors) = compile_and_validate(
        r"
function Counter({ count }) {
    return <div>{count}</div>;
}
",
    );
    assert!(transformed);
    assert!(errors.is_empty(), "codegen output should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_hook() {
    let (transformed, errors) = compile_and_validate(
        r"
function useToggle(initial) {
    const [value, setValue] = useState(initial);
    return [value, () => setValue(!value)];
}
",
    );
    assert!(transformed);
    assert!(errors.is_empty(), "hook codegen should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_conditional() {
    let (transformed, errors) = compile_and_validate(
        r"
function Toggle({ isOn }) {
    if (isOn) {
        return <div>ON</div>;
    }
    return <div>OFF</div>;
}
",
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
        r"
export function App({ title }) {
    return <h1>{title}</h1>;
}
",
    );
    assert!(transformed);
    assert!(errors.is_empty(), "exported component codegen should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_export_default() {
    let (transformed, errors) = compile_and_validate(
        r"
export default function Page({ content }) {
    return <main>{content}</main>;
}
",
    );
    assert!(transformed);
    assert!(errors.is_empty(), "export default codegen should be parseable: {errors:?}");
}

#[test]
fn codegen_valid_multiple_components() {
    let (transformed, errors) = compile_and_validate(
        r"
function Header({ title }) {
    return <h1>{title}</h1>;
}
function Footer({ text }) {
    return <footer>{text}</footer>;
}
",
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
        r"
const Button = ({ onClick, label }) => {
    return <button onClick={onClick}>{label}</button>;
};
",
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

// ---------------------------------------------------------------------------
// Semantic validation: unresolved reference detection via oxc_semantic
//
// These tests use insta snapshots to document the current state of codegen.
// When codegen bugs are fixed, the snapshots should update to show fewer
// (ideally zero) unresolved references.
// ---------------------------------------------------------------------------

#[test]
fn semantic_simple_component() {
    let result = compile_program(
        r"
function Greeting({ name }) {
    return <div>Hello, {name}!</div>;
}
",
        "test.tsx",
        &PluginOptions::default(),
    );
    assert!(result.transformed);
    let unresolved = validate_no_unresolved_refs(&result.code);
    insta::assert_snapshot!("semantic_simple_component", format!("{unresolved:?}"));
}

#[test]
fn semantic_hook_with_state() {
    let result = compile_program(
        r"
function Counter() {
    const [count, setCount] = useState(0);
    return <button onClick={() => setCount(count + 1)}>{count}</button>;
}
",
        "test.tsx",
        &PluginOptions::default(),
    );
    assert!(result.transformed);
    let unresolved = validate_no_unresolved_refs(&result.code);
    insta::assert_snapshot!("semantic_hook_with_state", format!("{unresolved:?}"));
}

#[test]
fn semantic_derived_values() {
    let result = compile_program(
        r#"
function Summary({ items }) {
    const total = items.length;
    const hasItems = total > 0;
    return <div>{hasItems ? total : "none"}</div>;
}
"#,
        "test.tsx",
        &PluginOptions::default(),
    );
    assert!(result.transformed);
    let unresolved = validate_no_unresolved_refs(&result.code);
    insta::assert_snapshot!("semantic_derived_values", format!("{unresolved:?}"));
}

#[test]
fn semantic_conditional_component() {
    let result = compile_program(
        r#"
function App({ user }) {
    const [active, setActive] = useState(true);
    const greeting = active ? "Hello" : "Goodbye";
    return (
        <div>
            <span>{greeting}, {user.name}</span>
            <button onClick={() => setActive(!active)}>Toggle</button>
        </div>
    );
}
"#,
        "test.tsx",
        &PluginOptions::default(),
    );
    assert!(result.transformed);
    let unresolved = validate_no_unresolved_refs(&result.code);
    insta::assert_snapshot!("semantic_conditional_component", format!("{unresolved:?}"));
}

#[test]
fn semantic_multiple_components() {
    let result = compile_program(
        r"
function Header({ title }) {
    return <h1>{title}</h1>;
}
function Footer({ text }) {
    return <footer>{text}</footer>;
}
",
        "test.tsx",
        &PluginOptions::default(),
    );
    assert!(result.transformed);
    let unresolved = validate_no_unresolved_refs(&result.code);
    insta::assert_snapshot!("semantic_multiple_components", format!("{unresolved:?}"));
}

#[test]
fn semantic_arrow_component() {
    let result = compile_program(
        r"
const Button = ({ onClick, label }) => {
    return <button onClick={onClick}>{label}</button>;
};
",
        "test.tsx",
        &PluginOptions::default(),
    );
    assert!(result.transformed);
    let unresolved = validate_no_unresolved_refs(&result.code);
    insta::assert_snapshot!("semantic_arrow_component", format!("{unresolved:?}"));
}

#[test]
fn test_color_picker_no_hang() {
    let source =
        std::fs::read_to_string("../../benchmarks/fixtures/realworld/color-picker.tsx").unwrap();
    let result = compile_program(&source, "color-picker.tsx", &PluginOptions::default());
    // If we reach this point, the compiler completed without hanging.
    let _ = result;
}

#[test]
fn test_availability_schedule_no_hang() {
    let source =
        std::fs::read_to_string("../../benchmarks/fixtures/realworld/availability-schedule.tsx")
            .unwrap();
    let result = compile_program(&source, "availability-schedule.tsx", &PluginOptions::default());
    // If we reach this point, the compiler completed without hanging.
    let _ = result;
}
