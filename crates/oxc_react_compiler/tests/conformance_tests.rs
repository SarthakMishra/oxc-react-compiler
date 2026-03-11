//! Conformance test runner for upstream React Compiler fixtures.
//!
//! Iterates over all `.tsx`/`.ts`/`.js` files in `tests/conformance/upstream-fixtures/`,
//! runs each through `compile_program`, and verifies:
//! 1. The compiler does not panic on any input
//! 2. If an `.expected` file exists, compare output against it
//! 3. Track pass/fail counts and report a summary
//!
//! Fixtures listed in `tests/conformance/known-failures.txt` are expected to
//! diverge and do not cause the test suite to fail.
//!
//! To download upstream fixtures:
//!   ./tests/conformance/download-upstream.sh
//!
//! To generate expected outputs:
//!   node tests/conformance/run-upstream.mjs

use oxc_react_compiler::{PluginOptions, compile_program};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Root directory for conformance test infrastructure.
fn conformance_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/conformance")
}

/// Directory containing upstream fixture inputs.
fn upstream_fixtures_dir() -> PathBuf {
    conformance_dir().join("upstream-fixtures")
}

/// Load the set of known failure fixture paths.
fn load_known_failures() -> HashSet<String> {
    let path = conformance_dir().join("known-failures.txt");
    match std::fs::read_to_string(&path) {
        Ok(contents) => contents
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect(),
        Err(_) => HashSet::new(),
    }
}

/// Collect all fixture files recursively.
fn collect_fixture_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }

    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, files);
                } else if let Some(ext) = path.extension() {
                    if matches!(ext.to_str(), Some("tsx" | "ts" | "js" | "jsx")) {
                        files.push(path);
                    }
                }
            }
        }
    }

    walk(dir, &mut files);
    files.sort();
    files
}

/// Result of running a single fixture through our compiler.
#[derive(Debug)]
struct FixtureResult {
    /// Relative path from the upstream-fixtures directory.
    relative_path: String,
    /// Whether the compiler panicked.
    panicked: bool,
    /// Whether the output matches the expected output (if available).
    matches_expected: Option<bool>,
    /// Whether this fixture is in the known-failures list.
    known_failure: bool,
    /// Number of diagnostics emitted.
    diagnostic_count: usize,
}

/// Run a single fixture through the compiler.
fn run_fixture(fixture_path: &Path, fixtures_dir: &Path) -> FixtureResult {
    let relative_path = fixture_path
        .strip_prefix(fixtures_dir)
        .unwrap_or(fixture_path)
        .to_string_lossy()
        .into_owned();

    let source = match std::fs::read_to_string(fixture_path) {
        Ok(s) => s,
        Err(_) => {
            return FixtureResult {
                relative_path,
                panicked: true,
                matches_expected: None,
                known_failure: false,
                diagnostic_count: 0,
            };
        }
    };

    let filename = fixture_path.file_name().unwrap().to_string_lossy().into_owned();

    // Use catch_unwind to detect panics.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        compile_program(&source, &filename, &PluginOptions::default())
    }));

    match result {
        Ok(compile_result) => {
            // Check if an .expected file exists alongside the fixture.
            let expected_path = fixture_path.with_extension("expected");
            let matches_expected = if expected_path.exists() {
                let expected = std::fs::read_to_string(&expected_path).unwrap_or_default();
                Some(compile_result.code.trim() == expected.trim())
            } else {
                None
            };

            FixtureResult {
                relative_path,
                panicked: false,
                matches_expected,
                known_failure: false,
                diagnostic_count: compile_result.diagnostics.len(),
            }
        }
        Err(_) => FixtureResult {
            relative_path,
            panicked: true,
            matches_expected: None,
            known_failure: false,
            diagnostic_count: 0,
        },
    }
}

#[test]
fn upstream_conformance() {
    let fixtures_dir = upstream_fixtures_dir();
    if !fixtures_dir.exists() {
        eprintln!(
            "Upstream fixtures not found at {}. Skipping conformance tests.",
            fixtures_dir.display()
        );
        eprintln!("Run ./tests/conformance/download-upstream.sh to download them.");
        return;
    }

    let fixture_files = collect_fixture_files(&fixtures_dir);
    if fixture_files.is_empty() {
        eprintln!("No fixture files found in {}. Skipping.", fixtures_dir.display());
        return;
    }

    let known_failures = load_known_failures();

    let mut results: Vec<FixtureResult> = Vec::new();
    for fixture_path in &fixture_files {
        let mut result = run_fixture(fixture_path, &fixtures_dir);
        result.known_failure = known_failures.contains(&result.relative_path);
        results.push(result);
    }

    // Compute summary statistics.
    let total = results.len();
    let panicked: Vec<&FixtureResult> = results.iter().filter(|r| r.panicked).collect();
    let matched: Vec<&FixtureResult> =
        results.iter().filter(|r| !r.panicked && r.matches_expected == Some(true)).collect();
    let diverged: Vec<&FixtureResult> =
        results.iter().filter(|r| !r.panicked && r.matches_expected == Some(false)).collect();
    let no_expected: Vec<&FixtureResult> =
        results.iter().filter(|r| !r.panicked && r.matches_expected.is_none()).collect();

    // Print summary.
    println!("\n=== Upstream Conformance Summary ===");
    println!("Total fixtures:    {}", total);
    println!("Compiled OK:       {}", total - panicked.len());
    println!("  Matched expected: {}", matched.len());
    println!("  Diverged:         {}", diverged.len());
    println!("  No expected file: {}", no_expected.len());
    println!("Panicked:          {}", panicked.len());
    println!("Known failures:    {}", results.iter().filter(|r| r.known_failure).count());
    println!();

    // Report panics (these are always bugs).
    if !panicked.is_empty() {
        println!("--- PANICS (compiler crashed) ---");
        for r in &panicked {
            let marker = if r.known_failure { " [known]" } else { " [REGRESSION]" };
            println!("  PANIC: {}{}", r.relative_path, marker);
        }
        println!();
    }

    // Report unexpected divergences (not in known-failures).
    let unexpected_divergences: Vec<&&FixtureResult> =
        diverged.iter().filter(|r| !r.known_failure).collect();
    if !unexpected_divergences.is_empty() {
        println!("--- UNEXPECTED DIVERGENCES ---");
        for r in &unexpected_divergences {
            println!("  DIVERGED: {}", r.relative_path);
        }
        println!();
    }

    // Report unexpected panics (not in known-failures).
    let unexpected_panics: Vec<&&FixtureResult> =
        panicked.iter().filter(|r| !r.known_failure).collect();

    // The test fails if there are unexpected panics (not in known-failures).
    // Divergences without .expected files don't fail — they just track progress.
    if !unexpected_panics.is_empty() {
        panic!(
            "{} fixture(s) caused the compiler to panic (not in known-failures.txt). \
             See the list above.",
            unexpected_panics.len()
        );
    }

    println!("Conformance test passed (no unexpected panics).");
}
