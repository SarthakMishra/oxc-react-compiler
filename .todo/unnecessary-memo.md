# Unnecessary Memoization (176 fixtures)

133 fixtures where we add `_c()` but upstream returns source unchanged.
43 fixtures where neither memoizes but our output still differs.

## Unnecessary Memoization (133 fixtures)

**Symptom:** We emit `_c(N)` memoization. Upstream returns the input
source unchanged (no memoization, no transforms, or only DCE/const-prop).

**Root causes:**

### 1. Missing DCE / constant propagation (~13 fixtures with obvious names)

Upstream applies dead-code elimination and constant propagation even
when it doesn't memoize. Our zero-scope bail-out returns original source,
but upstream may apply transforms that change the output.

Fixtures: `constant-propagation.js`, `dce-loop.js`, `dce-unused-const.js`,
`constant-propagation-for.js`, `constant-propagation-phi.js`, etc.

**Upstream:** `DeadCodeElimination.ts`, `ConstantPropagation.ts`

**Current state:** We have `dead_code_elimination.rs` and
`constant_propagation.rs` but they may not run when zero-scope bail-out
triggers (we return original source before these transforms apply).

**Fix strategy:** If upstream applies DCE/const-prop even for zero-scope
functions, we need to apply these transforms BEFORE the zero-scope check,
then return the transformed (but non-memoized) source.

### 2. Over-eager scope creation for non-reactive functions (~100+ fixtures)

We create reactive scopes for functions that upstream determines have no
reactive dependencies. This means our scope analysis (reactive place
inference, scope variable inference) is too aggressive.

Common patterns:
- Functions with only destructured parameters that are fully consumed
- Functions that only use module-level imports
- Helper functions with no hooks/state/context

**Partial fix (2026-03-14):** The `infer_reactive_places` parameter-only
seeding fix (see [scope-analysis.md](scope-analysis.md)#over-counting)
narrowed the reactive seed set so non-parameter locals in the entry block
are no longer marked reactive. This reduced over-eager scope creation for
some of these functions (+31 fixtures total).

**Fix strategy:** This overlaps with scope-analysis over-counting.
Further improvements to reactive place inference precision and scope
pruning will continue reducing this category.

### 3. Functions upstream doesn't compile at all

Some fixtures contain functions that upstream skips entirely (not
components or hooks, in Infer mode). If we're using CompilationMode::All,
we may compile functions that upstream wouldn't touch.

**What to check:** Are we correctly honoring `@compilationMode` directives?
Some fixtures may specify `@compilationMode:"infer"` and we may not
properly skip non-component/non-hook functions.

## Both No-Memo, Different Output (43 fixtures)

Both compilers produce non-memoized output but the source differs.
This likely means upstream applies transforms (renaming, simplification)
that we don't.

**Fix strategy:** Lower priority. These require understanding what
non-memoization transforms upstream applies.

**Key files:**
- `crates/oxc_react_compiler/src/optimization/dead_code_elimination.rs`
- `crates/oxc_react_compiler/src/optimization/constant_propagation.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_places.rs`
- `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`
