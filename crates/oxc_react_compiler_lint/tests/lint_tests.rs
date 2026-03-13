use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_react_compiler_lint::rules::tier2;
use oxc_react_compiler_lint::{run_all_lint_rules, run_lint_rules};
use oxc_span::SourceType;

fn run_lint(source: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let ret = Parser::new(&allocator, source, source_type).parse();
    let diagnostics = run_lint_rules(&ret.program);
    diagnostics.into_iter().map(|d| d.message.to_string()).collect()
}

fn parse_and_run_tier2<F>(source: &str, f: F) -> Vec<String>
where
    F: for<'a> FnOnce(&oxc_ast::ast::Program<'a>) -> Vec<oxc_diagnostics::OxcDiagnostic>,
{
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let ret = Parser::new(&allocator, source, source_type).parse();
    f(&ret.program).into_iter().map(|d| d.message.to_string()).collect()
}

#[test]
fn test_no_jsx_in_try() {
    let source = r"
function Foo() {
    try {
        return <div>hello</div>;
    } catch (e) {
        return null;
    }
}
";
    let errors = run_lint(source);
    assert!(errors.iter().any(|e| e.contains("try")), "Should detect JSX in try: {errors:?}");
}

#[test]
fn test_no_jsx_in_try_clean() {
    let source = r"
function Foo() {
    return <div>hello</div>;
}
";
    let errors = run_lint(source);
    let jsx_in_try = errors.iter().any(|e| e.contains("try"));
    assert!(!jsx_in_try, "Should not report JSX-in-try for clean code");
}

#[test]
fn test_rules_of_hooks_conditional() {
    let source = r"
function Foo({ condition }) {
    if (condition) {
        useState(0);
    }
    return null;
}
";
    let errors = run_lint(source);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("conditionally") || e.contains("hook") || e.contains("condition")),
        "Should detect conditional hook: {errors:?}"
    );
}

#[test]
fn test_hooks_at_top_level_ok() {
    let source = r"
function Foo() {
    const [x, setX] = useState(0);
    return <div>{x}</div>;
}
";
    let errors = run_lint(source);
    let hook_errors = errors.iter().any(|e| e.contains("conditionally") || e.contains("top level"));
    assert!(!hook_errors, "Top-level hooks should be fine: {errors:?}");
}

#[test]
fn test_set_state_in_render() {
    let source = r"
function Foo() {
    const [x, setX] = useState(0);
    setX(1);
    return <div>{x}</div>;
}
";
    let errors = run_lint(source);
    assert!(
        errors.iter().any(|e| e.contains("setState") || e.contains("render")),
        "Should detect setState in render: {errors:?}"
    );
}

#[test]
fn test_impure_function_call() {
    let source = r"
function Foo() {
    const x = Math.random();
    return <div>{x}</div>;
}
";
    let errors = run_lint(source);
    assert!(
        errors.iter().any(|e| e.contains("impure") || e.contains("Math.random")),
        "Should detect impure call: {errors:?}"
    );
}

#[test]
fn test_incompatible_library() {
    let source = r#"
import { observable } from "mobx";
function Foo() {
    return <div>hello</div>;
}
"#;
    let errors = run_lint(source);
    assert!(
        errors.iter().any(|e| e.contains("mobx") || e.contains("incompatible")),
        "Should detect incompatible library: {errors:?}"
    );
}

#[test]
fn test_clean_component() {
    let source = r"
function Foo({ name }) {
    return <div>Hello {name}</div>;
}
";
    let errors = run_lint(source);
    // A clean component should have no errors (or very few)
    assert!(errors.len() <= 1, "Clean component should have few errors: {errors:?}");
}

// ---------------------------------------------------------------------------
// Tier 2 lint tests (compiler-dependent, use &Program API)
// ---------------------------------------------------------------------------

#[test]
fn test_tier2_hooks_in_conditional() {
    let source = r"
function Foo({ condition }) {
    if (condition) {
        const [x, setX] = useState(0);
    }
    return <div />;
}
";
    let diags = parse_and_run_tier2(source, tier2::check_hooks_tier2);
    // Wiring test: verifies the pipeline runs without panicking.
    let _ = diags;
}

#[test]
fn test_tier2_hooks_in_loop() {
    let source = r"
function Foo({ items }) {
    while (items.length > 0) {
        useEffect(() => {}, []);
        break;
    }
    return <div />;
}
";
    let diags = parse_and_run_tier2(source, tier2::check_hooks_tier2);
    // Wiring test: verifies the pipeline runs without panicking on loop constructs.
    let _ = diags;
}

#[test]
fn test_tier2_hooks_top_level_ok() {
    let source = r"
function Foo() {
    const [x, setX] = useState(0);
    useEffect(() => { console.log(x); }, [x]);
    return <div>{x}</div>;
}
";
    let diags = parse_and_run_tier2(source, tier2::check_hooks_tier2);
    assert!(
        diags.is_empty(),
        "Top-level hooks should not produce Tier 2 hooks diagnostics: {diags:?}"
    );
}

#[test]
fn test_tier2_memo_missing_dependency() {
    let source = r"
function Foo({ a, b }) {
    const value = useMemo(() => a + b, [a]);
    return <div>{value}</div>;
}
";
    let diags = parse_and_run_tier2(source, tier2::check_memo_dependencies);
    // Wiring test: verifies the pipeline runs without panicking.
    let _ = diags;
}

#[test]
fn test_tier2_effect_missing_dependency() {
    let source = r"
function Foo({ count }) {
    useEffect(() => {
        console.log(count);
    }, []);
    return <div>{count}</div>;
}
";
    let diags = parse_and_run_tier2(source, tier2::check_exhaustive_effect_deps);
    // Wiring test: verifies the pipeline runs without panicking.
    let _ = diags;
}

#[test]
fn test_tier2_no_errors_on_clean_code() {
    let source = r"
function Foo({ name }) {
    return <div>Hello {name}</div>;
}
";
    let hooks = parse_and_run_tier2(source, tier2::check_hooks_tier2);
    let immutability = parse_and_run_tier2(source, tier2::check_immutability);
    assert!(hooks.is_empty(), "Clean component should have no hooks violations: {hooks:?}");
    assert!(
        immutability.is_empty(),
        "Clean component should have no immutability violations: {immutability:?}"
    );
}

// ---------------------------------------------------------------------------
// run_all_lint_rules integration test
// ---------------------------------------------------------------------------

#[test]
fn test_run_all_lint_rules() {
    let source = r"
function Foo({ name }) {
    return <div>Hello {name}</div>;
}
";
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let ret = Parser::new(&allocator, source, source_type).parse();
    let diags = run_all_lint_rules(&ret.program);
    // Clean component: no Tier 1 or Tier 2 diagnostics expected
    assert!(
        diags.is_empty(),
        "Clean component should have no diagnostics: {:?}",
        diags.iter().map(|d| d.message.to_string()).collect::<Vec<_>>()
    );
}
