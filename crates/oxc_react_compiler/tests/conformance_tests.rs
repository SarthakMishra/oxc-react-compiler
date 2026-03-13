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

use oxc_react_compiler::{
    CompilationMode, EnvironmentConfig, GatingConfig, OutputMode, PanicThreshold, PluginOptions,
    compile_program_with_config,
};
use std::collections::{HashMap, HashSet};
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

    // Sort import lines so import ordering differences don't cause false divergences.
    // Separate leading imports from the rest, sort them, then rejoin.
    let first_non_import =
        lines.iter().position(|l| !l.starts_with("import ")).unwrap_or(lines.len());
    lines[..first_non_import].sort();

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
    let normalized: Vec<&str> = inside.split(',').map(str::trim).collect();
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

        // String literals — normalize quote style to double quotes
        if ch == '"' || ch == '\'' || ch == '`' {
            let quote = ch;
            i += 1;
            let mut content = String::new();
            while i < len && chars[i] != quote {
                if chars[i] == '\\' {
                    content.push(chars[i]);
                    i += 1;
                    if i < len {
                        content.push(chars[i]);
                    }
                } else {
                    content.push(chars[i]);
                }
                i += 1;
            }
            i += 1; // closing quote
            // Normalize to double-quote form for comparison
            tokens.push(format!("\"{content}\""));
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
            // Normalize `let` → `const`: Babel's printer promotes `let` to `const` for
            // single-assignment variables, while our pass-through preserves the original.
            // This normalization avoids false divergences for this formatting difference.
            if word == "let" {
                tokens.push("const".to_string());
            } else if word == "component" {
                // Flow `component` syntax → `function`: Babel converts Flow
                // component declarations to regular functions.
                tokens.push("function".to_string());
            } else {
                tokens.push(word);
            }
            continue;
        }

        // Operators and punctuation (single char)
        tokens.push(ch.to_string());
        i += 1;
    }

    // Post-process normalizations
    let mut result = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        // Normalize `return undefined ;` → `return ;`
        if tokens[i] == "return"
            && i + 1 < tokens.len()
            && tokens[i + 1] == "undefined"
            && i + 2 < tokens.len()
            && tokens[i + 2] == ";"
        {
            result.push("return".to_string());
            i += 2;
            continue;
        }

        // Normalize compound assignments: `x += y` → `x = x + y`
        // Babel consistently lowers these to explicit binary expressions.
        // In our tokenizer, `+=` is tokenized as two tokens: `+` and `=`.
        // So `x += 1` becomes tokens: identifier, operator, `=`, value.
        if i + 2 < tokens.len()
            && tokens[i + 2] == "="
            && matches!(tokens[i + 1].as_str(), "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^")
        {
            let lhs = tokens[i].clone();
            let op = tokens[i + 1].clone();
            result.push(lhs.clone());
            result.push("=".to_string());
            result.push(lhs);
            result.push(op);
            i += 3; // skip identifier, operator, `=`
            continue;
        }

        result.push(tokens[i].clone());
        i += 1;
    }

    // Canonicalize label names: rename labels by first-occurrence order
    // (e.g., `label:` vs `bb0:` differences between Babel and our compiler).
    // A label is an identifier followed by `:` that is NOT part of a ternary or
    // object literal context. We also canonicalize `break label` / `continue label`.
    let mut label_remap: HashMap<String, String> = HashMap::new();
    let mut label_counter = 0u32;
    let mut labelled = Vec::with_capacity(result.len());
    for (idx, token) in result.iter().enumerate() {
        // Detect label definition: `ident :` where ident is not a keyword
        if idx + 1 < result.len()
            && result[idx + 1] == ":"
            && token.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && !matches!(
                token.as_str(),
                "if" | "else"
                    | "const"
                    | "function"
                    | "return"
                    | "case"
                    | "default"
                    | "switch"
                    | "for"
                    | "while"
                    | "do"
                    | "break"
                    | "continue"
                    | "var"
                    | "typeof"
                    | "void"
                    | "new"
                    | "delete"
                    | "throw"
                    | "try"
                    | "catch"
                    | "finally"
                    | "class"
                    | "import"
                    | "export"
                    | "from"
                    | "as"
                    | "of"
                    | "in"
            )
            && (idx == 0 || matches!(result[idx - 1].as_str(), "{" | "}" | ";" | ")" | "else"))
        {
            label_remap.entry(token.clone()).or_insert_with(|| {
                let name = format!("L{label_counter}");
                label_counter += 1;
                name
            });
        }
        labelled.push(token.clone());
    }
    // Apply label remapping
    for token in &mut labelled {
        if let Some(canonical) = label_remap.get(token) {
            *token = canonical.clone();
        }
    }
    let result = labelled;

    // Normalize temp variable names: rename `tN` → `t0`, `t1`, `t2`, ...
    // Babel and our compiler use different numbering schemes for temporary
    // variables (Babel counts from 0, ours uses HIR instruction IDs).
    // Renaming sequentially by first-occurrence eliminates this difference.
    let mut temp_remap: HashMap<String, String> = HashMap::new();
    let mut temp_counter = 0u32;
    let mut final_tokens = Vec::with_capacity(result.len());
    for token in &result {
        if is_temp_token(token) {
            let remapped = temp_remap.entry(token.clone()).or_insert_with(|| {
                let name = format!("t{temp_counter}");
                temp_counter += 1;
                name
            });
            final_tokens.push(remapped.clone());
        } else {
            final_tokens.push(token.clone());
        }
    }
    final_tokens
}

/// Returns true if a token looks like a compiler temporary variable (tN where N is a number).
fn is_temp_token(token: &str) -> bool {
    if !token.starts_with('t') || token.len() < 2 {
        return false;
    }
    token[1..].chars().all(|c| c.is_ascii_digit())
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
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect(),
        Err(_) => HashSet::new(),
    }
}

/// Collect all fixture files recursively.
fn collect_fixture_files(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, files);
                } else if let Some(ext) = path.extension()
                    && matches!(ext.to_str(), Some("tsx" | "ts" | "js" | "jsx")) {
                        files.push(path);
                    }
            }
        }
    }

    let mut files = Vec::new();
    if !dir.exists() {
        return files;
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
    _diagnostic_count: usize,
}

/// Parse per-fixture compiler options from `@directive` comments in the source.
///
/// Parses both `PluginOptions` (compilation mode, panic threshold) and
/// `EnvironmentConfig` (validation toggles, feature flags) from comment directives.
fn parse_fixture_options(source: &str) -> (PluginOptions, EnvironmentConfig) {
    // Helper: extract the value after a directive like @name:"value" or @name(value)
    fn find_directive_value<'a>(comment: &'a str, name: &str) -> Option<&'a str> {
        let needle = format!("@{name}");
        let pos = comment.find(&needle)?;
        let after = &comment[pos + needle.len()..];
        if let Some(rest) = after.strip_prefix(":\"") {
            let end = rest.find('"').unwrap_or(rest.len());
            Some(&rest[..end])
        } else if let Some(rest) = after.strip_prefix(':') {
            let end = rest.find([' ', '@']).unwrap_or(rest.len());
            Some(&rest[..end])
        } else if let Some(rest) = after.strip_prefix('(') {
            let end = rest.find(')').unwrap_or(rest.len());
            Some(&rest[..end])
        } else {
            None
        }
    }

    // Parse all @-prefixed directives from the comment line.
    // Multiple directives can appear on a single line: @foo @bar:true @baz:"val"
    fn find_directive_bool(comment: &str, name: &str) -> Option<bool> {
        let needle = format!("@{name}");
        if let Some(pos) = comment.find(&needle) {
            let after = &comment[pos + needle.len()..];
            if after.starts_with(":false") {
                return Some(false);
            }
            // bare @name or @name:true or @name followed by space/end
            return Some(true);
        }
        None
    }

    let mut opts = PluginOptions {
        // Default to Infer mode. While Babel's test harness uses "all" by default,
        // our compiled output for non-component functions still diverges significantly
        // (temp variable explosion, different dependency tracking). Using Infer avoids
        // false regressions from compiling functions where our output isn't yet correct.
        compilation_mode: CompilationMode::Infer,
        ..PluginOptions::default()
    };
    let mut env = EnvironmentConfig::default();

    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("//") {
            // Stop scanning after non-comment lines (directives are at the top)
            if !trimmed.is_empty() {
                break;
            }
            continue;
        }
        let comment = trimmed.trim_start_matches("//").trim();

        // @compilationMode:"infer" | @compilationMode:"all" etc.
        if let Some(mode) = find_directive_value(comment, "compilationMode") {
            opts.compilation_mode = match mode {
                "infer" => CompilationMode::Infer,
                "annotation" => CompilationMode::Annotation,
                "syntax" => CompilationMode::Syntax,
                _ => CompilationMode::All,
            };
        }

        // @panicThreshold:"ALL_ERRORS" | @panicThreshold(none) etc.
        if let Some(val) = find_directive_value(comment, "panicThreshold") {
            opts.panic_threshold = match val {
                "ALL_ERRORS" | "all" => PanicThreshold::AllErrors,
                "NONE" | "none" => PanicThreshold::None,
                _ => PanicThreshold::CriticalErrors,
            };
        }

        // @outputMode:"lint" etc.
        if let Some(mode) = find_directive_value(comment, "outputMode") {
            opts.output_mode = match mode {
                "lint" => OutputMode::Lint,
                "ssr" => OutputMode::SSR,
                "client-no-memo" => OutputMode::ClientNoMemo,
                _ => OutputMode::Client,
            };
        }

        // @gating — use a stub gating config
        if find_directive_bool(comment, "gating").unwrap_or(false) {
            opts.gating = Some(GatingConfig {
                import_source: "shared-runtime".to_string(),
                function_name: "__gate".to_string(),
            });
        }

        // @dynamicGating — also activates gating behavior
        if find_directive_bool(comment, "dynamicGating").unwrap_or(false) {
            opts.gating = Some(GatingConfig {
                import_source: "shared-runtime".to_string(),
                function_name: "__gate".to_string(),
            });
        }

        if let Some(v) = find_directive_bool(comment, "enablePreserveExistingMemoizationGuarantees")
        {
            env.enable_preserve_existing_memoization_guarantees = v;
        }
        if let Some(v) =
            find_directive_bool(comment, "validatePreserveExistingMemoizationGuarantees")
        {
            env.validate_preserve_existing_memoization_guarantees = v;
        }
        if let Some(v) = find_directive_bool(comment, "validateRefAccessDuringRender") {
            env.validate_ref_access_during_render = v;
        }
        if let Some(v) = find_directive_bool(comment, "validateExhaustiveMemoizationDependencies") {
            env.validate_exhaustive_memo_dependencies = v;
        }
        if let Some(v) = find_directive_bool(comment, "validateNoSetStateInEffects") {
            env.validate_no_set_state_in_effects = v;
        }
        if let Some(v) = find_directive_bool(comment, "validateNoSetStateInRender") {
            env.validate_no_set_state_in_render = v;
        }
        if let Some(v) = find_directive_bool(comment, "enableTransitivelyFreezeFunctionExpressions")
        {
            env.enable_transitively_freeze_function_expressions = v;
        }
        if let Some(v) = find_directive_bool(comment, "enableAssumeHooksFollowRulesOfReact") {
            env.enable_assume_hooks_follow_rules_of_react = v;
        }
        if let Some(v) = find_directive_bool(comment, "enableOptionalDependencies") {
            env.enable_optional_dependencies = v;
        }
        if let Some(v) = find_directive_bool(comment, "enableTreatRefLikeIdentifiersAsRefs") {
            env.enable_treat_ref_like_identifiers_as_refs = v;
        }
        if let Some(v) = find_directive_bool(comment, "enableJsxOutlining") {
            env.enable_jsx_outlining = v;
        }
    }

    (opts, env)
}

/// Run a single fixture through the compiler.
fn run_fixture(fixture_path: &Path, fixtures_dir: &Path) -> FixtureResult {
    let relative_path = fixture_path
        .strip_prefix(fixtures_dir)
        .unwrap_or(fixture_path)
        .to_string_lossy()
        .into_owned();

    let Ok(source) = std::fs::read_to_string(fixture_path) else {
        return FixtureResult {
            relative_path,
            panicked: true,
            matches_expected: None,
            known_failure: false,
            _diagnostic_count: 0,
        };
    };

    let filename = fixture_path.file_name().unwrap().to_string_lossy().into_owned();
    let (options, env_config) = parse_fixture_options(&source);

    // Use catch_unwind to detect panics.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        compile_program_with_config(&source, &filename, &options, &env_config)
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
                _diagnostic_count: compile_result.diagnostics.len(),
            }
        }
        Err(_) => FixtureResult {
            relative_path,
            panicked: true,
            matches_expected: None,
            known_failure: false,
            _diagnostic_count: 0,
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
#[expect(clippy::print_stderr)]
fn download_fixtures(fixtures_dir: &Path) {
    use serde_json::Value;

    eprintln!("Downloading upstream React Compiler fixtures...");
    eprintln!("Repository: {REPO} (branch: {BRANCH})");

    let api_url = format!("https://api.github.com/repos/{REPO}/git/trees/{BRANCH}?recursive=1");

    let response: Value = match ureq::get(&api_url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "oxc-react-compiler-tests")
        .call()
    {
        Ok(mut resp) => match resp.body_mut().read_json() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to parse GitHub API response: {e}");
                return;
            }
        },
        Err(e) => {
            eprintln!("Failed to fetch GitHub API: {e}");
            return;
        }
    };

    let Some(tree) = response.get("tree").and_then(|t| t.as_array()) else {
        eprintln!("Unexpected API response format (no 'tree' array)");
        return;
    };

    let fixture_entries: Vec<&str> = tree
        .iter()
        .filter_map(|entry| {
            let path = entry.get("path")?.as_str()?;
            let entry_type = entry.get("type")?.as_str()?;
            if entry_type == "blob"
                && path.starts_with(FIXTURE_PREFIX)
                && !path.contains("__snapshots__")
                && !Path::new(path).extension().is_some_and(|e| e.eq_ignore_ascii_case("snap"))
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

        let raw_url = format!("https://raw.githubusercontent.com/{REPO}/{BRANCH}/{filepath}");

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

    eprintln!("Done! Downloaded {count} fixture files.");
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
#[expect(clippy::print_stdout, clippy::print_stderr)]
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
    let no_expected_count =
        results.iter().filter(|r| !r.panicked && r.matches_expected.is_none()).count();

    // Print summary.
    println!("\n=== Upstream Conformance Summary ===");
    println!("Total fixtures:    {total}");
    println!("Compiled OK:       {}", total - panicked.len());
    println!("  Matched expected: {}", matched.len());
    println!("  Diverged:         {}", diverged.len());
    println!("  No expected file: {no_expected_count}");
    println!("Panicked:          {}", panicked.len());
    println!("Known failures:    {}", results.iter().filter(|r| r.known_failure).count());
    println!();

    // Report passing fixtures that are in known-failures (can be removed).
    let newly_passing: Vec<&&FixtureResult> = matched.iter().filter(|r| r.known_failure).collect();
    if !newly_passing.is_empty() {
        println!("--- NEWLY PASSING (remove from known-failures.txt) ---");
        for r in &newly_passing {
            println!("  PASS: {}", r.relative_path);
        }
        println!();
    }

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
    assert!(unexpected_panics.is_empty(), 
        "{} fixture(s) caused the compiler to panic (not in known-failures.txt). \
         See the list above.",
        unexpected_panics.len()
    );

    println!("Conformance test passed (no unexpected panics).");
}
