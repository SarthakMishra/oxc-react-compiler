# Backlog Index

> Last updated: 2026-03-18

Render equivalence is at 36% (9/25 pairs). Gap 8 partially fixed (unscooped StoreLocal variables in scope boundaries). Gap 9 (JSX tag temps) resolved. The remaining 64% failures stem from Destructure-in-scope issues (useState destructures not wired as scope outputs), and control flow issues (return inside scope body produces dead code).

## P0 -- Scope Output Correctness (Blocks 64% of Renders)

- [~] Fix scope output variables not produced inside scope body — [scope-inference.md](scope-inference.md)#gap-8-scope-output-variables-not-produced-inside-scope-body-partially-fixed
- [ ] Fix temporal dead zone / initialization order in scope reload — [scope-inference.md](scope-inference.md)#gap-10-temporal-dead-zone--initialization-order

## P2 -- Validation & Coverage Gaps

- [ ] Fix 208 false bail-outs (over-conservative validation) — [validation-gaps.md](validation-gaps.md)#gap-5-false-bail-outs-208-fixtures
- [ ] Fix 66 silent bail-outs (missing compilable patterns) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs-66-fixtures

## P3 -- Scope Inference Quality

- [ ] Fix over-memoization / slot count divergence — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence
