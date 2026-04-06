# Scope Grouping Algorithm Research

**Date:** 2026-04-05
**Baseline:** 560 matched (`use_mutable_range=false`), 503 matched (`use_mutable_range=true`)

## Summary

Tested multiple scope grouping algorithms to find one that achieves >= 555 with `use_mutable_range=true` while maintaining >= 560 with `use_mutable_range=false`. **No algorithm improves over the current `use_mutable_range=false` approach (560).** The best algorithms achieve 560 but are functionally equivalent to the existing approach.

## Algorithms Tested

### Algorithm 1: Raw mutable range (baseline)

- **use_mutable_range=true:** 503
- **use_mutable_range=false:** 560
- **Description:** Current upstream-matched behavior. Raw `mutable_range.end` for grouping and scope ranges.

### Algorithm 2: Use-based grouping

- **use_mutable_range=true:** 537
- **Description:** Instead of checking `isMutable(instr, operand)`, union lvalue with ALL operands that have `mutable_range.start > 0` (non-global). Groups producer+consumer regardless of mutable range width.
- **Outcome:** Over-merges aggressively. Worse than Algorithm 4.

### Algorithm 4: Selective extension for trivial-range identifiers

Extend mutable range to `last_use + 1` only for identifiers with trivial ranges (`end == start + 1`, never mutated after creation).

| Threshold (span <=) | Score |
|---------------------|-------|
| 1 (exact match)     | 544   |
| <= 1                | 550   |
| <= 2                | 550   |
| <= 5                | 550   |
| <= 10               | 551   |
| <= 20               | 559   |
| <= 25               | 559   |
| <= 28               | 560   |
| <= 30               | 560   |
| <= 50               | 560   |
| always              | 560   |

- **Best result:** 560 at threshold >= 28
- **No regression:** All variants maintain 560 with `use_mutable_range=false`

### Algorithm 4c: Fixed extension (+1/+2) for trivial-range

- **+1 extension:** 516
- **+2 extension:** 516
- **Outcome:** Fixed extensions too small to capture variable-distance consumers.

### Algorithm 5b: Split grouping/scoping

Group with full `last_use` extension, scope ranges with raw mutable range.

- **use_mutable_range=true:** 538
- **Outcome:** Tight scope ranges hurt -- scopes need to cover last_use to correctly include consuming instructions.

### Algorithm 5d: Group selectively, scope with last_use

Group with Algorithm 4's selective extension (trivial-range only), scope ranges always extended to last_use.

- **use_mutable_range=true:** 560
- **Outcome:** Matches baseline. No improvement in diverged fixtures.

### Algorithm 6: Trivial-range extension in isMutable only

Extend trivial-range identifiers in `isMutable` check but not in lvalue check.

- **use_mutable_range=true:** 526
- **Outcome:** Worse than Algorithm 4. The lvalue check extension is critical.

## Detailed Comparison: Algorithm 4 (threshold 28) vs Baseline

Both achieve 560 matched fixtures. Comparing diverged fixtures:

| Metric | Baseline (false) | Alg 4 thresh 28 (true) |
|--------|-----------------|----------------------|
| Matched | 560 | 560 |
| Diverged fixtures | 226 | 227 |
| Total slot diffs | 448 | 453 |

Algorithm 4 regresses exactly 1 fixture (`capture-backedge-phi-with-later-mutation.js`) while fixing 1 other, resulting in a net-even trade at 560.

## Key Findings

1. **The gap from 503 to 560 is caused by identifiers with mutable ranges spanning 1-28 instructions.** These are intermediary temps (PropertyLoad results, function call results) whose mutable range is narrower than their actual usage span.

2. **No algorithm improves over the current `use_mutable_range=false` approach.** The best results achieve 560, matching the existing behavior. The threshold-based approaches converge to equivalence with the current "always extend" approach.

3. **The over-merging problem (419 fixtures) cannot be fixed by changing the grouping algorithm.** The 419 fixtures where we over-merge are caused by the last_use extension making unrelated allocations overlap. But removing the extension causes -57 regression because it's needed for correct grouping of intermediary temps.

4. **The root cause is instruction ID divergence from upstream.** Upstream's mutable ranges are computed from instruction IDs that differ from ours (due to SSA differences, phi placement, instruction ordering). The last_use extension is a workaround for these structural differences.

5. **The scope range computation (Phase 3) MUST use last_use extension.** Algorithms that use raw mutable range for scope ranges (Algorithm 5b: 538) lose fixtures because scopes are too narrow to encompass consuming instructions.

## Conclusion

The `effective_range = max(mutable_range, last_use + 1)` workaround is the correct approach given our current HIR structure. Fixing the 419 over-merged fixtures requires either:

1. **Matching upstream instruction IDs exactly** -- eliminating SSA divergences so mutable ranges naturally cover consumers
2. **Implementing PruneNonEscapingScopes** -- upstream's pass that removes unnecessary scopes after grouping, which would post-hoc fix some over-merging
3. **A fundamentally different scope inference approach** not based on mutable range overlap at all (e.g., dependency-based scope boundaries from PropagateScopeDependencies)

None of these can be solved by tweaking the grouping algorithm parameters alone.
