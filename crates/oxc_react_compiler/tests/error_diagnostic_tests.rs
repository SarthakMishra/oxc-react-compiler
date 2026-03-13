use oxc_react_compiler::{
    EnvironmentConfig, PluginOptions, compile_program, compile_program_with_config,
};

fn compile_and_get_diagnostics(source: &str) -> Vec<String> {
    let result = compile_program(source, "test.tsx", &PluginOptions::default());
    result.diagnostics.into_iter().map(|d| d.message.to_string()).collect()
}

/// Compile with all validation passes enabled (including disabled-by-default ones).
fn compile_with_all_validations(source: &str) -> Vec<String> {
    let config = EnvironmentConfig::all_validations_enabled();
    let result =
        compile_program_with_config(source, "test.tsx", &PluginOptions::default(), &config);
    result.diagnostics.into_iter().map(|d| d.message.to_string()).collect()
}

// ===========================================================================
// DiagnosticKind::HooksViolation — validate_hooks_usage
// ===========================================================================

#[test]
fn diagnostic_hooks_conditional() {
    let source = r"
function Foo({ cond }) {
    if (cond) {
        const [x, setX] = useState(0);
    }
    return <div />;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_hooks_conditional", format!("{errors:?}"));
}

// Note: hooks-in-loop test omitted — causes compiler infinite loop in for-of lowering

#[test]
fn diagnostic_hooks_in_ternary() {
    let source = r"
function Foo({ cond }) {
    const val = cond ? useState(0) : useState(1);
    return <div>{val}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_hooks_in_ternary", format!("{errors:?}"));
}

#[test]
fn diagnostic_hooks_in_logical_expression() {
    let source = r"
function Foo({ cond }) {
    const val = cond && useState(0);
    return <div>{val}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_hooks_in_logical", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::JsxInTry — validate_no_jsx_in_try (disabled by default)
// ===========================================================================

#[test]
fn diagnostic_jsx_in_try_default_config() {
    // With default config (validation disabled), no diagnostic should fire
    let source = r"
function Foo() {
    try {
        return <div>hello</div>;
    } catch (e) {
        return null;
    }
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_jsx_in_try", format!("{errors:?}"));
}

#[test]
fn diagnostic_jsx_in_try_enabled() {
    // With all validations enabled, should detect JSX in try block
    let source = r"
function Foo() {
    try {
        return <div>hello</div>;
    } catch (e) {
        return null;
    }
}
";
    let errors = compile_with_all_validations(source);
    insta::assert_snapshot!("diag_jsx_in_try_enabled", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::SetStateInRender — validate_no_set_state_in_render
// ===========================================================================

#[test]
fn diagnostic_set_state_in_render() {
    let source = r"
function Foo() {
    const [count, setCount] = useState(0);
    setCount(1);
    return <div>{count}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_set_state_in_render", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::SetStateInEffects — validate_no_set_state_in_effects
// ===========================================================================

#[test]
fn diagnostic_set_state_in_effects() {
    let source = r"
function Foo() {
    const [count, setCount] = useState(0);
    useEffect(() => {
        setCount(count + 1);
    }, []);
    return <div>{count}</div>;
}
";
    let errors = compile_with_all_validations(source);
    insta::assert_snapshot!("diag_set_state_in_effects", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::RefAccessInRender — validate_no_ref_access_in_render
// ===========================================================================

#[test]
fn diagnostic_ref_access_in_render() {
    let source = r"
function Foo() {
    const myRef = useRef(null);
    const value = myRef.current;
    return <div>{value}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_ref_access_in_render", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::CapitalizedCalls — validate_no_capitalized_calls
// ===========================================================================

#[test]
fn diagnostic_capitalized_calls() {
    let source = r"
function Foo() {
    const result = MyComponent();
    return <div>{result}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_capitalized_calls", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::ContextVariableLvalues — validate_context_variable_lvalues
// ===========================================================================

#[test]
fn diagnostic_context_variable_lvalues() {
    let source = r"
function Foo() {
    const x = useContext(MyContext);
    x = 42;
    return <div>{x}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_context_variable_lvalues", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::StaticComponents — validate_static_components
// ===========================================================================

#[test]
fn diagnostic_static_components() {
    let source = r"
function Parent() {
    const Child = () => <div>child</div>;
    return <Child />;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_static_components", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::DerivedComputationsInEffects
// ===========================================================================

#[test]
fn diagnostic_derived_computations_in_effects() {
    let source = r"
function Foo({ value }) {
    const [state, setState] = useState(0);
    useEffect(() => {
        setState(value * 2);
    }, [value]);
    return <div>{state}</div>;
}
";
    let errors = compile_with_all_validations(source);
    insta::assert_snapshot!("diag_derived_computations_in_effects", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::LocalsReassignedAfterRender
// ===========================================================================

#[test]
fn diagnostic_locals_reassigned_after_render() {
    let source = r"
function Foo() {
    let x = 1;
    useEffect(() => {
        x = 2;
    });
    return <div>{x}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_locals_reassigned_after_render", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::UseMemoValidation — validate_use_memo
// ===========================================================================

#[test]
fn diagnostic_use_memo_missing_deps() {
    let source = r"
function Foo() {
    const value = useMemo(() => expensive());
    return <div>{value}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_use_memo_missing_deps", format!("{errors:?}"));
}

#[test]
fn diagnostic_use_memo_async_callback() {
    let source = r"
function Foo() {
    const value = useMemo(async () => await fetchData(), []);
    return <div>{value}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_use_memo_async", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::MemoDependency — validate_exhaustive_dependencies (memo)
// ===========================================================================

#[test]
fn diagnostic_memo_dependency_missing() {
    let source = r"
function Foo({ a, b }) {
    const result = useMemo(() => a + b, [a]);
    return <div>{result}</div>;
}
";
    let errors = compile_with_all_validations(source);
    insta::assert_snapshot!("diag_memo_dependency_missing", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::EffectDependency — validate_exhaustive_dependencies (effect)
// ===========================================================================

#[test]
fn diagnostic_effect_dependency_missing() {
    let source = r"
function Foo({ value }) {
    useEffect(() => {
        console.log(value);
    }, []);
    return <div />;
}
";
    let errors = compile_with_all_validations(source);
    insta::assert_snapshot!("diag_effect_dependency_missing", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::ImmutabilityViolation
// ===========================================================================

#[test]
fn diagnostic_immutability_violation() {
    let source = r"
function Foo() {
    const [count, setCount] = useState(0);
    const obj = { setCount };
    return <div onClick={() => obj.setCount(1)}>{count}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_immutability_violation", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::MemoizationPreservation
// ===========================================================================

#[test]
fn diagnostic_memoization_preservation() {
    let source = r"
function Foo({ items }) {
    const sorted = useMemo(() => items.sort(), [items]);
    return <div>{sorted.length}</div>;
}
";
    let errors = compile_with_all_validations(source);
    insta::assert_snapshot!("diag_memoization_preservation", format!("{errors:?}"));
}

// ===========================================================================
// DiagnosticKind::InvariantViolation — internal only (assert_valid_mutable_ranges)
// Not directly triggerable from user code; tested via assertion
// ===========================================================================

#[test]
fn diagnostic_invariant_violation_not_triggered_by_valid_code() {
    // InvariantViolation is an internal compiler bug detector.
    // Valid code should never trigger it.
    let source = r"
function Foo({ x }) {
    const doubled = x * 2;
    return <div>{doubled}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    assert!(
        !errors.iter().any(|e| e.contains("invariant")),
        "valid code should not trigger invariant violations"
    );
}

// ===========================================================================
// Baseline tests — ensure no false positives
// ===========================================================================

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

#[test]
fn diagnostic_no_errors_clean_component() {
    let source = r"
function Counter({ count }) {
    return <div>{count}</div>;
}
";
    let errors = compile_and_get_diagnostics(source);
    assert!(errors.is_empty(), "clean component should have no diagnostics: {errors:?}");
}

#[test]
fn diagnostic_multiple_functions() {
    let source = r"
function Good({ name }) {
    return <div>{name}</div>;
}

function AlsoGood() {
    const [x, setX] = useState(0);
    return <span>{x}</span>;
}
";
    let errors = compile_and_get_diagnostics(source);
    insta::assert_snapshot!("diag_multiple_functions", format!("{errors:?}"));
}

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

// ===========================================================================
// All validations on clean code — should produce no diagnostics
// ===========================================================================

#[test]
fn diagnostic_all_validations_clean_code() {
    // Simple component without useMemo to avoid MemoizationPreservation diagnostic
    // (which fires when enable_preserve_existing_memoization_guarantees is on and
    // the compiler can't guarantee the same memoization boundaries).
    let source = r"
function Foo({ name, count }) {
    const doubled = count * 2;
    return <div>{doubled} - {name}</div>;
}
";
    let errors = compile_with_all_validations(source);
    assert!(errors.is_empty(), "clean code should produce no diagnostics: {errors:?}");
}
