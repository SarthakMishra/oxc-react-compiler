# Roadmap: 415 to 600+ Conformance

> Target: +185 fixtures minimum (415/1717 -> 600/1717, 35%)
>
> Current failure breakdown (1302 total):
> - We bail, they compile: 221
> - We compile, they don't: 126
> - Both compile, slots MATCH: 270
> - Both compile, slots DIFFER: 584
> - Both no memo (format diff): 94
> - Skipped (Flow): 38

---

## Executive Summary

The 600 target requires +185 fixtures. The highest-yield paths are:
(1) codegen fixes for the 270 "slots match" fixtures (closest to passing),
(2) eliminating remaining false bail-outs (221 recoverable),
(3) fixing slot count divergences via scope analysis improvements.

---

## Stream 1: Eliminate Remaining False Bail-Outs (+80-120 fixtures)

The 221 "we bail, they compile" category breaks down as follows:

### 1A: Silent bail-outs / 0 scopes produced (66 fixtures)

**What's happening:** We compile successfully but produce 0 reactive
scopes, so we return source unchanged.

**Improvements already made:**
- Param-ID reactive seeding with mutable value gate (94 -> 70)
- Hook hoisting via scope splitting (70 -> 66)

**Root cause:** The `is_mutable_instruction` gate in
`infer_reactive_scope_variables.rs` is still too restrictive for some
instruction patterns.

**Estimated yield:** 20-30 fixtures

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`
- `crates/oxc_react_compiler/src/entrypoint/program.rs`

### 1B: Frozen-mutation false positives (44 fixtures)

**Improvements already made (158 -> 44):**
- Phase 77: Hybrid effects+instruction checker rewrite
- Method signature allowlist
- Ref value exclusion
- Call-conditional exclusion in inner frozen check

**Fix strategy:** Wire mutable range data from `infer_mutation_aliasing_ranges.rs`
into the validator for range-based freeze determination.

**Estimated yield:** 20-30 fixtures

**Key files:**
- `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs`
- `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`

### 1C: Preserve-existing-memoization validation (54 fixtures)

**This is now the largest single bail-out category.** The validator
checks that compiler-generated memoization preserves manually-written
useMemo/useCallback. When our scope analysis differs from upstream's,
the validator rejects.

**Fix strategy:**
1. Audit the validator against `ValidatePreservedManualMemoization.ts`
2. Some fixtures will fix as scope analysis improves
3. Consider relaxing to accept "superset" memoization

**Estimated yield:** 20-30 fixtures

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_preserved_manual_memoization.rs`

### 1D: Minor validator fixes (~52 fixtures combined)

| Validator | Fixtures | Status |
|---|---|---|
| locals-reassigned-after-render | ~26 | Partially fixed (30 -> 26) |
| ref-access-during-render | 13 | Partially fixed (18 -> 13) |
| global-reassignment | 8 | Partially fixed (15 -> 8) |
| hooks-as-values | 3 | Not started |
| setState-during-render | 2 | Mostly fixed (14 -> 2) |

**Estimated yield:** 15-25 fixtures

---

## Stream 2: Codegen Structure Fixes (+60-100 fixtures)

The 270 "both compile, slots MATCH" fixtures have correct scope
structure but wrong code generation within scopes. These are the
closest-to-passing fixtures and represent the highest yield per fix.

### 2A: Temp variable inlining improvements

**Improvements already made:**
- rename_variables pass for scope output temp naming
- Cross-scope LoadLocal inlining for named variables
- Post-SSA LoadLocal temp inlining pass (+35 conformance)

**Remaining patterns:**
- Property chain collapse (`t0 = a; t1 = t0.b` -> `a.b`)
- Temps in scope dependency positions
- Destructuring pattern temps

**Estimated yield:** 40-60 fixtures

### 2B: Scope declaration and dependency ordering

Ordering of `$[N] = value` assignments and dependency checks may differ.

**Estimated yield:** 10-20 fixtures

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`

---

## Stream 3: Fix Slot Count Divergences (+30-50 fixtures)

The 584 "both compile, slots DIFFER" category. Focus on +1 (103) and
-1 (125) fixtures as they have the smallest structural difference.

**Key sub-causes:**
- Extra reactive scopes for non-reactive expressions
- Missing scope merges
- Extra dependencies within scopes
- Missing scope pruning for primitive-only scopes

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/merge_scopes.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/propagate_dependencies.rs`
- `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`

---

## Stream 4: Reduce "We Compile, They Don't" (126 fixtures)

Upstream rejects with an error but we compile. Each fix requires matching
an upstream validation. See [upstream-errors.md](upstream-errors.md).

**Estimated yield:** 15-25 fixtures

---

## Stream 5: "Both No Memo" Format Differences (94 fixtures)

Both compilers produce non-memoized output but the source text differs.

**Estimated yield:** 10-20 fixtures

---

## Prioritized Execution Order

| Priority | Stream | Estimated Yield | Cumulative |
|---|---|---|---|
| 1 | 2A: Temp inlining codegen | 40-60 | 455-475 |
| 2 | 1C: Preserve-memo validator | 20-30 | 475-505 |
| 3 | 1B: Frozen-mutation mutable ranges | 20-30 | 495-535 |
| 4 | 1D: Minor validator fixes | 15-25 | 510-560 |
| 5 | 1A: Silent bail-outs | 20-30 | 530-590 |
| 6 | 3: Scope slot divergences | 30-50 | 560-640 |
| 7 | 2B: Scope ordering | 10-20 | 570-660 |
| 8 | 4: Upstream error matching | 15-25 | 585-685 |
| 9 | 5: Format differences | 10-20 | 595-705 |

**Conservative path to 600:** Priorities 1-6 yield a conservative
+145-225 fixtures, reaching 560-640. The lower bound requires hitting
Stream 3 as well; the upper bound reaches 600 by Priority 4-5.

---

## Cross-References

- [false-bailouts.md](false-bailouts.md) -- Streams 1B, 1C, 1D
- [scope-analysis.md](scope-analysis.md) -- Streams 1A, 3
- [codegen-structure.md](codegen-structure.md) -- Stream 2
- [unnecessary-memo.md](unnecessary-memo.md) -- Streams 1A, 4, 5
- [upstream-errors.md](upstream-errors.md) -- Stream 4
