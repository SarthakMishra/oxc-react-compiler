//! Per-pass snapshot tests for the oxc-react-compiler pipeline.
//!
//! These tests compile small, focused React component fixtures and snapshot
//! the final codegen output. Each fixture is designed to exercise a specific
//! compiler pass (scope inference, conditional scoping, dependency tracking,
//! destructure handling, JSX codegen, scope merging). Because the pipeline
//! is monolithic, we snapshot the end-to-end output — any regression in an
//! individual pass will cause the corresponding snapshot to change.

use oxc_react_compiler::{PluginOptions, compile_program};

/// Helper: compile a TSX source and return the codegen output.
/// Panics if the component was not transformed.
fn compile_fixture(source: &str) -> String {
    let result = compile_program(source, "fixture.tsx", &PluginOptions::default());
    assert!(
        result.transformed,
        "expected the fixture to be transformed but it was not.\nSource:\n{source}"
    );
    assert!(!result.code.is_empty(), "transformed code should not be empty");
    result.code
}

// ---------------------------------------------------------------------------
// Snapshot: basic memoization (scope inference + codegen)
// ---------------------------------------------------------------------------

/// A simple component that derives a value from props.
/// Exercises: scope inference, basic memoization codegen.
#[test]
fn snapshot_basic_memoization() {
    let code = compile_fixture(
        r"
function Doubler({ value }) {
    const doubled = value * 2;
    return <span>{doubled}</span>;
}
",
    );
    insta::assert_snapshot!("basic_memoization", code);
}

// ---------------------------------------------------------------------------
// Snapshot: conditional rendering (conditional scoping)
// ---------------------------------------------------------------------------

/// A component that conditionally renders different JSX branches.
/// Exercises: conditional scoping, control-flow analysis.
#[test]
fn snapshot_conditional_rendering() {
    let code = compile_fixture(
        r#"
function Status({ isActive }) {
    if (isActive) {
        return <div className="active">Active</div>;
    }
    return <div className="inactive">Inactive</div>;
}
"#,
    );
    insta::assert_snapshot!("conditional_rendering", code);
}

// ---------------------------------------------------------------------------
// Snapshot: hook dependencies (dependency tracking)
// ---------------------------------------------------------------------------

/// A component with useState and a derived value that depends on the state.
/// Exercises: hook call handling, dependency tracking between state and
/// derived values.
#[test]
fn snapshot_hook_dependencies() {
    let code = compile_fixture(
        r"
function Counter() {
    const [count, setCount] = useState(0);
    const next = count + 1;
    return <button onClick={() => setCount(next)}>{count}</button>;
}
",
    );
    insta::assert_snapshot!("hook_dependencies", code);
}

// ---------------------------------------------------------------------------
// Snapshot: destructured props (destructure handling)
// ---------------------------------------------------------------------------

/// A component that destructures multiple props with a default value.
/// Exercises: destructured parameter handling, default value propagation.
#[test]
fn snapshot_destructured_props() {
    let code = compile_fixture(
        r#"
function UserCard({ name, age, role = "member" }) {
    return (
        <div>
            <h2>{name}</h2>
            <p>Age: {age}</p>
            <p>Role: {role}</p>
        </div>
    );
}
"#,
    );
    insta::assert_snapshot!("destructured_props", code);
}

// ---------------------------------------------------------------------------
// Snapshot: JSX children (JSX codegen)
// ---------------------------------------------------------------------------

/// A component that accepts and renders children alongside its own elements.
/// Exercises: JSX children codegen, slot-like patterns.
#[test]
fn snapshot_jsx_children() {
    let code = compile_fixture(
        r#"
function Card({ title, children }) {
    return (
        <div className="card">
            <h3>{title}</h3>
            <div className="body">{children}</div>
        </div>
    );
}
"#,
    );
    insta::assert_snapshot!("jsx_children", code);
}

// ---------------------------------------------------------------------------
// Snapshot: multiple scopes (scope merging)
// ---------------------------------------------------------------------------

/// A component with multiple independent derived values, each forming its
/// own reactive scope.
/// Exercises: scope merging / splitting, multi-scope memoization codegen.
#[test]
fn snapshot_multiple_scopes() {
    let code = compile_fixture(
        r#"
function Summary({ items, threshold }) {
    const count = items.length;
    const hasMany = count > threshold;
    const label = hasMany ? "Many items" : "Few items";
    return (
        <div>
            <span>{label}</span>
            <span>({count})</span>
        </div>
    );
}
"#,
    );
    insta::assert_snapshot!("multiple_scopes", code);
}
