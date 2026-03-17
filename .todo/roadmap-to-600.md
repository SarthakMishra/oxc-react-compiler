# Roadmap: 422 to 600+ Conformance

> Target: +178 fixtures minimum (422/1717 -> 600/1717, 35%)
>
> Current failure breakdown (1295 total):
> - We bail, they compile: 350
> - We compile, they don't: 95
> - Both compile, slots MATCH: 190
> - Both compile, slots DIFFER: 553
> - Both no memo (format diff): 107

---

## Executive Summary

The 600 target requires +178 fixtures. This roadmap identifies 7 work
streams ordered by fixture-yield and dependency structure. The critical
path runs through three areas: (1) eliminating false bail-outs (~194
recoverable), (2) fixing slot count divergences via mutable-range-based
scope analysis (~50-80 recoverable from stable IDs + mutable ranges),
and (3) codegen structure fixes for the 190 "slots match" fixtures
(~40-60 recoverable). Together these three streams provide a
conservative +180 ceiling and a realistic +178 target.

---

## Stream 1: Eliminate Remaining False Bail-Outs (+120-150 fixtures)

The 350 "we bail, they compile" category is the highest-yield target
because each fix is a direct 1:1 fixture gain. The sub-categories:

### 1A: Silent bail-outs / 0 scopes produced (157 fixtures)

**What's happening:** We compile successfully but produce 0 reactive
scopes, so we return source unchanged. Upstream produces scopes and
memoized output for these same fixtures. This is NOT a bail-out in
the validation sense -- it's a scope analysis gap where our pipeline
runs to completion but fails to create scopes that upstream creates.

**Root cause:** The `is_mutable_instruction` gate in
`infer_reactive_scope_variables.rs` prevents scope creation for
instruction sets that are reactive but contain only non-allocating,
non-mutable instructions (e.g., LoadLocal of a param, arithmetic on
params, string concatenation). Upstream creates scopes for these
because it uses `ValueKind` from `InferMutableRanges` which classifies
more instructions as mutable (especially CallExpression results and
function expressions that capture reactive values).

**Fix strategy:**
1. Audit `is_mutable_instruction()` against upstream's scope creation
   criteria -- the function is likely too restrictive
2. Check if `CallExpression` results are being classified as mutable
   (they should be -- calls can return objects)
3. Check if function expressions that capture reactive values get scopes
4. Review whether the `any_reactive && any_mutable` gate matches
   upstream's `InferReactiveScopeVariables.ts` logic

**Estimated yield:** 60-80 fixtures (many of the 157 are genuinely
non-reactive functions where upstream also produces 0 scopes but the
fixture comparison fails due to format differences)

**Depends on:** Nothing (can start immediately)

**Risk:** Medium. Loosening the mutable gate could cause over-scoping
regressions in other categories. Need careful A/B testing.

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/infer_reactive_scope_variables.rs`
- `crates/oxc_react_compiler/src/entrypoint/program.rs` (zero-scope skip)

### 1B: Frozen-mutation false positives (104 fixtures)

**What's happening:** `validate_no_mutation_after_freeze` rejects
functions that upstream compiles. The hybrid rewrite (Phase 77) reduced
this from 158 to ~104 but significant false positives remain.

**Root cause:** The validator uses name-based freeze tracking as a
fallback for cases the effects layer doesn't cover. With stable
IdentifierIds now in place, the abstract heap can correctly propagate
freeze/mutate state across references -- but the validator hasn't been
updated to consume this data.

**Fix strategy:**
1. Export mutable range data from `infer_mutation_aliasing_ranges`:
   `IdentifierId -> MutableRange { start: InstructionId, end: InstructionId }`
2. Thread mutable ranges through the pipeline to the validator
3. Replace name-based freeze heuristics with range-based checks:
   a value is frozen at instruction I if `mutable_range.end < I`
4. Keep effects-based `MutateFrozen` detection as the primary layer
5. Use mutable ranges as the secondary layer (replacing name tracking)

**Estimated yield:** 50-70 fixtures (the remaining 104 minus cases
that need alias/phi/derivation chain tracking which mutable ranges
alone won't solve)

**Depends on:** Stable IdentifierIds (done)

**Risk:** Medium. Mutable range computation in
`infer_mutation_aliasing_ranges.rs` may not be fully correct yet.
Need to verify range accuracy on sample fixtures before wiring to
the validator.

**Key files:**
- `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs`
- `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`
- `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`

### 1C: Preserve-existing-memoization validation (37 fixtures)

**What's happening:** `validate_preserved_manual_memoization` (Pass 61)
incorrectly rejects fixtures that upstream accepts. The validator checks
that compiler-generated memoization preserves any manually-written
useMemo/useCallback. When our scope analysis differs from upstream's,
the validator sees scopes that don't match the manual memo sites and
flags them.

**Root cause:** This is a downstream symptom of scope analysis
divergences. When our scopes don't match upstream's, the preserve-memo
validator sees different scope boundaries and rejects. Additionally,
the stable IdentifierId refactor changed scope analysis enough to
cause 5 new preserve-memo regressions.

**Fix strategy:**
1. Audit the preserve-memo validator against upstream
   `ValidatePreservedManualMemoization.ts` -- check if our scope
   matching logic is correct
2. Some fixtures may fix themselves as scope analysis improves
3. Consider relaxing the validator to accept "superset" memoization
   (we memoize more than manual, which is safe)

**Estimated yield:** 15-25 fixtures (some will fix as scope analysis
improves, some need validator logic fixes)

**Depends on:** Partially on Stream 2 (scope analysis improvements)

**Risk:** Low. Validator changes are isolated.

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_preserved_manual_memoization.rs`

### 1D: Minor validator fixes (38 fixtures combined)

| Validator | Fixtures | Fix |
|---|---|---|
| locals-reassigned-after-render | 16 | Trace callback identity through StoreLocal chains for effect detection |
| Cannot reassign outside component/hook | 8 | Indirect callback patterns in global reassignment validator |
| Cannot access refs during render | 8 | Ref access in event handler callbacks passed through variables |
| Hooks referenced as values | 3 | Check if we over-flag hook references in non-call positions |
| Misc (setState, deps, conditional) | 3 | Conditional setState through lambda chains |

**Estimated yield:** 20-30 fixtures

**Depends on:** Nothing

**Risk:** Low. Each is a self-contained validator fix.

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_locals_not_reassigned_after_render.rs`
- `crates/oxc_react_compiler/src/validation/validate_no_global_reassignment.rs`
- `crates/oxc_react_compiler/src/validation/validate_no_ref_access_in_render.rs`
- `crates/oxc_react_compiler/src/validation/validate_hooks_usage.rs`
- `crates/oxc_react_compiler/src/validation/validate_no_set_state_in_render.rs`

---

## Stream 2: Fix Slot Count Divergences (+50-80 fixtures)

The 553 "both compile, slots DIFFER" category is the largest single
bucket but also the hardest -- each fixture requires diagnosing which
pass produces the wrong scope structure.

### 2A: Scope over-count (our_slots - expected >= +1, ~312 fixtures)

The +1 (110) and +2 (125) cases are the most tractable because the
structural difference is small.

**Sub-causes:**

**2A-i: Extra scopes for non-escaping values (~40-60 fixtures)**

`prune_non_escaping_scopes` may not be pruning all scopes whose
declarations are never read outside the scope. Upstream's
`PruneNonEscapingScopes.ts` checks whether scope declarations
"escape" (are used after the scope ends or passed to external
functions). Our implementation may miss some escape patterns.

**2A-ii: Missing scope merges (~30-50 fixtures)**

`MergeReactiveScopesThatInvalidateTogether` has been implemented
(Sub-tasks 4a-4f) but may not match upstream's merge criteria exactly.
The +1 over-count fixtures are strong candidates: two scopes that
should have been merged into one.

**2A-iii: Extra dependencies in scopes (~20-30 fixtures)**

Scopes with correct declaration sets but too many dependencies.
Check if stable IdentifierIds improved dependency deduplication
in `propagate_dependencies.rs`.

**Fix strategy:**
1. Sample 20 fixtures from +1 over-count, diff scope structure
2. Categorize: extra scope (prune issue) vs extra dep vs missed merge
3. Fix the most common sub-cause first

**Estimated yield:** 30-50 fixtures

**Depends on:** Stable IdentifierIds (done), benefits from 1B (mutable ranges)

**Risk:** Medium. Scope analysis changes affect all compilation.

### 2B: Scope under-count (our_slots - expected <= -1, ~241 fixtures)

Under-counting means we're missing scopes or missing declarations.
This overlaps with Stream 1A (zero-scope production).

**Fix strategy:** Many under-count fixtures will improve as 1A
(silent bail-outs) and 2A (over-count) are fixed, because the root
causes often overlap (e.g., fixing is_mutable_instruction affects
both categories).

**Estimated yield:** 20-30 fixtures (as side-effect of other fixes)

**Depends on:** Streams 1A, 2A

---

## Stream 3: Codegen Structure Fixes (+40-60 fixtures)

The 190 "both compile, slots MATCH" fixtures have correct scope
structure but wrong code generation within scopes. These are the
closest-to-passing fixtures.

### 3A: Temp variable inlining improvements (~80-100 of the 190)

**What's happening:** We emit intermediate SSA temporaries that
upstream inlines. Patterns like `const t0 = props.x; return t0;`
should be `return props.x;`.

**Root cause:** Our post-SSA temp inlining in codegen.rs handles
single-use temps but may miss:
- Property chain collapse (`t0 = a; t1 = t0.b` -> `a.b`)
- Temps in scope dependency positions
- Temps across scope boundaries
- Destructuring pattern temps

**Fix strategy:**
1. Sample 20 "slots match" fixtures, categorize the diff type
2. If temp inlining is dominant, extend the inlining pass to cover
   more patterns (property chains, destructuring)
3. If ordering is dominant, fix declaration/dependency ordering

**Estimated yield:** 40-60 fixtures

**Depends on:** Nothing (can start immediately)

**Risk:** Low. Codegen changes don't affect scope analysis.

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`

### 3B: Scope declaration and dependency ordering (~20-30 of the 190)

**What's happening:** Within a scope guard, the order of `$[N] = value`
assignments or `$[0] !== dep0 || $[1] !== dep1` checks differs from
upstream.

**Fix strategy:** Match upstream's ordering in
`CodegenReactiveFunction.ts` -- declarations are ordered by their
source position, dependencies by their position in the dependency
collection pass.

**Estimated yield:** 10-20 fixtures

**Depends on:** Nothing

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`

---

## Stream 4: Reduce "We Compile, They Don't" (95 fixtures)

These are cases where upstream rejects with an error but we compile.
Each fix requires matching an upstream validation.

### 4A: Match upstream validation errors (~35 fixtures)

From `.todo/upstream-errors.md`: 23 validation errors we should emit
plus 12 internal errors.

**Fix strategy:**
- 3 preserve-memo validation errors: will improve with Stream 1C
- 2 type-provider configuration errors: add type provider validation
- 2 frozen-mutation errors: will improve with Stream 1B
- Other individual validators: case-by-case

**Estimated yield:** 15-25 fixtures

**Depends on:** Partially on Streams 1B, 1C

### 4B: Over-compilation in Infer mode (~60 fixtures)

We may compile functions that upstream skips in `compilationMode:"infer"`
(non-component, non-hook functions). If we're compiling helpers or
utility functions that upstream doesn't touch, we produce memoized
output for functions that should pass through unchanged.

**Fix strategy:**
1. Check if `@compilationMode` directive parsing is complete
2. Verify component/hook detection matches upstream heuristics
3. Check if we're compiling nested helper functions that upstream skips

**Estimated yield:** 10-20 fixtures

**Depends on:** Nothing

**Risk:** Low. Detection changes are isolated.

**Key files:**
- `crates/oxc_react_compiler/src/entrypoint/program.rs`

---

## Stream 5: "Both No Memo" Format Differences (107 fixtures)

Both compilers produce non-memoized output but the source text differs.
Upstream may apply DCE, const-prop, or other transforms even when not
memoizing.

**Fix strategy:**
1. Check if upstream applies transforms before the zero-scope bail-out
2. If so, run DCE/const-prop before checking scope count
3. May also involve whitespace/formatting normalization

**Estimated yield:** 10-20 fixtures (low-hanging formatting fixes)

**Depends on:** Nothing

**Risk:** Low.

**Key files:**
- `crates/oxc_react_compiler/src/optimization/dead_code_elimination.rs`
- `crates/oxc_react_compiler/src/optimization/constant_propagation.rs`

---

## Stream 6: Leverage Stable IdentifierIds (cross-cutting)

The stable IdentifierId refactor (Phase 78) is complete but the
downstream passes haven't been updated to take advantage of it.
This is a force multiplier for Streams 1-3.

### 6A: Simplify propagate_dependencies with stable IDs

**What's needed:** Remove the dual-tracking (IdentifierId + DeclarationId)
in `propagate_dependencies.rs`. With stable IDs, IdentifierId alone
is sufficient for dependency identity. This should fix dep deduplication
bugs that cause over-counting.

**Estimated yield:** Counted in Stream 2A (10-20 fixtures from better deps)

**Depends on:** Stable IdentifierIds (done)

### 6B: Simplify frozen-mutation validator

**What's needed:** Replace name-based freeze tracking with ID-based
tracking. The abstract heap now has stable IDs, so freeze propagation
can use IDs directly.

**Estimated yield:** Counted in Stream 1B

**Depends on:** Stable IdentifierIds (done)

---

## Stream 7: Quick Wins (immediate, independent)

### 7A: useMemo/useCallback argument count fix (+17 fixtures)

Remove the argument-count validation from `validate_use_memo.rs`.
`useMemo(fn)` without deps is valid React. This is documented in
`.todo/false-bailouts.md` as "likely a 5-line fix yielding 17 fixtures."

**The validate_use_memo.rs file already has the fix applied** (comment
at line 42-44 says "DIVERGENCE: Upstream does NOT validate argument
count"). Verify this is working correctly by checking if the 17
fixtures are now passing.

**Estimated yield:** 0 (already fixed) or 17 if the fix isn't active

**Depends on:** Nothing

### 7B: Frozen-mutation hook-without-JSX regression (+3-5 fixtures)

Gate param pre-freeze in `validate_no_mutation_after_freeze` on whether
the function contains JSX. Hooks without JSX should not pre-freeze
their params.

**Estimated yield:** 3-5 fixtures

**Depends on:** Nothing

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`

---

## Prioritized Execution Order

The streams are ordered by a combination of fixture yield, dependency
structure, and risk. Work within a stream can often be parallelized.

| Priority | Stream | Estimated Yield | Cumulative | Dependencies |
|---|---|---|---|---|
| 1 | 7A: useMemo arg count fix | 0-17 | 422-439 | None |
| 2 | 7B: Hook-without-JSX freeze | 3-5 | 425-444 | None |
| 3 | 1D: Minor validator fixes | 20-30 | 445-474 | None |
| 4 | 1A: Silent bail-outs (is_mutable_instruction) | 60-80 | 505-554 | None |
| 5 | 1B: Frozen-mutation mutable ranges | 50-70 | 555-624 | Stable IDs (done) |
| 6 | 3A: Temp inlining codegen | 40-60 | 595-684 | None |
| 7 | 1C: Preserve-memo validator | 15-25 | 610-709 | Partial: Stream 2 |
| 8 | 2A: Scope over-count | 30-50 | 640-759 | Stable IDs (done) |
| 9 | 4A: Upstream error matching | 15-25 | 655-784 | Partial: 1B, 1C |
| 10 | 3B: Scope ordering | 10-20 | 665-804 | None |
| 11 | 5: Format differences | 10-20 | 675-824 | None |
| 12 | 2B: Scope under-count | 20-30 | 695-854 | 1A, 2A |
| 13 | 4B: Over-compilation | 10-20 | 705-874 | None |

**Conservative path to 600:** Streams 1-5 (Priorities 1-6) yield a
conservative +120-180 fixtures, reaching 542-602. Adding Stream 1C
or 2A pushes past 600 comfortably.

**Aggressive path to 600:** Priorities 1-5 alone could reach 600 if
the mutable-range integration (1B) and is_mutable_instruction fix (1A)
hit their upper yield estimates.

---

## Architectural Risks

### Risk 1: is_mutable_instruction loosening causes over-scoping

Loosening the mutable gate (Stream 1A) to create more scopes will fix
silent bail-outs but may worsen the over-count category. Mitigation:
audit upstream's exact scope creation criteria in
`InferReactiveScopeVariables.ts` before making changes.

### Risk 2: Mutable range accuracy

The mutable range data from `infer_mutation_aliasing_ranges.rs` may
not be fully accurate. If ranges are too wide, the frozen-mutation
validator will under-flag (missing real mutations). If too narrow,
false positives persist. Mitigation: validate ranges on sample
fixtures before wiring to the validator.

### Risk 3: Stable IdentifierId downstream effects

The stable ID refactor is foundational but "verified compatible"
rather than "verified optimal" for downstream passes. Passes that
previously worked around fresh-per-reference IDs may now have
redundant or incorrect logic. Mitigation: each stream should audit
the passes it touches for stable-ID-aware simplifications.

### Risk 4: Cross-stream interference

Scope analysis changes (Streams 1A, 2A, 2B) affect all compilation.
A fix in one stream can cause regressions in another. Mitigation:
run full conformance suite after each change, track net gain not
just per-category gain.

---

## Measurement Plan

After each stream completion:
1. Run conformance suite, record total passing count
2. Re-run failure categorization script to get updated breakdown
3. Update this roadmap with actual yields
4. Adjust priorities based on observed yields vs estimates

---

## Cross-References

- [false-bailouts.md](false-bailouts.md) -- Streams 1B, 1C, 1D, 7A, 7B
- [scope-analysis.md](scope-analysis.md) -- Streams 1A, 2A, 2B
- [codegen-structure.md](codegen-structure.md) -- Stream 3
- [unnecessary-memo.md](unnecessary-memo.md) -- Streams 1A, 4B, 5
- [upstream-errors.md](upstream-errors.md) -- Stream 4A
- [stable-identifier-ids.md](stable-identifier-ids.md) -- Stream 6
