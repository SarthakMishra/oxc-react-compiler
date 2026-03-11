# Testing Strategy

> Test infrastructure for verifying upstream behavioral equivalence.
> See REQUIREMENTS.md Section 15.

---

### Gap 1: Upstream Fixture Test Harness

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/`
**Current state:** Nothing implemented.
**What's needed:**

- Test runner that loads upstream fixture files (input React source + expected output)
- Run input through the full pipeline
- Compare output with expected output
- Handle fixture format:
  - Input: `.js`/`.tsx` React component source
  - Expected: compiled output JavaScript
- Fixture categories: basic components, hooks, control flow, edge cases
- Initially: port a small subset of fixtures for end-to-end validation
- Goal: pass the full upstream fixture suite

Infrastructure:
- Conformance test crate or test module
- Fixture file discovery and loading
- Diff-friendly test failure output

**Depends on:** Pipeline orchestration, Codegen (must produce output to compare)

---

### Gap 2: Per-Pass Snapshot Tests

**Current state:** `insta` is in dev-dependencies but no tests exist.
**What's needed:**

- For each implemented pass, snapshot tests using `insta`:
  - Create small HIR fixtures programmatically
  - Run the pass
  - Snapshot the HIR state after the pass
- HIR pretty-printer for snapshot output:
  - Print blocks, instructions, terminals in a human-readable format
  - Include effect annotations, types, mutable ranges
- Snapshot naming convention: `pass_name__test_case_name`

**Depends on:** HIR types (must have Display/Debug implementations)

---

### Gap 3: Comparison Tests

**Current state:** Nothing implemented.
**What's needed:**

- Run the same input through both the Babel plugin and the OXC plugin
- Diff the outputs
- Track which fixtures produce identical output vs divergent
- Report divergence percentage
- Useful for incremental progress tracking

Infrastructure:
- Script to run Babel plugin on fixture inputs
- Script to run OXC plugin on same inputs
- Diffing and reporting tool

**Depends on:** Full pipeline working end-to-end

---

### Gap 4: Performance Benchmarking

**Current state:** Nothing implemented.
**What's needed:**

- Benchmark suite using `criterion` or similar
- Key benchmarks:
  - Parse + compile a single component (various sizes)
  - Full file compilation (multiple components)
  - Individual pass timing (which passes are hotspots)
- Comparison with Babel plugin (JS vs Rust performance)
- Memory usage profiling
- CI integration for performance regression detection

**Depends on:** Full pipeline working end-to-end
