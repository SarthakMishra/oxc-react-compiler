# Fixture Conformance Backlog

> Every item exists to increase the fixture pass rate. Nothing else.

**Current: 415/1717 (24.2%)**

**Target: 600/1717 (35%) -- see [roadmap-to-600.md](roadmap-to-600.md)**

Last updated: 2026-03-17

---

## Completed

- [x] Stable IdentifierId refactor (Phase 78) -- file deleted, fully implemented
- [x] useMemo/useCallback argument count fix -- [false-bailouts.md](false-bailouts.md)
- [x] rename_variables pass for scope output temp naming -- [codegen-structure.md](codegen-structure.md)
- [x] validate_no_unsupported_nodes pass -- [upstream-errors.md](upstream-errors.md)
- [x] Post-SSA LoadLocal temp inlining pass (+35 conformance)
- [x] Hook hoisting via scope splitting
- [x] Cross-scope LoadLocal inlining for named variables
- [x] Hybrid effects+instruction frozen-mutation rewrite (Phase 77)
- [x] Method allowlist + ref exclusion + call-conditional exclusion for frozen-mutation
- [x] setState name heuristic gated behind config flag
- [x] Param-only reactive place seeding
- [x] Param-ID reactive seeding with mutable value gate

---

## Failure Breakdown (from analysis at 415/1717)

| Category | Fixtures | Roadmap Stream |
|---|---|---|
| We bail, they compile (silent / 0 scopes) | 66 | [1A](roadmap-to-600.md#1a-silent-bail-outs--0-scopes-produced-66-fixtures) |
| We bail, they compile (preserve-memo) | 54 | [1C](roadmap-to-600.md#1c-preserve-existing-memoization-validation-54-fixtures) |
| We bail, they compile (frozen-mutation FP) | 44 | [1B](roadmap-to-600.md#1b-frozen-mutation-false-positives-44-fixtures) |
| We bail, they compile (other validators) | 57 | [1D](roadmap-to-600.md#1d-minor-validator-fixes-52-fixtures-combined) |
| Both compile, slots MATCH (codegen diff) | 270 | [2A](roadmap-to-600.md#2a-temp-variable-inlining-improvements), [2B](roadmap-to-600.md#2b-scope-declaration-and-dependency-ordering) |
| Both compile, slots DIFFER | 584 | [3](roadmap-to-600.md#stream-3-fix-slot-count-divergences-30-50-fixtures) |
| We compile, they don't | 126 | [4](roadmap-to-600.md#stream-4-reduce-we-compile-they-dont-126-fixtures) |
| Both no memo (format diff) | 94 | [5](roadmap-to-600.md#stream-5-both-no-memo-format-differences-94-fixtures) |

---

## Priority 1 -- Codegen (closest to passing, highest yield per fix)

- [ ] Extend temp variable inlining (property chains, destructuring) (+40-60) -- [codegen-structure.md](codegen-structure.md)#temp-variable-inlining-differences
- [ ] Fix scope declaration/dependency ordering (+10-20) -- [codegen-structure.md](codegen-structure.md)#scope-declaration-ordering

## Priority 2 -- Validator False Positives (direct 1:1 gains)

- [ ] Audit preserve-memo validator against upstream (+20-30) -- [false-bailouts.md](false-bailouts.md)#preserve-memo-validation-false-positives-54-fixtures
- [ ] Wire mutable ranges into frozen-mutation validator (+20-30) -- [false-bailouts.md](false-bailouts.md)#frozen-mutation-false-positives-44-fixtures
- [ ] Fix locals-reassigned-after-render FPs (~26 remaining) -- [false-bailouts.md](false-bailouts.md)#locals-reassigned-false-positives-26-fixtures
- [ ] Fix ref-access-during-render FPs (13 remaining) -- [false-bailouts.md](false-bailouts.md)#ref-access-false-positives-13-fixtures
- [ ] Fix global-reassignment FPs (8 remaining) -- [false-bailouts.md](false-bailouts.md)#global-reassignment-false-positives-8-fixtures

## Priority 3 -- Scope Analysis

- [ ] Fix silent bail-outs: loosen is_mutable_instruction gate (+20-30) -- [scope-analysis.md](scope-analysis.md)
- [ ] Fix scope over-counting (+1/+2 slot fixtures) -- [scope-analysis.md](scope-analysis.md)#over-counting
- [ ] Fix scope under-counting (-1 slot fixtures) -- [scope-analysis.md](scope-analysis.md)#under-counting

## Priority 4 -- Upstream Error Matching & Over-Compilation

- [ ] Match upstream validation errors / reduce over-compilation (126 fixtures) -- [upstream-errors.md](upstream-errors.md)

## Priority 5 -- Format & Structure

- [ ] Fix both-no-memo format differences (94 fixtures) -- [roadmap-to-600.md](roadmap-to-600.md#stream-5-both-no-memo-format-differences-94-fixtures)

---

## Skipped

- Flow fixtures (38) -- OXC parser limitation, not worth fixing
