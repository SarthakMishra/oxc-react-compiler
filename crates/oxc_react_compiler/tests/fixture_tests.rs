//! Fixture-based test harness for the React Compiler.
//!
//! Reads `.tsx`/`.ts` files from `tests/fixtures/`, compiles each with
//! `compile_program`, and snapshots the output via insta. This allows
//! comparing our output against upstream behavior and tracking regressions.
//!
//! To add a new fixture:
//!   1. Add a `.tsx` or `.ts` file in `tests/fixtures/`
//!   2. Run `cargo insta test --accept` to generate the initial snapshot
//!   3. Review the snapshot to verify correctness

use oxc_react_compiler::{PluginOptions, compile_program};
use std::path::Path;

/// Run a single fixture file through the compiler and snapshot its output.
fn run_fixture(fixture_path: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(fixture_path);

    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {fixture_path}: {e}"));

    let result = compile_program(&source, fixture_path, &PluginOptions::default());

    // Build a deterministic snapshot name from the fixture filename.
    let snap_name = fixture_path.replace('/', "__").replace('.', "_");

    // Snapshot both the transformed flag and the output code.
    let snapshot = format!(
        "transformed: {}\ndiagnostics: {}\n---\n{}",
        result.transformed,
        result.diagnostics.len(),
        if result.transformed { &result.code } else { "(not transformed)" }
    );

    insta::assert_snapshot!(snap_name, snapshot);
}

// ---------------------------------------------------------------------------
// Fixture tests: basic components
// ---------------------------------------------------------------------------

#[test]
fn fixture_basic_component() {
    run_fixture("basic-component.tsx");
}

#[test]
fn fixture_hook_with_state() {
    run_fixture("hook-with-state.tsx");
}

#[test]
fn fixture_component_with_conditional() {
    run_fixture("component-with-conditional.tsx");
}

#[test]
fn fixture_component_with_derived() {
    run_fixture("component-with-derived.tsx");
}

#[test]
fn fixture_exported_default() {
    run_fixture("exported-default.tsx");
}

#[test]
fn fixture_arrow_component() {
    run_fixture("arrow-component.tsx");
}

#[test]
fn fixture_multiple_components() {
    run_fixture("multiple-components.tsx");
}

#[test]
fn fixture_component_with_children() {
    run_fixture("component-with-children.tsx");
}

// ---------------------------------------------------------------------------
// Fixture tests: opt-outs and edge cases
// ---------------------------------------------------------------------------

#[test]
fn fixture_use_no_memo() {
    run_fixture("use-no-memo.tsx");
}

#[test]
fn fixture_non_component() {
    run_fixture("non-component.ts");
}

// ---------------------------------------------------------------------------
// Fixture directory scan test (ensures all fixtures are covered)
// ---------------------------------------------------------------------------

#[test]
fn all_fixtures_have_tests() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut fixture_files: Vec<String> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&fixtures_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension()
                && (ext == "tsx" || ext == "ts") {
                    fixture_files.push(path.file_name().unwrap().to_string_lossy().into_owned());
                }
        }
    }

    fixture_files.sort();

    // This test ensures we don't forget to add test functions for new fixtures.
    // If this fails, add a new #[test] fn fixture_xxx() above.
    let known_fixtures = vec![
        "arrow-component.tsx",
        "basic-component.tsx",
        "component-with-children.tsx",
        "component-with-conditional.tsx",
        "component-with-derived.tsx",
        "exported-default.tsx",
        "hook-with-state.tsx",
        "multiple-components.tsx",
        "non-component.ts",
        "use-no-memo.tsx",
    ];

    assert_eq!(
        fixture_files, known_fixtures,
        "Fixture files on disk don't match known fixtures. \
         Add new test functions for any new fixtures."
    );
}
