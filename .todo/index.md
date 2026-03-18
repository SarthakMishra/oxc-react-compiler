# Backlog Index

> Last updated: 2026-03-18

Render equivalence is at 32% (8/25 pairs). Gap 9 (JSX tag temps) is resolved. The remaining 68% failures stem from scope inference issues -- variables declared as scope outputs but not actually produced inside scope bodies, and control flow issues (return inside scope body produces dead code after it).

## P0 -- Scope Output Correctness (Blocks 68% of Renders)

- [ ] Fix scope output variables not produced inside scope body — [scope-inference.md](scope-inference.md)#gap-8-scope-output-variables-not-produced-inside-scope-body
- [ ] Fix temporal dead zone / initialization order in scope reload — [scope-inference.md](scope-inference.md)#gap-10-temporal-dead-zone--initialization-order

## P2 -- Validation & Coverage Gaps

- [ ] Fix 208 false bail-outs (over-conservative validation) — [validation-gaps.md](validation-gaps.md)#gap-5-false-bail-outs-208-fixtures
- [ ] Fix 66 silent bail-outs (missing compilable patterns) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs-66-fixtures

## P3 -- Scope Inference Quality

- [ ] Fix over-memoization / slot count divergence — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence
