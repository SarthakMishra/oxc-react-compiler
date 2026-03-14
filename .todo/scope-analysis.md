# Scope/Dependency Analysis Divergences (636 fixtures)

Both compilers produce `_c()` output but with different slot counts.
474 over-count (we have more slots than upstream), 162 under-count.

Slot count = number of reactive scopes x (deps + declarations per scope).
Over-counting means we create too many scopes or include too many deps.
Under-counting means we miss scopes or skip declarations.

## Over-Counting (474 fixtures)

**Symptom:** Our `_c(N)` has a larger N than upstream's.

**Distribution of over-count magnitude:**
- +1 slot: 97 fixtures
- +2 slots: 98 fixtures
- +3 slots: 62 fixtures
- +4 slots: 70 fixtures
- +5 slots: 47 fixtures
- +6 to +11: 100 fixtures

**Root causes (in order of likely impact):**

### 1. Extra reactive scopes for non-reactive expressions

We create reactive scopes for expressions that upstream determines are
non-reactive (no state/props/context dependency). Each extra scope adds
at least 2 slots (1 dep + 1 declaration).

**Upstream:** `InferReactivePlaces.ts` + `PruneNonReactivePlaces.ts` +
`PruneNonEscapingScopes.ts` + `PruneAlwaysInvalidatingScopes.ts`

**Partial fix (2026-03-14):** `infer_reactive_places` parameter-only seeding.
Previously the entry-block loop marked ALL `DeclareLocal` instructions as
reactive, including temporaries and non-parameter locals. Fixed to accept
`param_names: &[String]` and only seed `DeclareLocal` instructions whose
name appears in `param_names`, matching upstream `InferReactivePlaces.ts`
which seeds from `fn.params`. Result: +31 fixtures (399 to 430/1717).
Files: `infer_reactive_places.rs`, `pipeline.rs`, `pass_unit_tests.rs`.

**Remaining issues:**
- Are we still marking too many identifiers as reactive through fixpoint propagation?
- Is `prune_scopes.rs` correctly pruning non-escaping and always-invalidating scopes?
- Are we creating scopes for values that don't escape the function?

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_places.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`

### 2. Scope merging not aggressive enough

Upstream merges scopes that invalidate together (same deps or
output-to-input chains). If our merge pass misses valid merges,
we have more scopes = more slots.

**Upstream:** `MergeReactiveScopesThatInvalidateTogether.ts`

**Status:** Sub-tasks 4a-4f are complete, but the merge logic may still
differ from upstream in edge cases. The +1/+2 over-count fixtures are
likely candidates for merge-related fixes.

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`

### 3. Extra dependencies within scopes

A scope with the right declarations but too many deps will over-count.
This happens when we include dependencies that upstream excludes
(e.g., stable values that slip through our non-reactive filter).

**What to check:**
- Are we including deps for values that are already declared by a parent scope?
- Are we including deps for module-level constants that should be free variables?
- Are we including deps for stable hook returns that weren't correctly typed?

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs`

### 4. Missing scope pruning for primitive-only scopes

Upstream prunes scopes that only produce primitive values (numbers,
strings, booleans) because primitives are compared by value, not
reference -- caching them provides no benefit.

**Upstream:** `PruneNonEscapingScopes.ts` (checks if scope declarations
are all primitives)

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`

## Under-Counting (162 fixtures)

**Symptom:** Our `_c(N)` has a smaller N than upstream's.

**Distribution of under-count magnitude:**
- -1 slot: 62 fixtures
- -2 slots: 45 fixtures
- -3 slots: 17 fixtures
- -4 to -10: 38 fixtures

**Root causes (in order of likely impact):**

### 1. Missing scopes for independently-memoizable sub-expressions

Upstream creates separate scopes for sub-expressions that can be
memoized independently (e.g., a JSX element inside a function that
also returns another JSX element). We may merge these into one scope
or not create a scope at all.

**Upstream:** `InferReactiveScopeVariables.ts` + scope construction

### 2. Missing declarations within scopes

A scope with the right deps but missing declarations will under-count.
This happens when we fail to register a value as a scope declaration
(it gets emitted outside the scope guard instead of inside).

**What to check:**
- Are all values produced inside a scope registered as declarations?
- Are we failing to track some StoreLocal targets as scope outputs?

### 3. Scope over-merging

Our overlap detection (Pass 42) or invalidate-together merge may be
merging scopes that should remain separate, reducing total scope count.

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`

## Diagnostic Approach

For both over- and under-counting, the diagnostic approach is:

1. Pick a small fixture with a small slot difference (+1 or -1)
2. Run our compiler with debug output showing scope structure
3. Run upstream Babel on the same fixture
4. Compare scope-by-scope: which scopes differ, why?
5. Trace back to the pass that created/merged/pruned the differing scope

The +1/-1 fixtures are the best starting points because the structural
difference is small enough to diagnose precisely.
