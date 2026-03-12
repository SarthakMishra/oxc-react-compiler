# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.
> Items are ordered by dependency: nothing should be blocked by an item below it.

Last updated: 2026-03-12

---

## Active Work

_(Nothing active)_

---

## P0: Critical Bugs (blocking all memoization correctness and render equivalence)

- [x] **BUG: Destructured params not emitted in codegen** -- `build.rs` creates temp params for destructured args but never emits `Destructure` instructions, causing "X is not defined" for ALL compiled components -- [critical-bugs.md](critical-bugs.md)#bug-1-destructured-parameters-not-emitted
- [x] **BUG: Dependency filter drops all scope dependencies** -- `propagate_dependencies.rs:136` filters `.filter(|d| d.reactive)` but Place objects' `.reactive` field isn't propagated from identifier reactivity, resulting in empty deps → sentinel-only checks → no invalidation -- [critical-bugs.md](critical-bugs.md)#bug-2-dependency-filter-drops-all-scope-dependencies
- [ ] **BUG: canvas-sidebar 16ms outlier** -- 272 LOC fixture takes 16.6ms (10x expected), likely a pathological case in scope inference or mutation analysis causing quadratic behavior -- [critical-bugs.md](critical-bugs.md)#bug-3-canvas-sidebar-performance-outlier

---

## P1: Correctness (missing passes that affect output precision)

- [x] ComputeUnconditionalBlocks -- unconditional execution analysis for CFG -- [pipeline-completeness.md](pipeline-completeness.md)#gap-11-computeunconditionalblocks----unconditional-execution-analysis
- [x] CollectHoistablePropertyLoads -- property load hoisting via non-null guarantees -- [pipeline-completeness.md](pipeline-completeness.md)#gap-7-collecthoistablepropertyloads----critical-for-dependency-precision
- [x] CollectOptionalChainDependencies -- optional chain dependency semantics -- [pipeline-completeness.md](pipeline-completeness.md)#gap-8-collectoptionalchaindependencies----critical-for-optional-chain-correctness
- [x] DeriveMinimalDependenciesHIR -- tree-based dependency minimization -- [pipeline-completeness.md](pipeline-completeness.md)#gap-9-deriveminimaldependencieshir----dependency-tree-minimization
- [x] ScopeDependencyUtils -- shared dependency manipulation utilities -- [pipeline-completeness.md](pipeline-completeness.md)#gap-10-scopedependencyutils----shared-dependency-utilities
- [x] validate_no_ref_access_in_render: type-based ref detection instead of naming heuristic -- [pipeline-completeness.md](pipeline-completeness.md)#gap-2-validate_no_ref_access_in_renderrs----naming-heuristic-only
- [x] validate_no_set_state_in_render: type-based setState detection instead of naming heuristic -- [pipeline-completeness.md](pipeline-completeness.md)#gap-3-validate_no_set_state_in_renderrs----naming-heuristic-only

---

## P2: Upstream Parity (config flags, output modes, missing validation passes)

- [x] OutputMode::ClientNoMemo variant for benchmarking -- [config-parity.md](config-parity.md)#gap-5-outputmodeclientnomemo-variant-missing
- [x] validateExhaustiveEffectDependencies enum (off/all/missing-only/extra-only) -- [config-parity.md](config-parity.md)#gap-1-validateexhaustiveeffectdependencies-should-be-an-enum
- [x] enableEmitHookGuards ExternalFunction config -- [config-parity.md](config-parity.md)#gap-2-enableemithookguards-should-accept-externalfunction-config
- [x] validateNoImpureFunctionsInRender validation pass -- [config-parity.md](config-parity.md)#gap-6-validatenoimpurefunctionsinrender-validation-pass-missing
- [x] validateBlocklistedImports validation pass -- [config-parity.md](config-parity.md)#gap-7-validateblocklistedimports-validation-pass-missing
- [x] validateNoVoidUseMemo overlap check -- [config-parity.md](config-parity.md)#gap-8-validatenovoidusememo-overlap-check
- [x] enableTreatSetIdentifiersAsStateSetters heuristic -- [config-parity.md](config-parity.md)#gap-9-enabletreatsetidentifiersasstatesetters-heuristic
- [x] enableAllowSetStateFromRefsInEffects nuance -- [config-parity.md](config-parity.md)#gap-10-enableallowsetstatefromrefsineffects-nuance
- [x] enableVerboseNoSetStateInEffect richer diagnostics -- [config-parity.md](config-parity.md)#gap-11-enableverbosenosetstateineffect-richer-diagnostics
- [x] assertValidMutableRanges config gate -- [config-parity.md](config-parity.md)#gap-3-assertvalidmutableranges-should-be-config-gated
- [x] enableNameAnonymousFunctions config gate -- [config-parity.md](config-parity.md)#gap-4-enablenameanonymousfunctions-config-gate-missing
- [x] optimize_for_ssr.rs: comprehensive memoization stripping -- [pipeline-completeness.md](pipeline-completeness.md)#gap-1-optimize_for_ssrrs----minimal-ssr-stripping
- [x] validate_static_components.rs: React.memo detection, scope analysis -- [pipeline-completeness.md](pipeline-completeness.md)#gap-4-validate_static_componentsrs----pascalcase-check-only
- [x] outline_jsx.rs: verify no-op claim or implement outlining -- [pipeline-completeness.md](pipeline-completeness.md)#gap-5-outline_jsxrs----effectively-a-no-op
- [x] outline_functions.rs: actual hoisting + codegen support -- [pipeline-completeness.md](pipeline-completeness.md)#gap-6-outline_functionsrs----identifies-candidates-but-no-hoisting
- [x] assertWellFormedBreakTargets validation -- [pipeline-completeness.md](pipeline-completeness.md)#gap-12-assertwellformedbreaktargets----break-target-validation
- [x] PruneTemporaryLValues optimization -- [pipeline-completeness.md](pipeline-completeness.md)#gap-13-prunetemporarylvalues----temporary-cleanup

---

## P3: Code Quality (error handling, performance, maintainability)

- [x] disjoint_set.rs: replace expect() with Result in public API -- [code-quality.md](code-quality.md)#gap-1-disjoint_setrs36----expect-in-public-api
- [x] hir/build.rs: improve expect() messages on block lookups -- [code-quality.md](code-quality.md)#gap-3-hirbuildrs----multiple-expectunwrap-on-block-existence
- [x] ssa/enter_ssa.rs: add context to unwrap() on dominator map -- [code-quality.md](code-quality.md)#gap-4-ssaenter_ssars160163----unwrap-on-dominator-map
- [x] ordered_map.rs: audit Index impl callers for safety -- [code-quality.md](code-quality.md)#gap-2-ordered_maprs87----expect-in-index-impl
- [x] React.forwardRef / React.memo wrapper handling in program.rs -- [code-quality.md](code-quality.md)#gap-11-missing-reactforwardref--reactmemo-wrapper-handling-in-function-discovery
- [x] Add DIVERGENCE comments for intentional algorithm differences -- [code-quality.md](code-quality.md)#gap-10-missing--divergence-comments-for-intentional-algorithm-differences
- [x] Audit and remove #![allow(dead_code)] from ~40 files -- [code-quality.md](code-quality.md)#gap-9-allowdead_code-on-40-files
- [ ] place.clone() proliferation -- consider Rc or arena allocation -- [code-quality.md](code-quality.md)#gap-5-placeclone-proliferation-in-reactive-scope-analysis
- [ ] .to_string() on identifiers -- use Cow/Atom where possible -- [code-quality.md](code-quality.md)#gap-6-to_string-on-identifiers-in-hot-paths
- [x] infer_reactive_scope_variables.rs double allocation -- [code-quality.md](code-quality.md)#gap-7-infer_reactive_scope_variablesrs----double-allocation

---

## P4: Testing and CI

- [ ] Upstream conformance fixture suite (~500 fixtures) -- [testing-hardening.md](testing-hardening.md)#gap-1-upstream-conformance-fixture-suite
- [ ] Per-pass insta snapshot tests -- [testing-hardening.md](testing-hardening.md)#gap-2-per-pass-snapshot-tests-insta
- [ ] Babel output differential testing -- [testing-hardening.md](testing-hardening.md)#gap-3-babel-output-comparison-differential-testing
- [x] CI pipeline hardening (clippy -D warnings, fmt check, snapshots) -- [testing-hardening.md](testing-hardening.md)#gap-4-ci-pipeline-hardening
- [ ] Fuzz testing for HIR construction -- [testing-hardening.md](testing-hardening.md)#gap-5-property-based--fuzz-testing-for-hir-construction
- [ ] Vite plugin HMR integration test -- [testing-hardening.md](testing-hardening.md)#gap-6-integration-test-for-vite-plugin-hot-reload

---

## P5: Polish (build config, allocator, NAPI patterns)

- [x] Aggressive clippy lints (pedantic, nursery, cargo) -- [code-quality.md](code-quality.md)#gap-12-aggressive-clippy-lint-configuration
- [x] Release profile optimization (LTO, codegen-units, strip) -- [code-quality.md](code-quality.md)#gap-13-release-profile-optimization
- [x] NAPI never-throw pattern -- [code-quality.md](code-quality.md)#gap-14-napi-never-throw-pattern
- [x] Enum size control assertions -- [code-quality.md](code-quality.md)#gap-15-enum-size-control-assertions
- [ ] mimalloc allocator -- [code-quality.md](code-quality.md)#gap-16-mimalloc-allocator
- [ ] codegen.rs format!() for temp names -- [code-quality.md](code-quality.md)#gap-8-codegenrs550----unnecessary-format-string

---

## Blocked

_(Nothing blocked)_

---

## Completed Work (Archive)

- **P1: Scope boundary alignment** -- Name-based reactivity propagation in `infer_reactive_places`, hook detection via LoadGlobal binding name resolution, scope terminal self-referential fallthrough fix
- **Codegen Correctness** -- Fixed `promote_used_temporaries` to walk all places (operands, terminals, scope deps/decls), fixed `place_name()` fallback to use `t{id}` instead of `_t{id}`, JSX naming fixed as a consequence
- **Memoization Pipeline** -- Fixed scope assignment in `infer_reactive_scope_variables`, reactive param marking in `infer_reactive_places`, range extension in `infer_mutation_aliasing_ranges`, dependency/declaration population in `propagate_scope_dependencies_hir`, instruction dedup in `build_reactive_function`, added E2E memoization snapshot test, destructure pattern target propagation
- **Tier 2 Lint Rules** -- Full Rules of Hooks with CFG analysis, immutability checking, manual memoization preservation, exhaustive memo deps, exhaustive effect deps, structured DiagnosticKind filtering
- **Source Maps** -- Source map generation from codegen, NAPI passthrough, Vite plugin wiring, whole-file source map composition
- **Upstream Conformance** -- Fixture download, upstream oracle runner, differential comparison harness, output normalization
- **Documentation** -- Vite plugin usage guide, lint rules docs, configuration reference, known limitations
- **Clippy Cleanup** -- 258 mechanical fixes, crate-level allows for style lints, zero warnings across workspace
- **Test Coverage** -- Config parsing (10 tests), error diagnostics (17/17 variants, 26 tests), post-codegen validation (6 semantic tests), E2E dual-mode (31 tests), sprout runtime eval (11 tests)
- **Benchmark Suite** -- Real-world fixtures (16, 4 per tier), benchmark harness v2 with NAPI timing, deep correctness analysis (AST diff + render comparison), differential snapshots, CI integration, correctness score documentation
- **P0 Fixes (iteration 1)** -- Destructured param emission in build.rs + codegen, dependency filter fix in propagate_dependencies.rs + prune_scopes.rs
- **P2 Config (iteration 2)** -- assertValidMutableRanges gate, enableNameAnonymousFunctions gate, OutputMode::ClientNoMemo, outline_jsx DIVERGENCE verification
- **P1 Passes (iteration 3)** -- ComputeUnconditionalBlocks (post-dominator analysis with RPO), CollectHoistablePropertyLoads, CollectOptionalChainDependencies, DeriveMinimalDependenciesHIR, ScopeDependencyUtils, Type::Ref/SetState + type-based validation
- **P2 Passes (iteration 4)** -- ExhaustiveDepsMode enum, ExternalFunctionConfig, validateNoImpureFunctionsInRender, validateBlocklistedImports, validateNoVoidUseMemo, assertWellFormedBreakTargets, PruneTemporaryLValues, enableTreatSetIdentifiersAsStateSetters/enableAllowSetStateFromRefsInEffects/enableVerboseNoSetStateInEffect config flags
