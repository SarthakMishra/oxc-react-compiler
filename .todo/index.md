# Backlog Index

> Last updated: 2026-03-18

Conformance: **408/1717 (23.8%)**. Render equivalence: **88% (22/25)**. All 196 Rust tests pass, 0 panics.

Key breakdown of 1332 known-failure fixtures:
- 250 "both compile, slots match" (output format only)
- 673 "both compile, slots differ" (scope/memoization divergence)
- 151 "we bail, they compile" (false bail-outs -- down from 205 after memoization fix)
- 143 "we compile, they don't" (we over-compile -- usually fine)
- 87 "both no memo, format diff"
- 28 dep-comparison error fixtures (need `validateInferredDep` implementation)

## Active Work

(none)

## P1 -- Conformance: Output Format Divergences (247 fixtures)

- [ ] Destructuring codegen: emit `const { x } = t0` instead of `const x = t0.x` — [codegen-emission.md](codegen-emission.md)#gap-11-destructuring-pattern-codegen
- [ ] Named variable preservation: use original names instead of temps where upstream does — [codegen-emission.md](codegen-emission.md)#gap-12-named-variable-preservation
- [ ] `async` function keyword emission — [codegen-emission.md](codegen-emission.md)#gap-13-async-function-emission
- [ ] Housekeeping: update known-failures.txt (2 newly passing, 5 regressions) — [codegen-emission.md](codegen-emission.md)#gap-14-known-failures-housekeeping

## P1 -- Conformance: Scope/Memoization Divergences (622 fixtures)

- [ ] Under-memoization: 404 fixtures with fewer slots than upstream (scope merging too aggressive or scopes missing) — [scope-inference.md](scope-inference.md)#gap-11-under-memoization
- [ ] Over-memoization: 175 fixtures with more slots than upstream — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence

## P2 -- Conformance: False Bail-outs (~~205~~ 151 fixtures)

- [x] ~~58~~ 4 false "memoization preservation" errors (54 fixed; 28 dep-comparison error fixtures moved to known-failures) — [validation-gaps.md](validation-gaps.md)#gap-5a-false-memoization-preservation
- [ ] 63 silent bail-outs (compile but 0 scopes, no error) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs
- [ ] 26 false "frozen mutation" errors — [validation-gaps.md](validation-gaps.md)#gap-5b-false-frozen-mutation
- [ ] 16 false "reassigned after render" errors — [validation-gaps.md](validation-gaps.md)#gap-5c-false-reassigned-after-render
- [ ] 14 false "ref access in render" errors — [validation-gaps.md](validation-gaps.md)#gap-5d-false-ref-access-in-render
- [ ] 28 other false bail-outs (variable reassignment, hooks, setState) — [validation-gaps.md](validation-gaps.md)#gap-5e-other-false-bail-outs

## P3 -- Render Divergences (3 remaining)

- [ ] command-menu: active item class divergence — [codegen-emission.md](codegen-emission.md)#gap-15-remaining-render-divergences
- [ ] canvas-sidebar: minor content difference — [codegen-emission.md](codegen-emission.md)#gap-15-remaining-render-divergences
- [ ] multi-step-form: "0/0 fields" instead of "0/1 fields" — [codegen-emission.md](codegen-emission.md)#gap-15-remaining-render-divergences

## P4 -- Code Quality

- [ ] Ternary expression reconstruction (if/else instead of `?:`) — [codegen-emission.md](codegen-emission.md)#gap-6-ternary-expression-reconstruction
