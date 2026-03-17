# Unnecessary Memoization / Over-Compilation

This category overlaps significantly with "we compile, they don't" (126
fixtures) and scope analysis divergences. Rather than tracking a separate
fixture count, this file documents the root causes of over-compilation.

## Root Causes

### 1. Over-eager scope creation for non-reactive functions

We create reactive scopes for functions that upstream determines have no
reactive dependencies. Partially addressed by parameter-only seeding in
`infer_reactive_places` (Phase 74) and param-ID reactive seeding with
mutable value gate (Phase 80).

Further improvements to reactive place inference precision and scope
pruning will continue reducing this category.

See [scope-analysis.md](scope-analysis.md) for detailed tracking.

### 2. Functions upstream doesn't compile at all

Some fixtures contain functions that upstream skips entirely (not
components or hooks, in Infer mode). If we compile helpers or utility
functions that upstream doesn't touch, we produce memoized output for
functions that should pass through unchanged.

See [upstream-errors.md](upstream-errors.md) for detailed tracking.

### 3. Missing DCE / constant propagation

Upstream applies dead-code elimination and constant propagation even
when it doesn't memoize. Our zero-scope bail-out returns original source,
but upstream may apply transforms that change the output.

**Key files:**
- `crates/oxc_react_compiler/src/optimization/dead_code_elimination.rs`
- `crates/oxc_react_compiler/src/optimization/constant_propagation.rs`
- `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`
