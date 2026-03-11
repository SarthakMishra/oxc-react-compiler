# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.
> Items are ordered by dependency: nothing should be blocked by an item below it.

Last updated: 2026-03-11

---

## Critical Path: End-to-End Compilation

These items must be completed in order for the compiler to transform ANY code.
They form a strict dependency chain.

- [x] Wire compile_program to call BuildHIR for discovered functions -- [pipeline.md](pipeline.md)#gap-1-wire-compile-program-to-buildhir
- [x] Wire pipeline passes 29-46 (reactive scope construction) -- [pipeline.md](pipeline.md)#gap-2-wire-reactive-scope-construction-passes-29-46
- [x] Wire pipeline passes 47-60 (build RF + RF optimization) -- [pipeline.md](pipeline.md)#gap-3-wire-build-reactive-function-and-rf-optimization-passes-47-60
- [x] Wire pipeline pass 62 (codegen) and apply edits in compile_program -- [pipeline.md](pipeline.md)#gap-4-wire-codegen-and-source-replacement
- [x] Fix build_reactive_scope_terminals_hir to actually split blocks at scope boundaries -- [reactive-scopes.md](reactive-scopes.md)#gap-1-build-reactive-scope-terminals-hir-is-a-stub

---

## Priority 1: Correctness (affects memoization output)

These passes exist but are incomplete. Incorrect behavior here means wrong memoization decisions.

- [x] Complete infer_mutation_aliasing_effects phases 2-3 (abstract heap, fixpoint) -- [inference.md](inference.md)#gap-1-infer-mutation-aliasing-effects-phases-2-3
- [x] Complete infer_mutation_aliasing_ranges (transitive tracking) -- [inference.md](inference.md)#gap-2-infer-mutation-aliasing-ranges-transitive-tracking
- [x] Wire validate_hooks_usage and validate_no_capitalized_calls into pipeline -- [pipeline.md](pipeline.md)#gap-5-wire-validation-passes-12-13
- [x] Wire validate_exhaustive_dependencies into pipeline (pass 30) -- [pipeline.md](pipeline.md)#gap-6-wire-validate-exhaustive-dependencies
- [x] Wire validate_locals_not_reassigned_after_render into pipeline (pass 21) -- [pipeline.md](pipeline.md)#gap-7-wire-validate-locals-not-reassigned-after-render
- [x] Wire validate_preserved_manual_memoization into pipeline (pass 61) -- [pipeline.md](pipeline.md)#gap-8-wire-validate-preserved-manual-memoization

---

## Priority 2: Optimization Passes (no-op stubs)

These are real optimization passes that are currently no-ops. They affect output quality
but not correctness. The compiler will work without them.

- [x] Implement inline_iife -- [optimization.md](optimization.md)#gap-1-inline-iife
- [x] Implement optimize_props_method_calls -- [optimization.md](optimization.md)#gap-2-optimize-props-method-calls
- [x] Implement outline_jsx -- [optimization.md](optimization.md)#gap-3-outline-jsx
- [x] Implement outline_functions -- [optimization.md](optimization.md)#gap-4-outline-functions
- [x] Implement optimize_for_ssr -- [optimization.md](optimization.md)#gap-5-optimize-for-ssr

---

## Priority 3: Testing Infrastructure

- [x] Build fixture test harness comparing against upstream compiler snapshots -- [testing.md](testing.md)#gap-1-upstream-fixture-test-harness
- [x] Add end-to-end transformation snapshot tests -- [testing.md](testing.md)#gap-2-end-to-end-snapshot-tests
- [x] Add per-pass unit tests for inference and reactive scope passes -- [testing.md](testing.md)#gap-3-per-pass-unit-tests

---

## Priority 4: Integration and Polish

- [x] Implement Tier 2 lint rules (currently all return empty vecs) -- [lint.md](lint.md)#gap-1-tier-2-lint-rules
- [x] Add source map generation to codegen -- [codegen.md](codegen.md)#gap-1-source-map-generation
- [x] Vite plugin depends on pipeline working -- no code changes needed, just pipeline completion

---

## Active Work

_(Nothing in progress)_

---

## Blocked

_(Nothing blocked)_
