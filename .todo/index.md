# Backlog Index

> Last updated: 2026-03-18

Render equivalence: 68% nominal (17/25 pairs), but 5 fixtures crash for ALL compilers (fixture bugs, not compiler bugs). Adjusted: **17/20 valid fixtures match (85%)**. Conformance: 407/1717 matched. Correctness score: 93.8%. All 196 Rust tests pass, 0 panics.

Fixture bugs (crash for Original + Babel + OXC -- not our problem): data-table, time-slot-picker, command-menu, multi-step-form (1 case), booking-list (1 case).

## P1 -- Codegen Correctness (2 remaining render failures)

- [ ] canvas-sidebar: missing return statement in compiled output — [codegen-emission.md](codegen-emission.md)#gap-8-canvas-sidebar-missing-return
- [ ] availability-schedule: missing `continue` + operator precedence — [codegen-emission.md](codegen-emission.md)#gap-7-availability-schedule-arithmetic

## P2 -- False Bail-outs (coverage)

- [ ] toolbar: 0 scopes due to false semantic_difference bail — [validation-gaps.md](validation-gaps.md)#gap-7-toolbar-semantic-difference-bail
- [ ] 208 false bail-outs (over-conservative validation) — [validation-gaps.md](validation-gaps.md)#gap-5-false-bail-outs-208-fixtures
- [ ] 66 silent bail-outs (missing compilable patterns) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs-66-fixtures

## P3 -- Scope Quality

- [ ] Over-memoization in 8 fixtures (too many cache slots) — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence

## P4 -- Backlog (not blocking render equivalence)

- [ ] Ternary expression reconstruction (if/else instead of `?:`) — [codegen-emission.md](codegen-emission.md)#gap-6-ternary-expression-reconstruction
- [ ] Fix test fixtures that crash on undefined props — [codegen-emission.md](codegen-emission.md)#fixture-bugs
