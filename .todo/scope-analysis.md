# Scope/Dependency Analysis Divergences (584 fixtures)

Both compilers produce `_c()` output but with different slot counts.
Current breakdown (at 415/1717):
- our_slots - expected = -1: 125 fixtures
- our_slots - expected = +1: 103 fixtures
- our_slots - expected = +2: 48 fixtures
- Other magnitudes: 308 fixtures

## Over-Counting (~459 fixtures where our_slots > expected)

**Symptom:** Our `_c(N)` has a larger N than upstream's.

**Root causes (in order of likely impact):**

### 1. Extra reactive scopes for non-reactive expressions

We create reactive scopes for expressions that upstream determines are
non-reactive. Each extra scope adds at least 2 slots (1 dep + 1 declaration).

**Partial fixes applied:**
- Parameter-only seeding in `infer_reactive_places` (Phase 74): only seed
  `DeclareLocal` for params, not all entry-block locals (+31 fixtures)
- Param-ID reactive seeding with mutable value gate (Phase 80)
- Destructured param exclusion from reactive scope union (Phase 85)

**Remaining issues:**
- Are we still marking too many identifiers as reactive through fixpoint propagation?
- Is `prune_scopes.rs` correctly pruning non-escaping and always-invalidating scopes?
- Are we creating scopes for values that don't escape the function?

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_places.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`

### 2. Scope merging not aggressive enough

Upstream merges scopes that invalidate together. If our merge pass misses
valid merges, we have more scopes = more slots. The +1 over-count fixtures
(103) are strong candidates for merge-related fixes.

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`

### 3. Extra dependencies within scopes

A scope with the right declarations but too many deps will over-count.

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs`

### 4. Missing scope pruning for primitive-only scopes

Upstream prunes scopes that only produce primitive values.

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`

## Under-Counting (~125 fixtures where our_slots < expected)

**Symptom:** Our `_c(N)` has a smaller N than upstream's.

The -1 slot category (125 fixtures) is the most tractable. Under-counting
means we miss scopes or miss declarations. This overlaps with silent
bail-outs (66 fixtures producing 0 scopes).

**Root causes:**
1. Missing scopes for independently-memoizable sub-expressions
2. Missing declarations within scopes
3. Scope over-merging

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`

## Diagnostic Approach

For both over- and under-counting:
1. Pick a small fixture with a small slot difference (+1 or -1)
2. Run our compiler with debug output showing scope structure
3. Run upstream Babel on the same fixture
4. Compare scope-by-scope: which scopes differ, why?
5. Trace back to the pass that created/merged/pruned the differing scope
