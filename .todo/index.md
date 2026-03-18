# Backlog Index

> Last updated: 2026-03-18

Render equivalence is at 40% (10/25 pairs). Root cause analysis shows two dominant bugs accounting for nearly all remaining failures: (1) logical/ternary expression flattening destroys short-circuit semantics, and (2) uninitialized scope outputs from variables produced by useMemo/useCallback that are never assigned in the scope's if-branch.

## P0 -- Logical/Ternary Expression Codegen

- [ ] Reconstruct short-circuit expressions from Logical terminals instead of flattening — [codegen-emission.md](codegen-emission.md)#gap-5-logical-expression-flattening
- [ ] Reconstruct ternary expressions from If terminals with result places — [codegen-emission.md](codegen-emission.md)#gap-6-ternary-expression-reconstruction

## P0 -- Scope Output Correctness

- [ ] Fix uninitialized scope outputs for useMemo/useCallback results — [scope-inference.md](scope-inference.md)#gap-8-scope-output-variables-not-produced-inside-scope-body-partially-fixed
- [ ] Fix temporal dead zone / initialization order in scope reload — [scope-inference.md](scope-inference.md)#gap-10-temporal-dead-zone--initialization-order

## P2 -- Validation & Coverage Gaps

- [ ] Fix 208 false bail-outs (over-conservative validation) — [validation-gaps.md](validation-gaps.md)#gap-5-false-bail-outs-208-fixtures
- [ ] Fix 66 silent bail-outs (missing compilable patterns) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs-66-fixtures

## P3 -- Scope Inference Quality

- [ ] Fix over-memoization / slot count divergence — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence
