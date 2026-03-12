# Critical Bugs

> These bugs block ALL memoization correctness and render equivalence.
> Fixing these two bugs should bring the correctness score from 0.000 to a significant positive number.

Last updated: 2026-03-12

---

## Bug 1: Destructured parameters not emitted

**Symptom:** ALL 16 compiled fixtures fail to render. Error: `"X is not defined"` where X is a destructured prop (e.g., `status`, `count`, `users`).

**Render equivalence score:** 0.000 (0/25 render pairs match)

**Root cause:** `hir/build.rs` line ~455 in `lower_formal_params()` handles destructured parameters by creating a temp place (`t0`, `t1`) but never emits a `Destructure` instruction to extract the named bindings. The compiled output references variable names (e.g., `status`) that were never declared.

**Example:**
```
// Input
export function StatusBadge({ status }) { ... }

// OXC output (BROKEN)
export function StatusBadge(t0) {
  const t26 = status;  // ERROR: status is not defined
}

// Babel output (CORRECT)
export function StatusBadge(t0) {
  const { status } = t0;  // Destructure emitted
  ...
}
```

**Fix locations:**
1. `crates/oxc_react_compiler/src/hir/build.rs` (`lower_formal_params`): After creating the temp for a destructured param, emit a `Destructure` instruction that extracts the named bindings from the temp
2. `crates/oxc_react_compiler/src/hir/types.rs` (`Param` enum): May need to store the original destructure pattern alongside the temp Place, or rely on emitted instructions
3. Verify `extract_scope_declarations_from_destructuring` in `prune_scopes.rs` handles these correctly downstream

**Depends on:** Nothing. This is a standalone fix.

**Impact:** Fixes the "X is not defined" runtime error for ALL compiled components. Without this, no compiled component can render.

---

## Bug 2: Dependency filter drops all scope dependencies

**Symptom:** ALL 16 fixtures use sentinel checks (`$[N] === Symbol.for('react.memo_cache_sentinel')`) but ZERO use dependency checks (`$[N] !== dep`). This means every memoized scope runs only once (on first render) and never invalidates.

**Correctness score:** 0/16 fixtures fully memoized

**Root cause:** `reactive_scopes/propagate_dependencies.rs` line 136:

```rust
scope.dependencies = deps.into_iter().filter(|d| d.reactive).collect();
```

This filters dependencies to only include those where `place.reactive == true`. However, the `Place` objects collected as operands have their OWN `.reactive` field that is separate from the `Identifier`'s reactivity marking done in `infer_reactive_places` (Pass 29). The Place copies used during dependency collection don't reflect the identifier's reactivity status, so the filter removes all dependencies.

**Codegen consequence:** When `codegen.rs` checks `if deps.is_empty()` (line ~379), it sees empty and generates sentinel-only checks instead of dependency comparisons.

**Evidence comparison:**
```
// OXC codegen (BROKEN — sentinel only, runs once)
if ($[0] === Symbol.for("react.memo_cache_sentinel")) {
  $[0] = <computed value>;
}
const result = $[0];

// Babel codegen (CORRECT — checks dependency, re-runs when dep changes)
if ($[0] !== users || $[1] !== max) {
  $[0] = users;
  $[1] = max;
  $[2] = <computed value>;
}
const result = $[2];
```

**Fix options (pick one):**
1. **Remove the reactive filter entirely** — all external operands used inside a scope are dependencies, regardless of reactivity marking. This is the simplest fix and may be semantically correct (the codegen should emit deps for all external references).
2. **Propagate identifier reactivity to Place** — when collecting operands in Phase 2 of `propagate_dependencies`, look up the identifier's reactivity from the declaration site rather than reading `place.reactive` from the use site.
3. **Use identifier-level reactivity** — change `ReactiveScopeDependency` to store the identifier and check `identifier.reactive` instead of `place.reactive` during filtering.

**Recommended fix:** Option 1 (remove the filter). The upstream compiler tracks dependencies as "all external values read inside the scope" — reactivity is used to decide which SCOPES to create, not which DEPENDENCIES to track within a scope.

**Depends on:** Nothing. This is a standalone fix.

**Impact:** Enables proper cache invalidation. Combined with Bug 1, this should make most fixtures render correctly with working memoization.

---

## Bug 3: canvas-sidebar performance outlier

**Symptom:** The `canvas-sidebar` fixture (272 LOC) takes 16.6ms to compile, which is 10x the expected time based on LOC scaling. For comparison, `multi-step-form` (284 LOC) takes 1.4ms and `booking-list` (152 LOC) takes 1.0ms.

**Likely cause:** Quadratic or exponential behavior in one of the analysis passes, likely triggered by:
- Deep nesting (sidebar has tabs, layers, search, property editing — many nested scopes)
- Large number of reactive scope candidates (9 hooks, many derived values)
- Possible pathological case in `infer_mutation_aliasing_effects` (fixpoint iteration) or `infer_reactive_scope_variables` (union-find with many unions)

**Investigation needed:**
1. Profile the compilation with `transformReactFileTimed` to isolate which pass is slow
2. Add per-pass timing to `pipeline.rs` (behind a feature flag)
3. Check if `canvas-sidebar.tsx` has unusual patterns (deeply nested callbacks, many closures, large switch statements)

**Depends on:** Diagnostic infrastructure (per-pass timing)

**Impact:** Performance. Not blocking correctness but indicates a potential scalability issue that would affect larger real-world components.

---

## Priority Order

1. **Bug 1** (destructured params) — fixes render equivalence, unblocks all E2E testing
2. **Bug 2** (dependency filter) — fixes memoization correctness, enables proper cache invalidation
3. **Bug 3** (performance outlier) — investigation only, lower priority

Fixing bugs 1 + 2 together should bring:
- Render equivalence from 0.000 to a significant positive score
- Correctness from 0/16 to most fixtures passing
- Babel structural comparison score improvement
