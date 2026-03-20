# OXLint Fork: Lint Rule Integration for Testing

Integrate `oxc_react_compiler_lint` rules into a fork of the upstream `oxc-project/oxc` repo, building an oxlint binary with our React Compiler lint rules included. Distributed via yalc for easy local testing in real repos.

**Motivation:** Our lint rules currently live in `crates/oxc_react_compiler_lint` and are only callable via NAPI bindings. Integrating into the oxlint binary gives us a standard CLI interface, better IDE integration (via oxlint editor plugins), and easier adoption for testing in real projects.

**Maintenance cost:** The fork must be periodically rebased on upstream oxc. This is acceptable for a testing/validation phase but should not be a long-term strategy.

---

## Gap 1: Fork Setup and Workspace Integration

**Current state:** No fork exists. Our lint crate depends on `oxc_*` workspace crates from crates.io or git deps.

**What's needed:**
- Fork `oxc-project/oxc` to our GitHub org
- Add `oxc_react_compiler` and `oxc_react_compiler_lint` as workspace members in the fork
- Resolve dependency conflicts: our crates currently pull oxc crates from this project's workspace; in the fork, they must use the fork's workspace crates directly
- May require a thin adapter crate or Cargo path overrides to bridge the two workspaces
- Ensure `cargo check` passes for the full workspace including our crates

**Depends on:** None

---

## Gap 2: Rule Adapter Layer (declare_oxc_lint! Integration)

**Current state:** Our rules use a custom `check_*` function signature (`fn check_*(program: &Program) -> Vec<OxcDiagnostic>`). OXLint rules use the `declare_oxc_lint!` macro and implement the `Rule` trait with visitor-based callbacks (`run_once`, `run`, `run_on_symbol`, etc.).

**What's needed:**
- Write adapter implementations for each rule that wrap our `check_*` functions in the oxlint `Rule` trait
- Each adapter needs:
  - `declare_oxc_lint!` macro invocation with rule metadata (name, category, severity, docs)
  - `Rule::run_once` implementation that calls our `check_*` function and converts diagnostics
  - Proper rule category assignment (likely `correctness` or `restriction` under a `react-compiler` plugin prefix)
- Decide on rule naming convention (e.g., `react-compiler/rules-of-hooks`, `react-compiler/no-jsx-in-try`)
- 12 Tier 1 rules to wrap: `rules_of_hooks`, `no_jsx_in_try`, `no_ref_access_in_render`, `no_set_state_in_render`, `no_set_state_in_effects`, `use_memo_validation`, `no_capitalized_calls`, `purity`, `incompatible_library`, `static_components`, `no_deriving_state_in_effects`, `globals`

**Depends on:** Gap 1

---

## Gap 3: Rule Registry Registration

**Current state:** OXLint discovers rules through a central registry (the `oxc_linter` crate's rule table). Rules must be registered there to be available via CLI and config.

**What's needed:**
- Add our rules to the oxlint rule registry in the fork's `crates/oxc_linter` crate
- Register under a `react-compiler` plugin category (similar to how `eslint-plugin-react-hooks` rules are registered)
- Ensure rules appear in `oxlint --rules` output
- Ensure rules can be enabled/disabled via oxlint config (`.oxlintrc.json`)
- Verify rules produce correct diagnostic output format (spans, messages, help text)

**Depends on:** Gap 2

---

## Gap 4: Build and Verify the Binary

**Current state:** N/A -- no fork binary exists.

**What's needed:**
- Build the forked oxlint binary with our rules included
- Run oxlint against a small set of test files to verify:
  - Rules fire correctly on known-bad code
  - Rules do not fire on known-good code
  - Diagnostic output includes correct file positions and messages
- Run existing upstream oxlint tests to ensure no regressions from adding our crate
- Document the build process (cargo commands, feature flags if any)

**Depends on:** Gap 3

---

## Gap 5: Yalc Publishing Workflow

**Current state:** No distribution mechanism for the forked binary.

**What's needed:**
- Set up a `package.json` in the fork that wraps the oxlint binary for npm/yalc distribution
  - Note: upstream oxc already has npm packaging infrastructure (`npm/oxlint/`); leverage this
- Script to build the binary for the current platform and publish via `yalc publish`
- Document the install flow for testers: `yalc add @oxc/oxlint-react-compiler && npx oxlint ...`
- Consider whether to publish platform-specific binaries or just the local platform

**Depends on:** Gap 4

---

## Gap 6: CI Auto-Rebase and Conflict Detection

**Current state:** No automation for keeping the fork in sync.

**What's needed:**
- GitHub Actions workflow on the fork that:
  - Runs on a schedule (e.g., weekly) or on upstream release tags
  - Attempts to rebase our branch on upstream `main`
  - If rebase succeeds: pushes updated branch, runs full test suite
  - If rebase fails: opens an issue or sends a notification with the conflict details
- Alternative: use `git merge` instead of rebase if the commit history is not important
- Document the manual conflict resolution process for when auto-rebase fails

**Depends on:** Gap 1
