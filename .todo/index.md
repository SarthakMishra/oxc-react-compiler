# Backlog Index

> Last updated: 2026-03-18

Render equivalence: **88% (22/25 pairs match)**. Conformance: 407/1717 matched. All 160 Rust tests pass, 0 panics.

Remaining 3 render divergences: command-menu (active item styling), canvas-sidebar (JSX text whitespace edge case), multi-step-form (field count logic).

## P2 -- Codegen Correctness (3 remaining render divergences)

- [ ] command-menu: active item class divergence — [codegen-emission.md](codegen-emission.md)
- [ ] canvas-sidebar: minor JSX whitespace edge case — [codegen-emission.md](codegen-emission.md)
- [ ] multi-step-form: field count logic divergence — [codegen-emission.md](codegen-emission.md)

## P2 -- False Bail-outs (coverage)

- [ ] 208 false bail-outs (over-conservative validation) — [validation-gaps.md](validation-gaps.md)#gap-5-false-bail-outs-208-fixtures
- [ ] 66 silent bail-outs (missing compilable patterns) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs-66-fixtures

## P3 -- Scope Quality

- [ ] Over-memoization in 8 fixtures (too many cache slots) — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence

## P4 -- Backlog (not blocking render equivalence)

- [ ] Ternary expression reconstruction (if/else instead of `?:`) — [codegen-emission.md](codegen-emission.md)#gap-6-ternary-expression-reconstruction
