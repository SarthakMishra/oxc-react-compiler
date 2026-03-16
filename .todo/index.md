# Fixture Conformance Backlog

> Every item exists to increase the fixture pass rate. Nothing else.

**Current: 435/1717 (25.3%) -- 1282 failures remaining**

Last updated: 2026-03-16

---

## Priority 0 -- Architectural (root-cause fix for multiple tiers)

- [ ] Stable IdentifierId refactor: reuse declaration IDs for all references to the same binding -- [stable-identifier-ids.md](stable-identifier-ids.md)

> This is the single most impactful change. Fresh-ID-per-reference is the root
> cause of broken value flow in the abstract heap, which cascades into
> frozen-mutation false positives (158), scope over/under-counting (636), and
> incorrect alias tracking. Fixes here propagate across Tiers 1-4.

---

## Failure Breakdown (from automated analysis at 399/1717)

Note: These counts are from the 399/1717 baseline. The parameter seeding
fix (+31 fixtures) reduced over-count and unnecessary-memo categories.
Re-run breakdown analysis to get updated numbers.

| Root Cause | Fixtures | Fix |
|---|---|---|
| Slot over-count (too many scopes/deps) | ~443 | [scope-analysis.md] |
| False-positive frozen-mutation bail-out | 158 | [false-bailouts.md] |
| Slot under-count (missing scopes) | ~162 | [scope-analysis.md] |
| Same slots, different codegen structure | ~150 | [codegen-structure.md] |
| We memoize, upstream returns unchanged | ~102 | [unnecessary-memo.md] |
| Both no-memo, output differs | ~43 | [unnecessary-memo.md] |
| Upstream errors we should match | 35 | [upstream-errors.md] |
| False-positive locals-reassigned bail-out | 11 | [false-bailouts.md] |
| False-positive ref-access bail-out | 11 | [false-bailouts.md] |
| False-positive useMemo/useCallback args | 17 | [false-bailouts.md] |
| False-positive global-reassignment bail-out | 9 | [false-bailouts.md] |
| False-positive setState bail-out | 3 | [false-bailouts.md] |
| Flow syntax (parser limitation) | 38 | Skip |

---

## Tier 1 -- False-Positive Bail-Outs (~256 fixtures)

We reject functions that upstream compiles successfully. Each fix is
a direct 1:1 fixture gain -- bail-out removed = fixture passes.

- [~] Fix frozen-mutation false positives (158 remaining, 19 fixed via direct-only frozen check + param pre-freezing) -- [false-bailouts.md](false-bailouts.md)#frozen-mutation-false-positives
- [ ] Fix frozen-mutation false positive on hooks without JSX (pre-existing regression) -- [false-bailouts.md](false-bailouts.md)#frozen-mutation-hooks-without-jsx
- [~] Fix locals-reassigned-after-render false positives (11 remaining, 15 fixed via render-only detection) -- [false-bailouts.md](false-bailouts.md)#locals-reassigned-false-positives
- [~] Fix ref-access-during-render false positives (11 remaining, 9 fixed via non-render callback detection) -- [false-bailouts.md](false-bailouts.md)#ref-access-false-positives
- [ ] Fix useMemo/useCallback argument count false positives (17 fixtures) -- [false-bailouts.md](false-bailouts.md)#usememo-usecallback-arg-count
- [~] Fix global-reassignment false positives (15 fixtures, partially fixed) -- [false-bailouts.md](false-bailouts.md)#global-reassignment-false-positives
- [~] Fix setState-during-render false positives (3 remaining, 11 fixed via name heuristic gating) -- [false-bailouts.md](false-bailouts.md)#setstate-false-positives

## Tier 2 -- Slot Count Divergences (~636 fixtures)

Both compile with `_c()` but our slot count N differs. 474 over-count,
162 under-count. Fixing scope/dependency analysis is the highest-volume
path but each fix requires careful upstream comparison.

- [ ] Fix scope over-counting: extra reactive scopes (474 fixtures) -- [scope-analysis.md](scope-analysis.md)#over-counting
- [ ] Fix scope under-counting: missing reactive scopes (162 fixtures) -- [scope-analysis.md](scope-analysis.md)#under-counting

## Tier 3 -- Same Slots, Different Structure (~150 fixtures)

Slot count matches but generated code within scopes differs.
These are codegen and scope-internal ordering issues.

- [ ] Fix codegen structure divergences (150 fixtures) -- [codegen-structure.md](codegen-structure.md)#structure-divergences

## Tier 4 -- Unnecessary Memoization (~176 fixtures)

We add `_c()` caching but upstream returns source unchanged.
Root cause: missing DCE, const-prop, or incorrect scope creation
for non-reactive functions.

- [ ] Stop memoizing functions upstream doesn't memoize (133 + 43 fixtures) -- [unnecessary-memo.md](unnecessary-memo.md)#unnecessary-memoization

## Tier 5 -- Upstream Errors (~35 fixtures)

Upstream rejects with an error, we should too.

- [~] Match upstream validation errors (39 fixtures, 4 done via validate_no_unsupported_nodes) -- [upstream-errors.md](upstream-errors.md)#remaining-errors

## Skipped

- Flow fixtures (38) -- OXC parser limitation, not worth fixing
