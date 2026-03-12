# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.

Last updated: 2026-03-12

---

## Active Work

_(Nothing active)_

---

## Priority 1 -- Correctness

- [ ] Fix availability-schedule truncated output (zero memoization) -- [render-equivalence.md](render-equivalence.md)#gap-1-availability-schedule-truncated-output
- [ ] Fix phi-node / temporary variable resolution in ternary/logical branches -- [render-equivalence.md](render-equivalence.md)#gap-2-phi-node--temporary-variable-resolution
- [ ] Fix JSX hyphenated attribute name quoting (aria-label, data-*) -- [render-equivalence.md](render-equivalence.md)#gap-3-jsx-hyphenated-attribute-names
- [ ] Investigate multi-step-form timeout/segfault -- [render-equivalence.md](render-equivalence.md)#gap-4-multi-step-form-timeoutsegfault

## Priority 2 -- Upstream Parity

- [ ] Download upstream fixtures and generate expected outputs -- [upstream-conformance.md](upstream-conformance.md)#gap-1-download-and-catalog-upstream-fixtures
- [ ] Run baseline conformance and triage results -- [upstream-conformance.md](upstream-conformance.md)#gap-3-run-baseline-conformance-and-triage-results
- [ ] Populate known-failures.txt -- [upstream-conformance.md](upstream-conformance.md)#gap-4-populate-known-failurestxt
- [ ] Add conformance to CI as non-blocking check -- [upstream-conformance.md](upstream-conformance.md)#gap-5-add-conformance-to-ci-as-non-blocking-check
- [ ] Fix panics to increase upstream pass rate -- [upstream-conformance.md](upstream-conformance.md)#gap-6-iteratively-fix-panics-to-increase-pass-rate
- [ ] Fix high-priority upstream divergences -- [upstream-conformance.md](upstream-conformance.md)#gap-7-fix-high-priority-divergences
- [ ] Address conservative memoization misses -- [render-equivalence.md](render-equivalence.md)#gap-5-conservative-memoization-misses

## Priority 3 -- Performance / Polish

- [ ] Add in-memory content-hash cache to Vite plugin -- [vite-caching.md](vite-caching.md)#gap-1-in-memory-content-hash-cache
- [ ] Add config change invalidation for cache -- [vite-caching.md](vite-caching.md)#gap-2-config-change-invalidation
- [ ] Optional disk cache for large projects -- [vite-caching.md](vite-caching.md)#gap-3-optional-disk-cache-for-large-projects
- [ ] Measure Vite plugin caching performance improvement -- [vite-caching.md](vite-caching.md)#gap-4-performance-measurement
- [ ] Add render equivalence tracking to CI -- [render-equivalence.md](render-equivalence.md)#gap-6-test-infrastructure-for-render-equivalence

---

## Blocked

_(Nothing blocked)_

---

## Completed Work (Archive)

All P0-P5 items have been implemented. Detail files have been removed.

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
