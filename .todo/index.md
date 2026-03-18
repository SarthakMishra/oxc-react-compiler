# Backlog Index

> Last updated: 2026-03-18

Render equivalence is at 40% (10/25 pairs). Gap 8 (destructure-in-scope) substantially fixed via codegen hoisting — todo-list now passes. Gap 9 (JSX tag temps) resolved. Gap 10 (unscooped variables) resolved. Remaining failures stem from other scope output issues (useCallback phantom temps), control flow issues (return inside scope body), and remaining destructure edge cases in complex fixtures.

## P0 -- Scope Output Correctness

- [ ] Fix useCallback/useMemo phantom temp scope outputs — remaining fixtures still use phantom temps for non-destructure cases
- [ ] Fix temporal dead zone / initialization order in scope reload — [scope-inference.md](scope-inference.md)#gap-10-temporal-dead-zone--initialization-order

## P2 -- Validation & Coverage Gaps

- [ ] Fix 208 false bail-outs (over-conservative validation) — [validation-gaps.md](validation-gaps.md)#gap-5-false-bail-outs-208-fixtures
- [ ] Fix 66 silent bail-outs (missing compilable patterns) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs-66-fixtures

## P3 -- Scope Inference Quality

- [ ] Fix over-memoization / slot count divergence — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence
