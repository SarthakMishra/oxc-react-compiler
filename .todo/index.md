# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.

Last updated: 2026-03-12

Current conformance: 374/1717 pass (21.8%), 0 panics, 1304 divergences across 2 categories.

---

## Active Work

_(Nothing active)_

---

## Priority 1 -- Conformance: TS Type Stripping ~~(~138 fixtures)~~ DONE

- [x] Add oxc_codegen + oxc_transformer as dev-dependencies
- [x] Implement parse-print roundtrip for type stripping in conformance tests
- [x] JSX normalization via OXC transformer (lower JSX to _jsx in both sides)
- [x] Bail on all validation errors (AllErrors threshold) — +24 fixtures
- [x] Skip functions with zero cache slots (no reactive scopes) — +90 fixtures
- [x] Verify impact and update known-failures.txt

## Priority 2 -- Conformance: Memoization Structure (~606 fixtures)

Deep compiler work needed — temp variable explosion, scope analysis, cache slots:

- [ ] Temp variable inlining pass (collapse SSA chains in codegen) -- [memoization-structure.md](memoization-structure.md)#gap-1-temp-variable-inlining-pass
- [ ] JSX syntax preservation in codegen (emit `<div>` not `_jsx()`) -- [memoization-structure.md](memoization-structure.md)#gap-2-jsx-syntax-preservation-in-codegen
- [ ] Cache slot count alignment -- [memoization-structure.md](memoization-structure.md)#gap-3-cache-slot-count-alignment
- [ ] Scope merging/splitting heuristic audit vs upstream -- [memoization-structure.md](memoization-structure.md)#gap-4-scope-mergingsplitting-heuristic-review

## Priority 3 -- Conformance: Over-Memoization Bail-Out (~698 fixtures)

Missing validation logic — our compiler compiles functions Babel skips:

- [ ] Categorize bail-out fixtures (triage script) -- [over-memoization-bailout.md](over-memoization-bailout.md)#gap-1-categorize-bail-out-fixtures
- [x] Validation-error bail-out threshold (match Babel error severities) — DONE (AllErrors threshold)
- [x] Zero-scope bail-out (return original source when no reactive scopes) — DONE
- [ ] Audit validation passes for error accuracy vs upstream -- [over-memoization-bailout.md](over-memoization-bailout.md)#gap-3-ensure-validation-passes-emit-correct-errors
- [ ] Mutation aliasing bail-out (escaped values analysis) -- [over-memoization-bailout.md](over-memoization-bailout.md)#gap-5-mutation-aliasing-bail-out
- [ ] "Too simple" function detection -- [over-memoization-bailout.md](over-memoization-bailout.md)#gap-6-too-simple-function-detection

## Priority 4 -- Performance / Polish

_(All previous items complete)_

---

## Blocked

_(Nothing blocked)_

---

## Completed Work (Archive)

All P0-P5 items have been implemented. Detail files have been removed.

### Conformance Quick Wins (2026-03-12)

- [x] TS type stripping via OXC parse→transform→print roundtrip (+30 fixtures)
- [x] JSX normalization via OXC transformer (normalize JSX representation)
- [x] Bail on all validation errors (AllErrors threshold, +24 fixtures)
- [x] Skip functions with zero cache slots (+90 fixtures)
- Total: 230/1717 → 374/1717 (+144 fixtures, 13.4% → 21.8%)

### Render Equivalence (formerly render-equivalence.md)

- Availability-schedule truncated output (zero memoization) fixed
- Phi-node / temporary variable resolution in ternary/logical branches fixed
- JSX hyphenated attribute name quoting (aria-label, data-*) fixed
- Multi-step-form timeout/segfault investigated and resolved
- Conservative memoization misses addressed (JSX codegen format, param destructuring, bail-out heuristics, scope analysis; conformance ratchet 163/1717)
- Render equivalence tracking added to CI

### Upstream Conformance (formerly upstream-conformance.md)

- Upstream fixtures downloaded with expected outputs generated
- Baseline conformance run and triaged (158/1717 pass, 0 panics)
- known-failures.txt populated; conformance added to CI as non-blocking check
- Panics fixed; high-priority divergences resolved (JSX parsing for .js files, conformance normalization)

### Vite Caching (formerly vite-caching.md)

- In-memory content-hash cache added to Vite plugin
- Config change invalidation implemented
- Optional disk cache for large projects added
- Performance measured via vite-cache-bench.mjs script

### P0: Critical Bugs

- [x] Destructured params not emitted in codegen (build.rs + codegen destructure pattern)
- [x] Dependency filter drops all scope dependencies (propagate_dependencies.rs + prune_scopes.rs)
- [x] canvas-sidebar 16ms outlier (O(N^2) fixed-point loop replaced with O(N) forward pass in infer_reactive_scope_variables; mimalloc, Cow codegen, double-alloc fix)

### P1: Correctness

- [x] ComputeUnconditionalBlocks (post-dominator analysis with RPO)
- [x] CollectHoistablePropertyLoads
- [x] CollectOptionalChainDependencies
- [x] DeriveMinimalDependenciesHIR
- [x] ScopeDependencyUtils
- [x] Type-based ref/setState detection in validation passes

### P2: Upstream Parity

- [x] ExhaustiveDepsMode enum, ExternalFunctionConfig, OutputMode::ClientNoMemo
- [x] Config gates: assertValidMutableRanges, enableNameAnonymousFunctions, enableTreatSetIdentifiersAsStateSetters, enableAllowSetStateFromRefsInEffects, enableVerboseNoSetStateInEffect
- [x] Validation passes: validateNoImpureFunctionsInRender, validateBlocklistedImports, validateNoVoidUseMemo, assertWellFormedBreakTargets
- [x] Enhanced passes: optimize_for_ssr, validate_static_components, outline_functions, outline_jsx (verified no-op)
- [x] PruneTemporaryLValues optimization

### P3: Code Quality

- [x] DisjointSet Option API, expect/unwrap message improvements, ordered_map audit
- [x] React.forwardRef/memo wrapper handling in program.rs
- [x] DIVERGENCE comments across 7 files
- [x] #![allow(dead_code)] reduced from ~40 to 5 files
- [x] Cow-based identifier display in codegen (place_name, identifier_display_name)
- [x] infer_reactive_scope_variables double allocation fix
- [x] place.clone() investigated (no hotspot found)

### P4: Testing and CI

- [x] Upstream conformance fixture suite (conformance_tests.rs with auto-download)
- [x] Per-pass insta snapshot tests (6 fixtures)
- [x] Babel output differential testing (bench.mjs --diff)
- [x] CI pipeline (ci.yml: clippy -D warnings, fmt check, release build)
- [x] Proptest fuzz testing (4 strategies in fuzz_hir.rs)
- [x] Vite plugin HMR support (handleHotUpdate in vite-plugin/index.ts)

### P5: Polish

- [x] Aggressive clippy lints (pedantic, nursery, cargo)
- [x] Release profile (LTO fat, codegen-units 1, strip symbols, panic abort)
- [x] NAPI never-throw (catch_unwind with safe fallbacks)
- [x] Enum size control (const assert! for InstructionValue, Terminal, Place, Instruction)
- [x] mimalloc global allocator in NAPI crate
- [x] codegen.rs Cow optimization for place_name/identifier_display_name

### Earlier Completed Work

- Scope boundary alignment, codegen correctness, memoization pipeline
- Tier 2 lint rules (Rules of Hooks, immutability, manual memoization, exhaustive deps)
- Source maps, upstream conformance infra, documentation
- Clippy cleanup (258 fixes, zero warnings), benchmark suite (16 real-world fixtures)
- Test coverage: 156 tests across config, diagnostics, codegen, E2E, snapshots, fuzz
