# Upstream Conformance Test Suite

> Validate behavioral equivalence with the upstream babel-plugin-react-compiler
> by running the same inputs through both compilers and comparing outputs.

---

### Gap 1: Port Upstream Fixture Inputs

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler/`
**Current state:** We have 10 hand-written fixture tests in `crates/oxc_react_compiler/tests/fixtures/`. The upstream compiler has ~2000+ fixture inputs covering edge cases, hooks, control flow, mutable ranges, etc.
**What's needed:**
- Download/copy the upstream fixture input files from `facebook/react` into a `tests/upstream-fixtures/` directory
- Organize them by category matching the upstream directory structure (compiler/, validation/, etc.)
- Add a script or Cargo test that iterates over all upstream fixtures
- Each fixture should be parseable and runnable through our pipeline without panicking (even if output differs)
**Depends on:** None

### Gap 2: Run Upstream Babel Plugin as Reference Oracle

**Upstream:** `compiler/packages/babel-plugin-react-compiler/`
**Current state:** No mechanism to run the upstream Babel plugin for comparison.
**What's needed:**
- Add a Node.js script (e.g., `tests/conformance/run-upstream.mjs`) that:
  - Installs `babel-plugin-react-compiler` and `@babel/core` as dev dependencies
  - Takes a fixture input file, runs it through the upstream Babel plugin
  - Writes the output to a `.expected` file alongside the input
- Generate `.expected` outputs for all upstream fixtures
- Store expected outputs in git so CI does not require running the Babel plugin every time
**Depends on:** Gap 1 (fixture inputs must exist)

### Gap 3: Differential Comparison Harness

**Upstream:** N/A (new infrastructure)
**Current state:** No automated comparison between our output and upstream output.
**What's needed:**
- A Rust integration test (e.g., `tests/conformance_tests.rs`) that for each upstream fixture:
  - Runs the input through `compile_program`
  - Compares our output against the `.expected` file
  - Records pass/fail status
- A summary reporter that prints pass/fail counts and lists divergences
- Support for a `known-failures.txt` file listing fixtures that are expected to diverge (with reasons)
- The test suite should pass CI even when known failures exist, but fail on regressions (a previously passing fixture now failing)
**Depends on:** Gap 1, Gap 2

### Gap 4: Behavioral Equivalence Normalization

**Upstream:** N/A
**Current state:** Direct string comparison will produce many false positives due to whitespace, variable naming, and import path differences.
**What's needed:**
- A normalization step before comparison that:
  - Strips whitespace/formatting differences (or re-formats both outputs with the same formatter)
  - Normalizes variable names (e.g., `$[0]` vs `_c[0]` cache slot naming)
  - Ignores import statement differences (our runtime import path may differ)
  - Optionally compares AST structure instead of text for deeper equivalence
- Document what kinds of divergences are acceptable vs problematic
**Depends on:** Gap 3

