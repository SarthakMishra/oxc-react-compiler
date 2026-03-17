# Fixture Conformance Backlog

> Every item exists to increase the fixture pass rate. Nothing else.

**Current: 422/1717 (24.6%) -- 1295 failures remaining**

**Target: 600/1717 (35%) -- see [roadmap-to-600.md](roadmap-to-600.md)**

Last updated: 2026-03-17

---

## Completed

- [x] Stable IdentifierId refactor (Phase 78) -- [stable-identifier-ids.md](stable-identifier-ids.md)
- [x] useMemo/useCallback argument count fix -- [false-bailouts.md](false-bailouts.md)#usememo-usecallback-arg-count

---

## Failure Breakdown (from analysis at 422/1717)

| Category | Fixtures | Roadmap Stream |
|---|---|---|
| We bail, they compile (silent / 0 scopes) | 157 | [1A](roadmap-to-600.md#1a-silent-bail-outs--0-scopes-produced-157-fixtures) |
| We bail, they compile (frozen-mutation FP) | 104 | [1B](roadmap-to-600.md#1b-frozen-mutation-false-positives-104-fixtures) |
| We bail, they compile (preserve-memo) | 37 | [1C](roadmap-to-600.md#1c-preserve-existing-memoization-validation-37-fixtures) |
| We bail, they compile (other validators) | 52 | [1D](roadmap-to-600.md#1d-minor-validator-fixes-38-fixtures-combined) |
| Both compile, slots DIFFER | 553 | [2A](roadmap-to-600.md#2a-scope-over-count-our_slots---expected--1-312-fixtures), [2B](roadmap-to-600.md#2b-scope-under-count-our_slots---expected---1-241-fixtures) |
| Both compile, slots MATCH (codegen diff) | 190 | [3A](roadmap-to-600.md#3a-temp-variable-inlining-improvements-80-100-of-the-190), [3B](roadmap-to-600.md#3b-scope-declaration-and-dependency-ordering-20-30-of-the-190) |
| We compile, they don't | 95 | [4A](roadmap-to-600.md#4a-match-upstream-validation-errors-35-fixtures), [4B](roadmap-to-600.md#4b-over-compilation-in-infer-mode-60-fixtures) |
| Both no memo (format diff) | 107 | [5](roadmap-to-600.md#stream-5-both-no-memo-format-differences-107-fixtures) |

---

## Priority 1 -- Quick Wins (immediate, no dependencies)

- [ ] Fix hook-without-JSX frozen-mutation regression (+3-5) -- [roadmap-to-600.md](roadmap-to-600.md#7b-frozen-mutation-hook-without-jsx-regression-3-5-fixtures)
- [~] Fix locals-reassigned-after-render false positives (16 remaining) -- [false-bailouts.md](false-bailouts.md)#locals-reassigned-false-positives
- [~] Fix ref-access-during-render false positives (8 remaining) -- [false-bailouts.md](false-bailouts.md)#ref-access-false-positives
- [~] Fix global-reassignment false positives (8 remaining) -- [false-bailouts.md](false-bailouts.md)#global-reassignment-false-positives
- [~] Fix setState-during-render false positives (3 remaining) -- [false-bailouts.md](false-bailouts.md)#setstate-false-positives

## Priority 2 -- High Yield (major fixture gains)

- [ ] Fix silent bail-outs: loosen is_mutable_instruction gate (+60-80) -- [roadmap-to-600.md](roadmap-to-600.md#1a-silent-bail-outs--0-scopes-produced-157-fixtures)
- [~] Wire mutable ranges into frozen-mutation validator (+50-70) -- [roadmap-to-600.md](roadmap-to-600.md#1b-frozen-mutation-false-positives-104-fixtures)
- [ ] Fix temp variable inlining in codegen (+40-60) -- [roadmap-to-600.md](roadmap-to-600.md#3a-temp-variable-inlining-improvements-80-100-of-the-190)

## Priority 3 -- Scope Analysis

- [ ] Fix scope over-counting: extra reactive scopes (~312 fixtures) -- [scope-analysis.md](scope-analysis.md)#over-counting
- [ ] Fix scope under-counting: missing reactive scopes (~241 fixtures) -- [scope-analysis.md](scope-analysis.md)#under-counting
- [ ] Fix preserve-memo validation false positives (37 fixtures) -- [roadmap-to-600.md](roadmap-to-600.md#1c-preserve-existing-memoization-validation-37-fixtures)

## Priority 4 -- Upstream Error Matching & Over-Compilation

- [~] Match upstream validation errors (35 fixtures) -- [upstream-errors.md](upstream-errors.md)#remaining-errors
- [ ] Fix over-compilation in Infer mode (~60 fixtures) -- [roadmap-to-600.md](roadmap-to-600.md#4b-over-compilation-in-infer-mode-60-fixtures)

## Priority 5 -- Format & Structure

- [ ] Fix codegen scope ordering divergences (~20-30 fixtures) -- [codegen-structure.md](codegen-structure.md)#structure-divergences
- [ ] Fix both-no-memo format differences (107 fixtures) -- [roadmap-to-600.md](roadmap-to-600.md#stream-5-both-no-memo-format-differences-107-fixtures)

---

## Skipped

- Flow fixtures (38) -- OXC parser limitation, not worth fixing
