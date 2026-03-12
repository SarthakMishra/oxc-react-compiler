# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.

Last updated: 2026-03-12

---

## Active Work

_(Nothing active)_

---

## Blocked

_(Nothing blocked)_

---

## Completed Work (Archive)

All P0–P5 items have been implemented. Detail files have been removed.

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
