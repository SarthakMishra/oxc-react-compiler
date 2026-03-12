# Code Quality, Performance, and Maintainability

> Issues identified from internal audit, clone/allocation analysis, and comparison
> with the oxc-angular-compiler reference patterns.

Last updated: 2026-03-12

---

## Part A: Error Handling (panicking public APIs)

### Gap 1: `disjoint_set.rs:36` -- `expect()` in public API

**File:** `crates/oxc_react_compiler/src/utils/disjoint_set.rs`
**Current state:** `expect("element not found")` on a map lookup in `find()`. If called with an unregistered element, this panics.
**What's needed:**
- Return `Option` or `Result` from `find()` and `union()`
- Propagate the error to callers (or add a `contains()` pre-check pattern)
- The disjoint set is used in scope merging -- a panic here crashes the entire compilation

---

### Gap 2: `ordered_map.rs:87` -- `expect()` in Index impl

**File:** `crates/oxc_react_compiler/src/utils/ordered_map.rs`
**Current state:** `Index` trait impl uses `expect()` which panics on missing keys.
**What's needed:**
- This is somewhat idiomatic for `Index` (HashMap does the same), but consider adding a safe `get()` method and using it in callers that cannot guarantee key existence
- Audit callers to determine if any use Index with potentially-missing keys

---

### Gap 3: `hir/build.rs` -- multiple `expect()`/`unwrap()` on block existence

**File:** `crates/oxc_react_compiler/src/hir/build.rs`
**Lines:** ~218, 321, 375, 428 (approximate)
**Current state:** Several `expect("block should exist")` calls when looking up blocks by ID.
**What's needed:**
- These are internal invariant assertions (the block was just created), so `expect()` is reasonable
- However, if HIR construction has a bug, the panic message is unhelpful
- Consider using `debug_assert!` for performance or wrapping in a helper that provides the block ID in the error message
- Lower priority than Gap 1 since these are internal invariants, not public API surface

---

### Gap 4: `ssa/enter_ssa.rs:160,163` -- `unwrap()` on dominator map

**File:** `crates/oxc_react_compiler/src/ssa/enter_ssa.rs`
**Current state:** `unwrap()` on dominator tree lookups. If the dominator computation is incomplete, this panics.
**What's needed:**
- Add context to the unwrap (`.expect("block {id} must have a dominator")`)
- Or return Result if dominator computation can legitimately fail (e.g., unreachable blocks)

---

## Part B: Performance (hot-path allocations)

### Gap 5: `place.clone()` proliferation in reactive scope analysis

**Files:** `infer_reactive_scope_variables.rs`, `propagate_dependencies.rs`, `align_scopes.rs`, `merge_scopes.rs`
**Current state:** 65+ `place.clone()` calls across reactive scope passes. `Place` contains an `Identifier` with a `String` name, making each clone allocate.
**What's needed:**
- Consider `Rc<Place>` or arena-allocated places to share without cloning
- Alternatively, use `Place` indices (u32 IDs) and a side table, similar to how HIR uses `BlockId`
- Profile first to confirm this is actually a bottleneck -- may not matter for typical component sizes
- **Risk:** Changing Place representation is a large refactor touching many files
**Depends on:** Profiling data

---

### Gap 6: `.to_string()` on identifiers in hot paths

**Files:** `hir/build.rs` (property key mapping), `codegen.rs`, `infer_types.rs`
**Current state:** 70+ `.to_string()` calls converting `&str` or atom references to owned `String`. Particularly visible in `build.rs` for property key handling.
**What's needed:**
- Use `Cow<'_, str>` or OXC's `Atom` type for identifier names where the string is borrowed from the input AST
- Most property keys come from AST string slices and only need to be owned if they outlive the AST
- Focus on `build.rs` first (highest call count)
**Depends on:** Understanding of OXC `Atom` lifetime model

---

### Gap 7: `infer_reactive_scope_variables.rs` -- double allocation

**File:** `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`
**Line:** ~150, 155
**Current state:** `Box::new(scope.clone())` creates a clone of a scope struct, then boxes it. Two allocations for one value.
**What's needed:**
- Clone into the box directly: `Box::new(scope)` and restructure to avoid needing the original after boxing
- Or use `Rc` if the scope needs to be shared

---

### Gap 8: `codegen.rs:550` -- unnecessary format string

**File:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
**Line:** ~550
**Current state:** `format!("t{}", id)` to create temporary variable names. Called once per temporary in codegen.
**What's needed:**
- Pre-compute a `SmallVec` or use `write!` to a reusable buffer
- Or use OXC's atom interning if available
- Low priority -- codegen runs once per function, not a hot loop

---

## Part C: Maintainability

### Gap 9: `#![allow(dead_code)]` on ~40 files

**Current state:** Nearly every module file has `#![allow(dead_code)]` at the top, suppressing warnings for unused functions, structs, and enum variants.
**What's needed:**
- Audit each file: remove dead code that will never be used, or mark items `pub` if they are part of the public API
- For items that are "planned but not yet wired," add a `// TODO:` comment explaining the intended use
- Consider using `#[cfg(test)]` for test-only utilities
- Remove the blanket `#![allow(dead_code)]` once cleaned up
- This can be done incrementally, one module at a time

---

### Gap 10: Missing `// DIVERGENCE:` comments for intentional algorithm differences

**Current state:** No `// DIVERGENCE:` comments exist anywhere in the codebase. There are at least two known intentional divergences from upstream:
1. Dominance computation uses Cooper-Harvey-Kennedy iterative algorithm instead of Lengauer-Tarjan (upstream uses a different approach)
2. `outline_jsx.rs` claims HIR lowering already handles JSX outlining (see pipeline-completeness.md Gap 5)
**What's needed:**
- Add `// DIVERGENCE: <reason>` comments wherever the implementation intentionally differs from upstream
- This makes it easy to audit during upstream merges and prevents accidental "fixes" that revert intentional choices
- Grep for TODOs and known differences, add DIVERGENCE markers

---

### Gap 11: Missing `React.forwardRef` / `React.memo` wrapper handling in function discovery

**File:** `crates/oxc_react_compiler/src/entrypoint/program.rs`
**Line:** ~521
**Current state:** There is a TODO comment noting this gap. The compiler's function discovery (`program.rs`) does not unwrap `React.forwardRef(function Component() { ... })` or `React.memo(function Component() { ... })` to find the inner function for compilation.
**What's needed:**
- Detect `React.forwardRef(fn)` and `React.memo(fn)` call patterns in the AST
- Extract the inner function expression and compile it
- Handle nested wrappers: `React.memo(React.forwardRef(fn))`
- This is important for real-world codebases that heavily use these HOCs

---

## Part D: Patterns from oxc-angular-compiler

### Gap 12: Aggressive clippy lint configuration

**Reference:** `oxc-angular-compiler/Cargo.toml` and `.cargo/config.toml`
**Current state:** Basic clippy configuration. No pedantic or nursery lints enabled.
**What's needed:**
- Enable `clippy::pedantic`, `clippy::nursery`, `clippy::cargo` lint groups in workspace `Cargo.toml`
- Add targeted `#[allow(...)]` for specific pedantic lints that are too noisy (e.g., `clippy::module_name_repetitions`)
- Fix all new warnings
- This catches real bugs (unchecked casts, missing error handling, unidiomatic patterns)

---

### Gap 13: Release profile optimization

**Reference:** `oxc-angular-compiler/Cargo.toml`
**Current state:** Default release profile.
**What's needed:**
- Add to workspace `Cargo.toml`:
  ```toml
  [profile.release]
  lto = "fat"
  codegen-units = 1
  panic = "abort"
  strip = "symbols"
  ```
- `lto = "fat"` enables cross-crate inlining for maximum performance
- `codegen-units = 1` maximizes optimization at the cost of compile time
- `panic = "abort"` eliminates unwinding overhead (acceptable for a compiler tool)
- `strip = "symbols"` reduces binary size

---

### Gap 14: NAPI never-throw pattern

**Reference:** `oxc-angular-compiler` NAPI layer
**Current state:** Unknown -- need to audit the NAPI binding layer.
**What's needed:**
- NAPI functions should never throw JS exceptions
- Return result structs with an `errors: Vec<Diagnostic>` array
- The Vite plugin layer handles error reporting
- This prevents Node.js crashes from Rust panics and gives the plugin control over error presentation

---

### Gap 15: Enum size control assertions

**Reference:** `oxc-angular-compiler` uses `static_assertions`
**Current state:** No size assertions on key enums.
**What's needed:**
- Add `static_assertions::assert_eq_size!` for critical enums like `InstructionValue`, `TerminalValue`, `Type`
- Prevents accidental size regressions when adding variants (a common issue with Rust enums -- one large variant bloats all values)
- Add to a `size_tests.rs` module or inline in the type definition files

---

### Gap 16: mimalloc allocator

**Reference:** `oxc-angular-compiler` uses mimalloc with platform feature flags
**Current state:** Default system allocator.
**What's needed:**
- Add `mimalloc` dependency with platform-specific features
- Set as global allocator in `lib.rs` or the NAPI entry point
- Typical 10-20% throughput improvement for allocation-heavy workloads (the compiler does many small allocations for HIR nodes)
- Gate behind a feature flag for easy toggling
