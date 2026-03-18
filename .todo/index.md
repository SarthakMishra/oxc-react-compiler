# Backlog Index

> Last updated: 2026-03-18

Render equivalence is at 68% (17/25 pairs). Logical expression flattening fix brought 7 new fixtures to semantic_match. Remaining failures: ternary expression result handling, uninitialized scope outputs, and runtime errors in complex fixtures.

## P0 -- Ternary Expression Codegen

- [ ] Reconstruct ternary expressions from If terminals with result places — [codegen-emission.md](codegen-emission.md)#gap-6-ternary-expression-reconstruction

## P0 -- Scope Output Correctness

- [ ] Fix uninitialized scope outputs for useMemo/useCallback results — [scope-inference.md](scope-inference.md)#gap-8-scope-output-variables-not-produced-inside-scope-body-partially-fixed
- [ ] Fix temporal dead zone / initialization order in scope reload — [scope-inference.md](scope-inference.md)#gap-10-temporal-dead-zone--initialization-order

## P2 -- Validation & Coverage Gaps

- [ ] Fix 208 false bail-outs (over-conservative validation) — [validation-gaps.md](validation-gaps.md)#gap-5-false-bail-outs-208-fixtures
- [ ] Fix 66 silent bail-outs (missing compilable patterns) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs-66-fixtures

## P3 -- Scope Inference Quality

- [ ] Fix over-memoization / slot count divergence — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence
