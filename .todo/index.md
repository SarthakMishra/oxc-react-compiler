# Backlog Index

> Last updated: 2026-03-18

Render equivalence: 68% (17/25 pairs). Conformance: 407/1717 matched. Correctness score: 93.8%. All 196 Rust tests pass, 0 panics. E2E transform coverage: 95-100% across all real projects.

5 fixtures crash for ALL compilers (Original, Babel, OXC) -- these are fixture bugs, not compiler bugs: data-table, time-slot-picker, command-menu, multi-step-form (partial).

## P1 -- Remaining Render Failures

- [ ] availability-schedule: wrong arithmetic (missing continue, operator precedence) — [codegen-emission.md](codegen-emission.md)#gap-7-availability-schedule-arithmetic
- [ ] canvas-sidebar: missing return statement in codegen — [codegen-emission.md](codegen-emission.md)#gap-8-canvas-sidebar-missing-return
- [ ] booking-list: localeCompare undefined on one test case (1/2 match) — [codegen-emission.md](codegen-emission.md)#gap-9-booking-list-localecompare
- [ ] toolbar: 0 scopes due to semantic_difference bail (we bail when babel compiles) — [validation-gaps.md](validation-gaps.md)#gap-7-toolbar-semantic-difference-bail

## P2 -- Scope Inference Quality

- [ ] Fix over-memoization in 8 fixtures (too many cache slots) — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence

## P2 -- Validation & Coverage Gaps

- [ ] Fix 208 false bail-outs (over-conservative validation) — [validation-gaps.md](validation-gaps.md)#gap-5-false-bail-outs-208-fixtures
- [ ] Fix 66 silent bail-outs (missing compilable patterns) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs-66-fixtures
