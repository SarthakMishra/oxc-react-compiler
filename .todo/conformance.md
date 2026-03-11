# Upstream Conformance Test Suite

> Validate behavioral equivalence with the upstream babel-plugin-react-compiler
> by running the same inputs through both compilers and comparing outputs.

---

### Gap 1: Port Upstream Fixture Inputs ✅

~~**Previous:** Download upstream fixture inputs from facebook/react into test suite.~~

**Completed**: Added `download-upstream.sh` script that fetches fixture inputs from the upstream babel-plugin-react-compiler test suite into `upstream-fixtures/` directory.

### Gap 2: Run Upstream Babel Plugin as Reference Oracle ✅

~~**Previous:** Add a Node.js script to run upstream Babel plugin and generate expected outputs.~~

**Completed**: Added `run-upstream.mjs` script that runs fixture inputs through the upstream babel-plugin-react-compiler and generates `.expected` output files for comparison.

### Gap 3: Differential Comparison Harness ✅

~~**Previous:** Build a Rust integration test that compares our output against upstream `.expected` files.~~

**Completed**: Added `conformance_tests.rs` integration test that runs each upstream fixture through `compile_program`, compares against `.expected` output, and supports a `known-failures.txt` file for expected divergences.

### Gap 4: Behavioral Equivalence Normalization ✅

~~**Previous:** Add output normalization (whitespace, imports, cache variable names) to the conformance test runner before comparison.~~

**Completed**: Added normalization to the conformance test runner that strips whitespace/formatting differences, normalizes cache variable names, and handles import path differences to reduce false positive divergences.

