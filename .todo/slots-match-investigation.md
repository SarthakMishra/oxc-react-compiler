# Stage 1a: "Slots MATCH" Investigation Results

> Completed: 2026-03-25
> Pool: 239 fixtures where both compile, same slot count, different structure

## Sub-Pattern Breakdown

| Pattern | Count | % | Fixable? | Est. Fixtures Fixed |
|---------|-------|---|----------|---------------------|
| B: Variable naming | 126 | 52.7% | YES | 40-80 |
| A: Instruction ordering | 55 | 23.0% | PARTIAL | 15-30 |
| C: Assignment/structure | 14 | 5.9% | MIXED | 5-10 |
| Other (scope boundary, JSX, etc.) | 44 | 18.4% | HARD | 5-15 |

## Pattern B: Variable Naming (126 fixtures, 52.7%)

### B1: High-numbered temp variables (dominant)

**Problem:** We emit temp variables with HIR instruction IDs (`t5`, `t6`, `t7`, `t8`, `t10`, `t11`) instead of renumbering from `t0` per function like upstream does.

**Example (rules-of-hooks):**
```
// Ours:   let t8; useHook(); if ($[0] !== props ...
// Theirs: useHook(); let t0; if ($[0] !== props ...
```

**Example (simple.js):**
```
// Ours:   let t7; let t11;
// Theirs: (declared inside scope) let t0; ... let t1;
```

**Root cause:** `codegen.rs` uses `InstructionId` as the temp var name suffix. Upstream's `ReactiveFunction` codegen renumbers temps sequentially from 0.

**Fix complexity:** MEDIUM — Need a temp counter in codegen that allocates fresh t0, t1, t2... instead of using instruction IDs. Must handle nested scopes correctly.

**Estimated impact:** Directly fixes naming in 48+ fixtures. Combined with other improvements, could help 60-80.

### B2: Temps where upstream uses original names (40 fixtures — PARTIALLY TRACTABLE, SCOPE-INFERENCE DEPENDENT)

**Problem:** We assign to a temp variable and then assign to the original; upstream assigns directly to the original.

**Extended investigation (2026-03-25):** This is the SINGLE LARGEST TRACTABLE sub-pattern in the entire 237-fixture slots-MATCH pool. Revised count: 40 fixtures (up from initial 34 estimate). These fixtures have matching slot counts and would pass if we preserved original variable names in scope outputs instead of using temps.

**Revised finding (2026-03-26): B2 is scope-inference dependent, NOT codegen-only.** Investigation revealed that many B2 fixtures also have scope boundary differences driven by scope inference. Changing which variable name is used for scope outputs (temp vs original) is a codegen change, but it does not pass conformance if the scope itself has different boundaries than upstream. B2 is therefore only partially addressable by codegen; the remainder requires scope inference improvements (Stage 3). This downgrades B2 from "largest tractable codegen fix" to "partially tractable, scope-dependent."

**Example (type-test-field-store.js):**
```
// Ours:   let t10; ... t10 = x.t; ... let z = t10;
// Theirs: let x; ... x = {...}; ... const z = x.t;
```

**Root cause:** Our codegen doesn't preserve original variable names for scope outputs when possible. Upstream tries to reuse the original declaration name. However, the scope boundaries themselves often differ, making name changes alone insufficient.

**Fix complexity:** HIGH — Requires both codegen changes (scope output naming) AND scope inference improvements (scope boundary accuracy). The codegen-only portion may fix a subset of the 40 fixtures, but the majority also need scope inference fixes.

### B3: Original names where upstream uses temps (23 fixtures)

Opposite of B2. Less common, lower priority.

### B4: Different original names or temp numbering (10 fixtures)

Edge cases with `t0` vs `t1` numbering within scopes, different `$` conflict resolution, etc.

## Pattern A: Instruction Ordering (55 fixtures, 23.0%)

### A1: Variable declarations at function level instead of inside control flow (primary)

**Problem:** Our `collect_all_scope_declarations` pre-declares ALL scope output variables at function level. Upstream declares them inside the relevant control flow block.

**Example (simple.js):**
```
// Ours:   let t7; let t11; if (x) { if ($[0] !== y) { t7 = ... } }
// Theirs: if (x) { let t0; if ($[0] !== y) { t0 = ... } }
```

**Root cause:** `collect_all_scope_declarations` in `codegen.rs` (documented as load-bearing in .todo/index.md). Removing it causes render collapse 96%→24%.

**Fix complexity:** HIGH — Cannot simply remove. Need to change declaration placement strategy to emit declarations at narrowest possible scope while maintaining correctness.

### A2: Hook calls after variable declarations

**Problem:** We emit `let t8; useHook();` but upstream emits `useHook(); let t0;`.

**Root cause:** All scope output declarations are emitted before any instructions.

**Fix complexity:** MEDIUM — Related to A1; fixing declaration placement would fix this too.

### A3: Try/catch mishandling

**Problem:** Some fixtures show try/catch being dropped or restructured incorrectly.

**Root cause:** Try/catch lowering in HIR or codegen. Separate issue from naming/ordering.

**Fix complexity:** HIGH — requires investigation of try-catch codegen path.

## Pattern C: Structural Differences (58 fixtures combined)

- **C1: Scope output variable choice** — We cache a derived value instead of the original object (5-10 fixtures)
- ~~**C2: Extra `return undefined`** in function expressions (affects ~10 fixtures, simple codegen fix)~~ ✅ **FIXED:** +5 fixtures
- **C3: Function outlining** — Upstream outlines certain lambdas to `_temp` at module scope (5 fixtures, not implemented)
- **C4: `$` conflict resolution** — Different strategy for conflicting dollar-sign variables (1 fixture)
- ~~**C5: Catch clause handling** — We emit `catch (e)` where upstream emits `catch {}` (2-3 fixtures)~~ ✅ **FIXED:** +0 net (blocked by A1 ordering)

## Recommended Implementation Order

### Stage 1b: Temp Variable Renumbering (B1) -- COMPLETE

~~**Estimated gain:** +25-40 fixtures~~

**Completed 2026-03-25. Actual gain: +2 fixtures (403→405).**

**What was implemented:**
- `renumber_temps_in_output` in `codegen.rs` — two-pass atomic temp renumbering (pass 1: collect all `tN` identifiers, pass 2: replace with sequential `t0`, `t1`, `t2`...)
- Fixed `is_temp_place` to use pattern matching instead of ID-based check
- Fixed Unicode safety in `replace_identifier_in_output` (char-boundary-safe slicing)
- Fixed cascade replacement bug with atomic two-pass approach (avoids `t1`→`t0` then `t10`→`t00` problem)
- Added `$` to word boundary detection for JS identifier matching

**Fixtures gained:** `gating/multi-arrow-expr-export-gating-test.js`, `gating/multi-arrow-expr-gating-test.js`

**Why the estimate was wrong:** The B1 sub-pattern (126 fixtures, "high-numbered temp variables") was counted by examining naming differences alone. In reality, most of those 126 fixtures ALSO differ in instruction ordering (where declarations appear) or scope output name preservation (B2). Renumbering temps to sequential names is necessary but not sufficient — the declarations also need to be in the right place, and scope outputs need to use original variable names where upstream does. Only 2 fixtures had temp numbering as their sole remaining difference.

### Stage 1c: Minor Codegen Fixes -- COMPLETE (+5 net, 405→410)

- ~~**C2: Remove extra `return undefined`** — +5-10 fixtures, trivial fix~~ **Completed:** +5 fixtures (capturing-func-mutate-nested.js, capturing-function-decl.js, hoisting-recursive-call.ts, mutate-captured-arg-separately.js, reassign-object-in-context.js). Codegen now omits trailing `return undefined` in function expressions.
- ~~**C5: Empty catch clause** — +2-3 fixtures~~ **Completed:** +0 net fixtures. Catch clause now emits `catch {}` instead of `catch (e)` when the parameter is unused, matching upstream. However, all catch-clause fixtures are also blocked by A1 (instruction ordering — declarations at function level instead of inside control flow), so improved catch output alone is not sufficient to pass.
- **B4 edge cases** — SKIPPED. Only 1 fixture affected and high implementation complexity. Not worth pursuing.

### Stage 1d: Lazy Scope Declaration Placement (A1+A2)

#### Phase 1: Move declarations to just-before-scope-guard ✅

~~**Status:** NOT STARTED — recommended as next task (2026-03-25)~~

**Completed (2026-03-25):** Implemented lazy scope declaration placement in `codegen.rs`. Instead of `collect_all_scope_declarations` emitting ALL scope output `let` declarations at function top, declarations are now emitted just before the scope guard that needs them. This places declarations after hook calls and before the relevant scope block, matching upstream behavior for the rules-of-hooks pattern.

**Result:** +6 fixtures (444 -> 450). Gained 6 fixtures (exceeded the +5 estimate). Same-slots-different-structure pool reduced from 233 to 227.

**Risk was LOW as predicted.** No regressions. All unit tests pass. Render rate and E2E rate unchanged.

#### Phase 2 (MEDIUM risk, +10-20): Move declarations inside control flow — BLOCKED BY SCOPE INFERENCE

- For scopes inside if/for/try blocks, emit declarations at the start of the control flow block.
- This is where the risk increases — must ensure no "already declared" errors or "assignment to const" issues.
- **Verify:** run full conformance after each change, check for regressions.
- **Impact analysis:** 39 fixtures have A1 (declaration-before-control-flow) as first diff. Total same-slots pool now 227 fixtures.

**Finding (2026-03-26): This is a scope inference issue, not codegen.** Investigation revealed that declarations cannot move inside control flow blocks purely via codegen changes. The prerequisite is that scope inference itself must produce scopes that are bounded by the control flow block. Currently, scope inference merges scopes across control flow boundaries (e.g., a scope that starts before an `if` and ends inside it). The codegen cannot place a declaration inside the `if` block if the scope that owns the declaration spans across the `if` boundary. This makes Phase 2 dependent on Stage 3 (scope inference improvements). The slots-MATCH pool is therefore dominated by scope inference accuracy, not codegen formatting.

**Status:** BLOCKED until scope inference improvements (Stage 3) can produce control-flow-scoped scopes.

#### Phase 3 (HIGH risk, +5-10): Merge declaration with initialization — COMPLETE (+0, dormant)

**Completed 2026-03-26.** Implemented merging of `let t0; t0 = expr;` into `let t0 = expr;` when the declaration and first assignment are adjacent. The merged form is emitted correctly.

**Result: +0 fixtures.** All affected fixtures also differ in other ways (scope inference, naming). The improvement is dormant — it produces correct output for the merge pattern, but no fixture has this as its sole remaining difference. The benefit will materialize once scope inference improvements (Stage 3) resolve the other differences in these fixtures.

~~- Instead of `let t0; ... t0 = expr;`, emit `let t0 = expr;` when possible.~~
~~- Pattern: `const q ;` → `const q =` (see `capture-indirect-mutate-alias.js`, `capture_mutate-across-fns.js`)~~
~~- Only merge when the declaration and first assignment are adjacent.~~

**Key files:**
- `codegen.rs` — `collect_all_scope_declarations` and lazy declaration placement logic
- `codegen.rs` — `codegen_scope` where scope guards are emitted

**Safety checks after each phase:**
1. `cargo test` — all unit tests pass
2. Conformance count does not decrease (no regressions)
3. Render rate stays at 92%+ (23/25)
4. E2E rate stays at 95-100%

### Revised Estimates After Stage 1b

**Stage 1b delivered +2 (not +25-40).** The key insight: naming and ordering are entangled. A fixture that differs in temp naming almost always also differs in declaration placement or instruction ordering. Fixing one without the other does not pass conformance.

**Revised total from Stage 1:** +2 (1b) + 5 (1c) + 6 (1d Phase 1) + TBD (1d Phases 2-3) = +13 so far, +10-30 remaining from Phases 2-3.

**Stage 1d Phase 1 COMPLETE (2026-03-25):** +6 fixtures gained (exceeded +5 estimate). Same-slots pool reduced 233 -> 227. Phases 2-3 remain for broader A1 pattern (39 fixtures with declaration-before-control-flow as first diff).

**Stage 1e COMPLETE (2026-03-26):** +6 fixtures gained (441->447). Dynamic gating parsing +3 (harness fix), empty catch handler +1, ObjectExpression computed key bail-out +2, const/let codegen +0.

**Stage 1f COMPLETE (2026-03-26):** +5 fixtures gained (447->452). Known-incompatible bail re-enabled +3, ESLint suppression bail +1, object property key quoting +1. Also: Phase 3 (merge decl+init) implemented +0 dormant, B2 found to be scope-inference dependent (not codegen-only).

### Key Finding (2026-03-26): Slots-MATCH Pool Dominated by Scope Inference

After completing all tractable codegen fixes (Stages 1b through 1e), the remaining 227-fixture slots-MATCH pool is dominated by **scope inference differences**, not codegen formatting issues. The major remaining patterns all trace back to scope inference:

1. **Declaration placement (A1, 39 fixtures):** Phase 2 requires scope inference to produce control-flow-scoped scopes (BLOCKED, see Phase 2 section above).
2. **Variable name preservation (B2, 40 fixtures):** Partially a scope output assignment issue, but many B2 fixtures also have scope boundary differences driven by scope inference.
3. **Instruction ordering within scopes:** The order of instructions inside scope guards depends on which instructions scope inference groups together.

**Implication:** Further slots-MATCH gains require Stage 3 (scope inference improvements), not more codegen fixes. The codegen-only approach has been exhausted for this pool. B2 (variable name preservation) was previously thought to be the last remaining codegen-addressable sub-pattern, but investigation (2026-03-26) found it too is scope-inference dependent — many B2 fixtures have scope boundary differences that prevent passing even with correct variable names. Phase 3 (merge decl+init) was also implemented and gained +0 (dormant until scope inference fixes). The codegen-only ceiling has been reached.
