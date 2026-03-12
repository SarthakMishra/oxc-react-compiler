# Testing and CI Hardening

> Tracking gaps in test coverage, conformance testing, and CI pipeline robustness.

Last updated: 2026-03-12

---

## Gap 1: Upstream conformance fixture suite

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/`
**Current state:** We have a conformance harness (completed, see archive) but need to expand coverage. The upstream has ~500 fixture tests covering edge cases across all pipeline passes.
**What's needed:**
- Download/sync the full upstream fixture set (automate with a script)
- Run each fixture through both our compiler and the upstream Babel compiler
- Diff the outputs and classify results: pass / fail / known-divergence
- Track a conformance percentage and trend it over time
- Prioritize fixing failures in order of frequency in real-world code
- Gate CI on "no regressions" (conformance score must not decrease)
**Depends on:** None

---

## Gap 2: Per-pass snapshot tests (insta)

**Current state:** E2E tests exist (31 tests) but they test the full pipeline end-to-end. Individual passes lack unit-level snapshot tests.
**What's needed:**
- For each pipeline pass, create snapshot tests using `insta` that show the HIR/IR before and after the pass runs
- Priority passes for snapshot testing:
  1. `enter_ssa.rs` -- SSA phi placement and renaming
  2. `infer_mutation_aliasing_effects.rs` -- effect inference
  3. `infer_reactive_places.rs` -- reactivity marking
  4. `infer_reactive_scope_variables.rs` -- scope assignment
  5. `propagate_dependencies.rs` -- dependency collection
  6. `build_reactive_function.rs` -- reactive tree construction
  7. `codegen.rs` -- final output
- Use `insta::assert_snapshot!` with a custom HIR pretty-printer
- Need a `Display` or `Debug` impl for HIR that produces stable, diffable output
**Depends on:** HIR pretty-printer (may already exist for debugging)

---

## Gap 3: Babel output comparison (differential testing)

**Current state:** Some infrastructure exists (see archive: "Upstream Conformance" and "Benchmark Suite").
**What's needed:**
- For each fixture, run through both compilers and produce a normalized diff
- Normalization: strip whitespace differences, normalize variable names (t0/t1 ordering may differ), normalize import ordering
- Classify differences:
  - **Semantic match** -- same behavior, different syntax (acceptable)
  - **Missing feature** -- we skip a transform upstream applies (track as gap)
  - **Wrong output** -- we produce different semantics (bug, must fix)
- Store diffs as checked-in snapshots (git diff --exit-code in CI)
**Depends on:** Gap 1 (fixture suite)

---

## Gap 4: CI pipeline hardening

**Current state:** Basic CI (cargo test, cargo build).
**What's needed:**
- Add `cargo clippy -- -D warnings` (fail on any clippy warning)
- Add `cargo fmt --check` (fail on unformatted code)
- Add conformance snapshot check (fail if snapshots change without explicit update)
- Add NAPI build + basic smoke test (ensure the Node.js binding loads and can compile a simple component)
- Add benchmark regression check (run benchmarks, compare against baseline, warn on >10% regression)
- Consider adding `cargo deny` for license/advisory checking
- Consider adding `cargo semver-checks` before releases
**Depends on:** None

---

## Gap 5: Property-based / fuzz testing for HIR construction

**Current state:** No fuzz testing.
**What's needed:**
- Use `cargo-fuzz` or `proptest` to generate random valid JavaScript ASTs and feed them through the HIR builder
- Goal: find panics, infinite loops, or assertion failures in `build.rs`
- This is particularly valuable because `build.rs` has many `expect()` calls (see code-quality.md Gap 3) that could panic on unexpected AST shapes
- Start with a simple fuzzer that generates function declarations with various statement/expression combinations
- Lower priority but high value for robustness
**Depends on:** None

---

## Gap 6: Integration test for Vite plugin hot reload

**Current state:** Unknown whether hot reload (HMR) works correctly with compiled output.
**What's needed:**
- Create an integration test that:
  1. Starts a Vite dev server with the plugin enabled
  2. Compiles a component
  3. Modifies the component source
  4. Verifies HMR produces correct re-compilation
  5. Verifies the `enableResetCacheOnSourceFileChanges` flag produces the right cache-busting code
- This can use Vite's programmatic API to avoid needing a browser
**Depends on:** Vite plugin being functional
