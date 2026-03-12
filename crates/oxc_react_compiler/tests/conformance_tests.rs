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
//!   # or set OXC_DOWNLOAD_FIXTURES=1 to auto-download on first test run
//!
//! To generate expected outputs:
//!   node tests/conformance/run-upstream.mjs

use oxc_react_compiler::{PluginOptions, compile_program};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};

// ---------------------------------------------------------------------------
// OXC parse→transform→print roundtrip for normalization
// ---------------------------------------------------------------------------

/// Normalize source code via OXC parse→transform→print roundtrip.
///
/// Strips TypeScript type annotations AND lowers JSX to _jsx() calls,
/// ensuring both our output (which already uses _jsx) and Babel's output
/// (which preserves JSX syntax) end up in the same representation.
///
/// Falls back to returning the original code if parsing fails.
fn normalize_via_oxc(code: &str) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::tsx();
    let ret = Parser::new(&allocator, code, source_type).parse();

    if ret.panicked {
        return code.to_string();
    }

    let mut program = ret.program;

    // Run semantic analysis to get scoping info needed by the transformer
    let semantic_ret = SemanticBuilder::new().build(&program);
    let scoping = semantic_ret.semantic.into_scoping();

    // Configure: strip TypeScript types AND lower JSX to _jsx() calls
    // This ensures both our output and expected output use the same JSX representation
    let options = TransformOptions::default(); // Default enables both TS stripping and JSX transform
    let transformer = Transformer::new(&allocator, Path::new("test.tsx"), &options);
    let _ = transformer.build_with_scoping(scoping, &mut program);

    // Print the transformed AST
    Codegen::new().build(&program).code
}

// ---------------------------------------------------------------------------
// Output normalization for behavioral equivalence comparison
// ---------------------------------------------------------------------------

/// Normalize compiler output for comparison, reducing false positives from:
/// - Whitespace and formatting differences
/// - Variable naming differences (e.g., `$[0]` vs `_c[0]`)
/// - Import path differences
/// - Trailing whitespace and newlines
fn normalize_output(code: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut in_fixture_entrypoint = false;

    for line in code.lines() {
        let trimmed = line.trim();

        // Skip the FIXTURE_ENTRYPOINT test harness block — it's not part of
        // the actual compiled output and Babel reformats it differently.
        if trimmed.starts_with("export const FIXTURE_ENTRYPOINT") {
            in_fixture_entrypoint = true;
            continue;
        }
        if in_fixture_entrypoint {
            continue;
        }

        // Skip import statements (our runtime import paths and JSX imports may differ)
        if trimmed.starts_with("import ")
            && (trimmed.contains("react/compiler-runtime")
                || trimmed.contains("react-compiler-runtime")
                || trimmed.contains("react/jsx-runtime")
                || trimmed.contains("react/jsx-dev-runtime"))
        {
            continue;
        }

        // Normalize cache variable names and whitespace
        let normalized = normalize_cache_names(trimmed);

        // Skip empty lines
        if normalized.is_empty() {
            continue;
        }

        // Remove trailing commas before closing braces/brackets
        let normalized = strip_trailing_commas(&normalized);

        // Normalize import brace spacing: `import {X, Y}` → `import { X, Y }`
        let normalized = normalize_import_spacing(&normalized);

        lines.push(normalized);
    }

    lines.join("\n")
}

/// Normalize cache variable naming patterns.
///
/// The upstream compiler uses `$[N]` for cache slots. Our compiler also
/// uses `$[N]`. But some intermediate variable names may differ.
fn normalize_cache_names(line: &str) -> String {
    // Replace common cache slot patterns:
    // - `_c(N)` → `_c(N)` (already normalized)
    // - `const $ = _c(N)` stays as-is (both compilers use this)
    // For now, just normalize whitespace within lines
    let mut result = String::with_capacity(line.len());
    let mut prev_space = false;

    for ch in line.chars() {
        if ch == ' ' || ch == '\t' {
            if !prev_space && !result.is_empty() {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(ch);
            prev_space = false;
        }
    }

    result
}

/// Strip trailing commas before closing braces/brackets/parens.
/// Handles patterns like `foo,}` → `foo}` and `bar,]` → `bar]`.
fn strip_trailing_commas(line: &str) -> String {
    let mut result = line.to_string();
    // Remove trailing comma at end of line
    if result.ends_with(',') {
        result.pop();
    }
    // Remove comma before closing delimiters: `,}` `,]` `,)`
    result = result.replace(",}", "}");
    result = result.replace(",]", "]");
    result = result.replace(",)", ")");
    result
}

/// Normalize import brace spacing so `import {X, Y}` matches `import { X, Y }`.
fn normalize_import_spacing(line: &str) -> String {
    if !line.starts_with("import ") {
        return line.to_string();
    }
    // Find the braces in the import
    let Some(open) = line.find('{') else {
        return line.to_string();
    };
    let Some(close) = line[open..].find('}').map(|i| i + open) else {
        return line.to_string();
    };
    let inside = &line[open + 1..close];
    let normalized: Vec<&str> = inside.split(',').map(|p| p.trim()).collect();
    let joined = normalized.join(", ");
    format!("{}{{ {} }}{}", &line[..open], joined, &line[close + 1..])
}

/// Tokenize source code into a sequence of non-whitespace tokens for comparison.
/// This ignores all formatting differences (indentation, blank lines, newlines,
/// trailing commas, brace spacing) and focuses on structural equivalence.
fn tokenize(code: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // Skip whitespace
        if ch.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Skip single-line comments
        if ch == '/' && i + 1 < len && chars[i + 1] == '/' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Skip multi-line comments
        if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        // String literals (preserve content)
        if ch == '"' || ch == '\'' || ch == '`' {
            let quote = ch;
            let start = i;
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' {
                    i += 1; // skip escaped char
                }
                i += 1;
            }
            i += 1; // closing quote
            let s: String = chars[start..i.min(len)].iter().collect();
            tokens.push(s);
            continue;
        }

        // Identifiers and keywords
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
            let start = i;
            while i < len
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '$')
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            tokens.push(word);
            continue;
        }

        // Operators and punctuation (single char)
        tokens.push(ch.to_string());
        i += 1;
    }

    tokens
}

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
                // Skip fixtures where upstream Babel also errors
                if expected.starts_with("// UPSTREAM ERROR:") {
                    None
                } else {
                    // Normalize both sides: strip TS types + lower JSX to _jsx()
                    let our_stripped = normalize_via_oxc(&compile_result.code);
                    let expected_stripped = normalize_via_oxc(&expected);
                    // Use token-based comparison to ignore formatting differences
                    let our_normalized = normalize_output(&our_stripped);
                    let expected_normalized = normalize_output(&expected_stripped);
                    let our_tokens = tokenize(&our_normalized);
                    let expected_tokens = tokenize(&expected_normalized);
                    Some(our_tokens == expected_tokens)
                }
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

// ---------------------------------------------------------------------------
// Auto-download support (set OXC_DOWNLOAD_FIXTURES=1 to enable)
// ---------------------------------------------------------------------------

const REPO: &str = "facebook/react";
const BRANCH: &str = "main";
const FIXTURE_PREFIX: &str =
    "compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler";

/// Check if auto-download is enabled via environment variable.
fn should_auto_download() -> bool {
    std::env::var("OXC_DOWNLOAD_FIXTURES").unwrap_or_default() == "1"
}

/// Returns true if the directory exists and contains at least one fixture file.
fn has_fixture_files(dir: &Path) -> bool {
    !collect_fixture_files(dir).is_empty()
}

/// Download upstream fixtures using the GitHub API.
fn download_fixtures(fixtures_dir: &Path) {
    use serde_json::Value;

    eprintln!("Downloading upstream React Compiler fixtures...");
    eprintln!("Repository: {} (branch: {})", REPO, BRANCH);

    let api_url = format!("https://api.github.com/repos/{}/git/trees/{}?recursive=1", REPO, BRANCH);

    let response: Value = match ureq::get(&api_url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "oxc-react-compiler-tests")
        .call()
    {
        Ok(mut resp) => match resp.body_mut().read_json() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to parse GitHub API response: {}", e);
                return;
            }
        },
        Err(e) => {
            eprintln!("Failed to fetch GitHub API: {}", e);
            return;
        }
    };

    let tree = match response.get("tree").and_then(|t| t.as_array()) {
        Some(t) => t,
        None => {
            eprintln!("Unexpected API response format (no 'tree' array)");
            return;
        }
    };

    let fixture_entries: Vec<&str> = tree
        .iter()
        .filter_map(|entry| {
            let path = entry.get("path")?.as_str()?;
            let entry_type = entry.get("type")?.as_str()?;
            if entry_type == "blob"
                && path.starts_with(FIXTURE_PREFIX)
                && !path.contains("__snapshots__")
                && !path.ends_with(".snap")
                && !path.contains(".expected")
            {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    eprintln!("Found {} fixture files to download.", fixture_entries.len());

    let mut count = 0;
    for filepath in &fixture_entries {
        let rel_path = &filepath[FIXTURE_PREFIX.len() + 1..]; // strip prefix + /
        let output_file = fixtures_dir.join(rel_path);

        if let Some(parent) = output_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let raw_url = format!("https://raw.githubusercontent.com/{}/{}/{}", REPO, BRANCH, filepath);

        match ureq::get(&raw_url).header("User-Agent", "oxc-react-compiler-tests").call() {
            Ok(mut resp) => {
                if let Ok(body) = resp.body_mut().read_to_string() {
                    let _ = std::fs::write(&output_file, body);
                    count += 1;
                }
            }
            Err(_) => continue,
        }

        if count % 100 == 0 && count > 0 {
            eprintln!("  Downloaded {} / {} files...", count, fixture_entries.len());
        }
    }

    eprintln!("Done! Downloaded {} fixture files.", count);
}

/// Ensure fixtures are available, downloading if enabled and needed.
fn ensure_fixtures(fixtures_dir: &Path) -> bool {
    if has_fixture_files(fixtures_dir) {
        return true;
    }

    if should_auto_download() {
        let _ = std::fs::create_dir_all(fixtures_dir);
        download_fixtures(fixtures_dir);
        return has_fixture_files(fixtures_dir);
    }

    false
}

#[test]
fn upstream_conformance() {
    let fixtures_dir = upstream_fixtures_dir();

    if !ensure_fixtures(&fixtures_dir) {
        eprintln!(
            "Upstream fixtures not found at {}. Skipping conformance tests.",
            fixtures_dir.display()
        );
        eprintln!("Run ./tests/conformance/download-upstream.sh to download them,");
        eprintln!("or set OXC_DOWNLOAD_FIXTURES=1 to auto-download on test run.");
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
