//! Property-based fuzz tests for HIR construction (P4 Gap 5).
//!
//! Uses proptest to generate random JavaScript-like source strings and ensures
//! the compiler never panics, regardless of input shape.

use std::fmt::Write as _;

use oxc_react_compiler::{PluginOptions, compile_program};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Generates a PascalCase component name (1–16 chars).
fn arb_component_name() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z]{0,15}"
}

/// Generates a simple identifier for parameters / variables.
fn arb_ident() -> impl Strategy<Value = String> {
    "[a-z][a-zA-Z0-9]{0,7}"
}

/// Generates a simple React-like function component with a `<div />` return.
fn arb_simple_component() -> impl Strategy<Value = String> {
    (arb_component_name(), prop::collection::vec(arb_ident(), 0..5)).prop_map(|(name, params)| {
        let params_str = params.join(", ");
        format!("function {name}({params_str}) {{ return <div />; }}")
    })
}

/// Generates a component that calls one or more hooks.
fn arb_hook_component() -> impl Strategy<Value = String> {
    (
        arb_component_name(),
        prop::collection::vec(
            prop::sample::select(vec![
                "useState(0)",
                "useState(null)",
                "useState(\"\")",
                "useEffect(() => {})",
                "useEffect(() => {}, [])",
                "useMemo(() => 42, [])",
                "useCallback(() => {}, [])",
                "useRef(null)",
                "useContext(Ctx)",
                "useReducer((s, a) => s, 0)",
            ]),
            1..5,
        ),
    )
        .prop_map(|(name, hooks)| {
            let mut hook_lines = String::new();
            for (i, h) in hooks.iter().enumerate() {
                let _ = writeln!(hook_lines, "  const v{i} = {h};");
            }
            format!("function {name}() {{\n{hook_lines}  return <div />;\n}}")
        })
}

/// Generates nested JSX structures of varying depth.
fn arb_nested_jsx() -> impl Strategy<Value = String> {
    (arb_component_name(), 1..6u32).prop_map(|(name, depth)| {
        let mut open = String::new();
        let mut close = String::new();
        for i in 0..depth {
            let tag = if i % 2 == 0 { "div" } else { "span" };
            let _ = write!(open, "<{tag}>");
            // Build closing tags in reverse order.
            close.insert_str(0, &format!("</{tag}>"));
        }
        format!("function {name}() {{ return ({open}hello{close}); }}")
    })
}

/// Generates edge-case components: empty bodies, many params, arrow functions.
fn arb_edge_case_component() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty function body
        arb_component_name().prop_map(|n| format!("function {n}() {{}}")),
        // Arrow function component
        arb_component_name().prop_map(|n| format!("const {n} = () => <div />;")),
        // Component with many parameters
        (arb_component_name(), prop::collection::vec(arb_ident(), 5..15)).prop_map(
            |(name, params)| {
                let params_str = params.join(", ");
                format!("function {name}({params_str}) {{ return <span />; }}")
            }
        ),
        // Component returning a fragment
        arb_component_name()
            .prop_map(|n| format!("function {n}() {{ return <><div /><span /></>; }}")),
        // Component with conditional return
        arb_component_name().prop_map(|n| format!(
            "function {n}({{ show }}) {{ if (show) return <div />; return null; }}"
        )),
    ]
}

// ---------------------------------------------------------------------------
// Config: keep test counts small so CI stays fast
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Simple component declarations must never cause a panic.
    #[test]
    fn fuzz_simple_component(source in arb_simple_component()) {
        let options = PluginOptions::default();
        let result = compile_program(&source, "fuzz.tsx", &options);
        // The compiler should always produce *some* output string.
        prop_assert!(!result.code.is_empty(), "compile_program returned empty code for: {source}");
    }

    /// Components containing hook calls must not panic.
    #[test]
    fn fuzz_hook_component(source in arb_hook_component()) {
        let options = PluginOptions::default();
        let result = compile_program(&source, "fuzz.tsx", &options);
        prop_assert!(!result.code.is_empty(), "compile_program returned empty code for: {source}");
    }

    /// Deeply nested JSX must not panic or stack-overflow.
    #[test]
    fn fuzz_nested_jsx(source in arb_nested_jsx()) {
        let options = PluginOptions::default();
        let result = compile_program(&source, "fuzz.tsx", &options);
        prop_assert!(!result.code.is_empty(), "compile_program returned empty code for: {source}");
    }

    /// Various edge-case component shapes must not panic.
    #[test]
    fn fuzz_edge_cases(source in arb_edge_case_component()) {
        let options = PluginOptions::default();
        let result = compile_program(&source, "fuzz.tsx", &options);
        prop_assert!(!result.code.is_empty(), "compile_program returned empty code for: {source}");
    }
}
