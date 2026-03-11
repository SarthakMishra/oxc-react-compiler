# OXC React Compiler — Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.
> Items are ordered by dependency: nothing should be blocked by an item below it.

Last updated: 2026-03-11

---

## Active Work

_(Nothing in progress)_

---

## Priority 1 — Core Pipeline (must build in order)

### Phase 1: HIR Foundation

- [x] HIR core types (Place, Identifier, MutableRange, ReactiveScope) — [hir-types.md](hir-types.md)#gap-1-core-id-newtypes-and-place-types
- [x] InstructionValue enum (~40 variants) — [hir-types.md](hir-types.md)#gap-2-instructionvalue-enum
- [x] Terminal enum (~20 variants) — [hir-types.md](hir-types.md)#gap-3-terminal-enum
- [x] HIR container types (HIRFunction, HIR, BasicBlock, Phi) — [hir-types.md](hir-types.md)#gap-4-hir-container-types
- [x] Effect, ValueKind, and supporting enums — [hir-types.md](hir-types.md)#gap-5-effect-valuekind-and-supporting-enums
- [x] ReactiveFunction IR types — [hir-types.md](hir-types.md)#gap-6-reactivefunction-ir-types
- [x] Environment and EnvironmentConfig — [environment.md](environment.md)#gap-1-environment-struct
- [x] PluginOptions and CompilationMode — [environment.md](environment.md)#gap-2-pluginoptions-and-compilationmode
- [x] ObjectShape, ShapeRegistry, FunctionSignature — [environment.md](environment.md)#gap-3-objectshape-shaperegistry-functionsignature
- [x] Globals registry (built-in shapes for Array, Object, hooks) — [environment.md](environment.md)#gap-4-globals-registry
- [x] CompilerError expansion (severity levels, error accumulator) — [environment.md](environment.md)#gap-5-compilererror-expansion
- [x] DisjointSet (union-find with path compression) — [utils.md](utils.md)#gap-1-disjointset
- [x] OrderedMap utility — [utils.md](utils.md)#gap-2-orderedmap

### Phase 2: BuildHIR (OXC AST to HIR)

- [x] HIR lowering: statements (variable declarations, assignments, returns, throw) — [build-hir.md](build-hir.md)#gap-1-statement-lowering
- [x] HIR lowering: expressions (binary, unary, calls, member access, template literals) — [build-hir.md](build-hir.md)#gap-2-expression-lowering
- [x] HIR lowering: control flow (if/else, switch, loops, try/catch, logical, ternary, optional chaining) — [build-hir.md](build-hir.md)#gap-3-control-flow-lowering
- [x] HIR lowering: JSX (elements, fragments, attributes, spread, children) — [build-hir.md](build-hir.md)#gap-4-jsx-lowering
- [x] HIR lowering: functions (declarations, expressions, arrows, closures, async, generators) — [build-hir.md](build-hir.md)#gap-5-function-lowering
- [x] HIR lowering: destructuring (object, array, nested, defaults, rest) — [build-hir.md](build-hir.md)#gap-6-destructuring-lowering
- [x] HIR lowering: patterns (for-of, for-in, spread, computed properties) — [build-hir.md](build-hir.md)#gap-7-pattern-and-iterator-lowering
- [x] Function discovery (find compilable components/hooks from OXC AST) — [build-hir.md](build-hir.md)#gap-8-function-discovery
- [x] Context variable handling (captured variables, closure analysis) — [build-hir.md](build-hir.md)#gap-9-context-variable-handling
- [x] Manual memoization markers (StartMemoize/FinishMemoize for useMemo/useCallback) — [build-hir.md](build-hir.md)#gap-10-manual-memoization-markers

### Phase 3: SSA and Early Optimization

- [x] EnterSSA (phi node insertion, identifier renaming via dominance frontiers) — [ssa.md](ssa.md)#gap-1-enterssa
- [x] EliminateRedundantPhi — [ssa.md](ssa.md)#gap-2-eliminateredundantphi
- [x] ConstantPropagation — [optimization.md](optimization.md)#gap-1-constantpropagation
- [x] InlineIIFE — [optimization.md](optimization.md)#gap-2-inlineiife
- [x] MergeConsecutiveBlocks — [optimization.md](optimization.md)#gap-3-mergeconsecutiveblocks
- [x] DeadCodeElimination — [optimization.md](optimization.md)#gap-4-deadcodeelimination
- [x] PruneMaybeThrows — [optimization.md](optimization.md)#gap-5-prunemaybethrows
- [x] OptimizePropsMethodCalls — [optimization.md](optimization.md)#gap-6-optimizepropsmethodcalls

### Phase 4: Type Inference and Mutation Analysis

- [x] InferTypes (constraint-based type inference with shape system) — [inference.md](inference.md)#gap-1-infertypes
- [x] AliasingEffect enum and composition rules — [inference.md](inference.md)#gap-2-aliasingeffect-enum-and-composition-rules
- [x] InferMutationAliasingEffects (abstract interpretation, fixpoint) — [inference.md](inference.md)#gap-3-infermutationaliasingeffects
- [x] AnalyseFunctions (recursive nested function analysis) — [inference.md](inference.md)#gap-4-analysefunctions
- [x] InferMutationAliasingRanges (MutableRange computation) — [inference.md](inference.md)#gap-5-infermutationaliasingranges

### Phase 5: Reactivity and Scope Inference

- [x] InferReactivePlaces (fixpoint with post-dominator analysis) — [inference.md](inference.md)#gap-6-inferreactiveplaces
- [x] RewriteInstructionKindsBasedOnReassignment — [inference.md](inference.md)#gap-7-rewriteinstructionkindsbasedonreassignment
- [x] InferReactiveScopeVariables (DisjointSet-based scope grouping) — [reactive-scopes.md](reactive-scopes.md)#gap-1-inferreactivescopevariables
- [x] MemoizeFbtAndMacroOperandsInSameScope — [reactive-scopes.md](reactive-scopes.md)#gap-2-memoizefbtandmacrooperandsinsamescope
- [x] AlignMethodCallScopes — [reactive-scopes.md](reactive-scopes.md)#gap-3-alignmethodcallscopes
- [x] AlignObjectMethodScopes — [reactive-scopes.md](reactive-scopes.md)#gap-4-alignobjectmethodscopes
- [x] PruneUnusedLabelsHIR — [reactive-scopes.md](reactive-scopes.md)#gap-5-pruneunusedlabelshir
- [x] AlignReactiveScopesToBlockScopesHIR — [reactive-scopes.md](reactive-scopes.md)#gap-6-alignreactivescopestoblockscopeshir
- [x] MergeOverlappingReactiveScopesHIR — [reactive-scopes.md](reactive-scopes.md)#gap-7-mergeoverlappingreactivescopeshir
- [x] BuildReactiveScopeTerminalsHIR — [reactive-scopes.md](reactive-scopes.md)#gap-8-buildreactivescopeterminalshir
- [x] FlattenReactiveLoopsHIR — [reactive-scopes.md](reactive-scopes.md)#gap-9-flattenreactiveloopshir
- [x] FlattenScopesWithHooksOrUseHIR — [reactive-scopes.md](reactive-scopes.md)#gap-10-flattenscopeswithhooksorusehir
- [x] PropagateScopeDependenciesHIR — [reactive-scopes.md](reactive-scopes.md)#gap-11-propagatescopedependencieshir

### Phase 6: ReactiveFunction and Codegen

- [x] BuildReactiveFunction (HIR CFG to ReactiveFunction tree) — [reactive-scopes.md](reactive-scopes.md)#gap-12-buildreactivefunction
- [x] PruneUnusedLabels (RF) — [reactive-scopes.md](reactive-scopes.md)#gap-13-pruneunusedlabels-rf
- [x] PruneNonEscapingScopes — [reactive-scopes.md](reactive-scopes.md)#gap-14-prunenonescapingscopes
- [x] PruneNonReactiveDependencies — [reactive-scopes.md](reactive-scopes.md)#gap-15-prunenonreactivedependencies
- [x] PruneUnusedScopes — [reactive-scopes.md](reactive-scopes.md)#gap-16-pruneunusedscopes
- [x] MergeReactiveScopesThatInvalidateTogether — [reactive-scopes.md](reactive-scopes.md)#gap-17-mergereactivescopesthatinvalidatetogether
- [x] PruneAlwaysInvalidatingScopes — [reactive-scopes.md](reactive-scopes.md)#gap-18-prunealwaysinvalidatingscopes
- [x] PropagateEarlyReturns — [reactive-scopes.md](reactive-scopes.md)#gap-19-propagateearlyreturns
- [x] PruneUnusedLvalues — [reactive-scopes.md](reactive-scopes.md)#gap-20-pruneunusedlvalues
- [x] PromoteUsedTemporaries — [reactive-scopes.md](reactive-scopes.md)#gap-21-promoteusedtemporaries
- [x] ExtractScopeDeclarationsFromDestructuring — [reactive-scopes.md](reactive-scopes.md)#gap-22-extractscopedeclarationsfromdestructuring
- [x] StabilizeBlockIds — [reactive-scopes.md](reactive-scopes.md)#gap-23-stabilizeblockids
- [x] RenameVariables — [reactive-scopes.md](reactive-scopes.md)#gap-24-renamevariables
- [x] PruneHoistedContexts — [reactive-scopes.md](reactive-scopes.md)#gap-25-prunehoistedcontexts
- [x] CodegenFunction (ReactiveFunction to JavaScript output) — [codegen.md](codegen.md)#gap-1-codegenfunction
- [x] Import insertion (react/compiler-runtime) — [codegen.md](codegen.md)#gap-2-import-insertion
- [x] Source map generation — [codegen.md](codegen.md)#gap-3-source-map-generation

---

## Priority 2 — Validation and Correctness

### Phase 7: Validation Passes

- [x] ValidateContextVariableLValues — [validation.md](validation.md)#gap-1-validatecontextvariablelvalues
- [x] ValidateUseMemo — [validation.md](validation.md)#gap-2-validateusememo
- [x] DropManualMemoization (conditional pass) — [validation.md](validation.md)#gap-3-dropmanualmemoization
- [x] ValidateHooksUsage — [validation.md](validation.md)#gap-4-validatehooksusage
- [x] ValidateNoCapitalizedCalls — [validation.md](validation.md)#gap-5-validatenocapitalizedcalls
- [x] ValidateLocalsNotReassignedAfterRender — [validation.md](validation.md)#gap-6-validatelocalsnotreassignedafterrender
- [x] AssertValidMutableRanges — [validation.md](validation.md)#gap-7-assertvalidmutableranges
- [x] ValidateNoRefAccessInRender — [validation.md](validation.md)#gap-8-validatenorefaccessinrender
- [x] ValidateNoSetStateInRender — [validation.md](validation.md)#gap-9-validatenosetstateinrender
- [x] ValidateNoDerivedComputationsInEffects — [validation.md](validation.md)#gap-10-validatenoderivedcomputationsineffects
- [x] ValidateNoSetStateInEffects — [validation.md](validation.md)#gap-11-validatenosetstateineffects
- [x] ValidateNoJSXInTryStatement — [validation.md](validation.md)#gap-12-validatenojsxintrystatement
- [x] ValidateNoFreezingKnownMutableFunctions — [validation.md](validation.md)#gap-13-validatenofreezingknownmutablefunctions
- [x] ValidateExhaustiveDependencies — [validation.md](validation.md)#gap-14-validateexhaustivedependencies
- [x] ValidateStaticComponents — [validation.md](validation.md)#gap-15-validatestaticcomponents
- [x] ValidatePreservedManualMemoization — [validation.md](validation.md)#gap-16-validatepreservedmanualmemoization

---

## Priority 3 — Integration and Tooling

### Pipeline Orchestration

- [x] Pipeline orchestrator (run all 62 passes in order with config-based gating) — [pipeline.md](pipeline.md)#gap-1-pipeline-orchestrator
- [x] Program-level compilation loop (discover functions, compile each, collect edits) — [pipeline.md](pipeline.md)#gap-2-program-level-compilation-loop
- [x] Lint mode (run pipeline, collect errors, skip codegen) — [pipeline.md](pipeline.md)#gap-3-lint-mode
- [x] OptimizeForSSR pass — [pipeline.md](pipeline.md)#gap-4-optimizeforssr
- [x] OutlineJSX pass — [pipeline.md](pipeline.md)#gap-5-outlinejsx
- [x] NameAnonymousFunctions pass — [pipeline.md](pipeline.md)#gap-6-nameanonymousfunctions
- [x] OutlineFunctions pass — [pipeline.md](pipeline.md)#gap-7-outlinefunctions

### Oxlint Rules (Tier 1 -- standalone AST rules)

- [x] no-jsx-in-try — [lint-rules.md](lint-rules.md)#gap-1-no-jsx-in-try
- [x] use-memo-validation — [lint-rules.md](lint-rules.md)#gap-2-use-memo-validation
- [x] no-capitalized-calls — [lint-rules.md](lint-rules.md)#gap-3-no-capitalized-calls
- [x] purity — [lint-rules.md](lint-rules.md)#gap-4-purity
- [x] incompatible-library — [lint-rules.md](lint-rules.md)#gap-5-incompatible-library
- [x] static-components — [lint-rules.md](lint-rules.md)#gap-6-static-components
- [x] no-set-state-in-render — [lint-rules.md](lint-rules.md)#gap-7-no-set-state-in-render
- [x] no-set-state-in-effects — [lint-rules.md](lint-rules.md)#gap-8-no-set-state-in-effects
- [x] no-ref-access-in-render — [lint-rules.md](lint-rules.md)#gap-9-no-ref-access-in-render
- [x] no-deriving-state-in-effects — [lint-rules.md](lint-rules.md)#gap-10-no-deriving-state-in-effects
- [x] globals — [lint-rules.md](lint-rules.md)#gap-11-globals

---

## Priority 4 — Shipping and Polish

### NAPI and Vite Plugin

- [x] NAPI bindings (async task pattern, TransformResult) — [integration.md](integration.md)#gap-1-napi-bindings
- [ ] Vite plugin (transform hook, file filtering, HMR) — [integration.md](integration.md)#gap-2-vite-plugin
- [x] Configuration parsing (JSON/TOML to PluginOptions) — [integration.md](integration.md)#gap-3-configuration-parsing
- [x] Gating support (feature flag wrapping) — [integration.md](integration.md)#gap-4-gating-support

### Oxlint Rules (Tier 2 -- compiler-dependent)

- [ ] hooks (full Rules of Hooks with HIR analysis) — [lint-rules.md](lint-rules.md)#gap-12-hooks-tier-2
- [ ] immutability (mutation of frozen values) — [lint-rules.md](lint-rules.md)#gap-13-immutability-tier-2
- [ ] preserve-manual-memoization — [lint-rules.md](lint-rules.md)#gap-14-preserve-manual-memoization-tier-2
- [ ] memo-dependencies (exhaustive deps with autofix) — [lint-rules.md](lint-rules.md)#gap-15-memo-dependencies-tier-2
- [ ] exhaustive-effect-deps (with autofix) — [lint-rules.md](lint-rules.md)#gap-16-exhaustive-effect-deps-tier-2

### Testing and Conformance

- [ ] Upstream fixture test harness — [testing.md](testing.md)#gap-1-upstream-fixture-test-harness
- [ ] Per-pass snapshot tests (insta) — [testing.md](testing.md)#gap-2-per-pass-snapshot-tests
- [ ] Comparison tests (Babel vs OXC output diffing) — [testing.md](testing.md)#gap-3-comparison-tests
- [ ] Performance benchmarking — [testing.md](testing.md)#gap-4-performance-benchmarking

### Release

- [ ] Pin upstream React compiler commit (UPSTREAM_VERSION.md) — [integration.md](integration.md)#gap-5-pin-upstream-commit
- [ ] Release packaging (platform-specific NAPI binaries) — [integration.md](integration.md)#gap-6-release-packaging

---

## Blocked

_(Nothing blocked)_
