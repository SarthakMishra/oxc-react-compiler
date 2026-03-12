# Upstream Conformance Testing

> Expand from zero upstream fixtures to full coverage of the ~2000+ React Compiler test suite.

**Priority:** P2 (Should Build Soon) -- not blocking correctness but essential for long-term parity and regression prevention.

---

## Current State

Infrastructure is fully built but not yet exercised:

- `crates/oxc_react_compiler/tests/conformance_tests.rs` -- test runner with `catch_unwind`, output normalization, known-failures support, auto-download via `OXC_DOWNLOAD_FIXTURES=1`
- `tests/conformance/download-upstream.sh` -- shell script to download fixtures via GitHub API
- `tests/conformance/run-upstream.mjs` -- Node.js script to generate `.expected` output files using Babel
- `tests/conformance/upstream-fixtures/` -- empty directory (only `.gitkeep`)
- `tests/conformance/known-failures.txt` -- empty (header comments only)
- `normalize_output()` in conformance_tests.rs handles import paths, whitespace, and cache variable name normalization
- CI (`ci.yml`) runs `cargo test --all` but since no fixtures exist, the conformance test silently passes (early return)

The upstream fixture source is:
`facebook/react` repo at `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/compiler`

---

## Gap 1: Download and Catalog Upstream Fixtures

**Current state:** Empty upstream-fixtures directory
**What's needed:**
- Run `download-upstream.sh` or use `OXC_DOWNLOAD_FIXTURES=1` to populate `tests/conformance/upstream-fixtures/`
- Verify the download captures all subdirectories (the upstream fixtures are organized into categories like `error.todo-flow`, `reduce-reactive-deps`, `fbt`, etc.)
- Document the total fixture count and directory structure
- Add `upstream-fixtures/` to `.gitignore` (fixtures should be downloaded on-demand, not committed -- verify this is already done)
- Verify the download script handles GitHub API rate limits gracefully (unauthenticated limit is 60 req/hr; the recursive tree endpoint may be fine as a single call)

**Files involved:**
- `tests/conformance/download-upstream.sh`
- `tests/conformance/upstream-fixtures/` (output)
- `.gitignore`

**Depends on:** None

---

## Gap 2: Generate Expected Outputs via Babel

**Current state:** `run-upstream.mjs` exists but has never been run against the full fixture set
**What's needed:**
- Run `node tests/conformance/run-upstream.mjs` to compile each fixture through the upstream Babel plugin and write `.expected` files alongside the inputs
- Handle fixtures that the upstream compiler intentionally rejects (error fixtures) -- these should produce `.expected-error` files or be categorized separately
- Verify that `run-upstream.mjs` correctly handles TypeScript fixtures (`.tsx`, `.ts`) vs plain JS
- The script must use the same Babel plugin version and default options that the upstream test suite uses
- Document which npm packages are required (likely `@babel/core`, `babel-plugin-react-compiler`, `@babel/preset-typescript`)

**Files involved:**
- `tests/conformance/run-upstream.mjs`
- `package.json` (dev dependencies for Babel)

**Depends on:** Gap 1 (fixtures must be downloaded first)

---

## Gap 3: Run Baseline Conformance and Triage Results

**Current state:** The conformance test runner is ready but has never been exercised
**What's needed:**
- Run `cargo test upstream_conformance` with fixtures and expected outputs in place
- Categorize all results into buckets:
  1. **Pass** -- output matches expected (after normalization)
  2. **Panic** -- compiler crashes (unwrap, assertion failure, stack overflow, etc.)
  3. **Divergence** -- compiles without panic but output differs from Babel
  4. **No expected** -- fixture compiled but no `.expected` file to compare against
- For panics, further categorize:
  - HIR lowering panics (unsupported AST patterns in `build_hir/`)
  - SSA/optimization panics (type mismatches, missing instruction kinds)
  - Codegen panics (unhandled `InstructionValue` variants)
- For divergences, categorize severity:
  - **Semantic** -- different memoization behavior (like availability-schedule)
  - **Cosmetic** -- same semantics but different variable names/formatting
  - **Conservative miss** -- OXC memoizes less (safe but suboptimal)
- Record baseline pass rate as a number to track over time

**Files involved:**
- `crates/oxc_react_compiler/tests/conformance_tests.rs`
- `tests/conformance/known-failures.txt` (output)

**Depends on:** Gap 1, Gap 2

---

## Gap 4: Populate known-failures.txt

**Current state:** Empty file with header comments
**What's needed:**
- After the baseline triage (Gap 3), populate `known-failures.txt` with all diverging fixtures
- Use categories in comments for organization:
  ```
  # --- Panics: HIR lowering ---
  error.todo-flow/some-fixture.tsx

  # --- Panics: codegen ---
  some-other-fixture.tsx

  # --- Divergences: semantic ---
  availability-schedule-like.tsx

  # --- Divergences: conservative miss ---
  another-fixture.tsx
  ```
- The conformance test (`upstream_conformance`) will then pass cleanly: it only fails on **unexpected** panics (panics not in known-failures.txt)
- This establishes the "ratchet" -- any future regression (previously passing fixture starts failing) will be caught by CI

**Files involved:**
- `tests/conformance/known-failures.txt`

**Depends on:** Gap 3

---

## Gap 5: Add Conformance to CI as Non-Blocking Check

**Current state:** CI runs `cargo test --all` but conformance silently skips when no fixtures are present
**What's needed:**
- Add a new CI job or step that:
  1. Downloads upstream fixtures (cache them between runs using GitHub Actions cache keyed on a date or upstream commit SHA)
  2. Generates expected outputs via Babel (also cacheable)
  3. Runs `cargo test upstream_conformance` with `OXC_DOWNLOAD_FIXTURES=1`
- Initially make this a **non-blocking** (`continue-on-error: true`) check so it reports but does not gate merges
- Once the known-failures list is stable, make it blocking (unexpected panics = CI failure)
- Consider caching the downloaded fixtures in a GitHub Actions artifact or cache to avoid re-downloading ~2000 files on every run

**Files involved:**
- `.github/workflows/ci.yml`
- Tests that need `npm install` for Babel (may need a Node.js step)

**Depends on:** Gap 4

---

## Gap 6: Iteratively Fix Panics to Increase Pass Rate

**Current state:** Unknown number of panics (baseline not yet run)
**What's needed:**
- After the baseline is established, work through panics in priority order:
  1. **Stack overflows / segfaults** -- these indicate infinite recursion or unbounded allocation, likely in BuildHIR or SSA
  2. **Assertion failures** -- indicate incorrect assumptions about HIR structure
  3. **Unwrap on None** -- indicate missing match arms or unhandled AST patterns
- Each fix should:
  - Remove the fixture from `known-failures.txt`
  - Add a targeted unit test if the fix is non-trivial
  - Verify no regressions in other fixtures
- Track pass rate over time: `(total - panics - divergences) / total`
- Goal: zero panics (all fixtures compile without crashing), even if output diverges

**Files involved:**
- Various files in `crates/oxc_react_compiler/src/` depending on the panic source
- `tests/conformance/known-failures.txt` (remove fixed entries)

**Depends on:** Gap 4 (baseline must exist)

---

## Gap 7: Fix High-Priority Divergences

**Current state:** Unknown scope until baseline is run; availability-schedule is a known semantic divergence
**What's needed:**
- After panics are eliminated, address semantic divergences (where OXC output has different runtime behavior than Babel)
- Conservative misses (OXC memoizes less) are lower priority -- they produce correct but suboptimal output
- Semantic differences (OXC memoizes differently or incorrectly) are higher priority
- Each fix should update known-failures.txt and verify no regressions
- This work overlaps with the render-equivalence todo (`.todo/render-equivalence.md`) for the benchmark fixtures

**Depends on:** Gap 6 (fix panics first, then divergences)

---

## Acceptance Criteria

1. `tests/conformance/upstream-fixtures/` can be populated via `download-upstream.sh` or `OXC_DOWNLOAD_FIXTURES=1`
2. `.expected` files are generated for all fixtures via `run-upstream.mjs`
3. `known-failures.txt` contains categorized list of all current divergences
4. `cargo test upstream_conformance` passes (no unexpected panics)
5. CI includes a conformance check (initially non-blocking)
6. Pass rate is tracked and documented
7. Zero panics (all fixtures compile without crashing)
