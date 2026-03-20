# Backlog Index

> Last updated: 2026-03-20

Conformance: **443/1717 (25.8%)**. Render equivalence: **96% (24/25)**. All tests pass, 0 panics.

Key breakdown of diverged fixtures:
- ~242 "both compile, slots match" (output format only)
- ~622 "both compile, slots differ" (scope/memoization divergence)
- ~28 silent bail-outs (compile but 0 scopes)
- ~174 "we bail, they compile" (false bail-outs)
- ~138 "we compile, they don't" (we over-compile -- usually fine)

### Lessons learned

- **Validation relaxation without scope fixes causes regressions.** Attempted relaxing `ValidatePreservedManualMemoization` (inner-scope tracking instead of scope-matching) -- conformance dropped 413->385 because we compiled programs incorrectly instead of safely bailing. REVERTED in `4a082dc`. Validation fixes MUST be paired with corresponding scope inference improvements.
- **Under-memoization root cause identified:** `last_use_map` tracks uses too broadly, preventing scope creation. Fix requires removing `last_use_map` + adding missing passes (e.g., `PropagateScopeDependenciesHIR`). This is foundational work that also unblocks validation relaxation.
- **Narrow mutable ranges require upstream's full effect inference.** Attempted switching union condition from `effective_range` (mutation+last_use) to `mutable_range` (mutation-only) 3 times with different prerequisites: (1) without compensating passes → 88%→40%, (2) with PropagateScopeMembership → 88%→36%, (3) with PropagateScopeMembership + JSX Capture edges → 88%→36%. The root cause: upstream's BFS produces wider mutation ranges through more complete aliasing effects (Apply resolution, object property captures, etc.) than ours. Our `effective_range` approximation compensates for missing aliasing effects. Fixing this requires porting upstream's full effect inference pipeline, not just removing `last_use_map`.
- **Frozen mutation + reassignment-after-render relaxation also regresses.** Attempted relaxing both validations (locally-created object exemption for frozen mutations, render-time function exemption for reassignment) -- both caused net conformance drops because we compile programs incorrectly without proper scope inference. Same root cause as validation lesson #1.

## Do NOT Attempt (until prerequisites are met)

- **Gap 11: Narrow mutable ranges** — proven to cause 88%→36% render regression even with compensating passes (PropagateScopeMembership, JSX Capture edges). Requires porting upstream's full aliasing effect pipeline for BFS to produce sufficiently wide mutation ranges without `last_use` hack.
- **Gap 5a: Memoization preservation validation** — proven to cause -28 conformance regression without scope inference fixes. BLOCKED on Gap 11.
- **Gap 5b-5e: All validation relaxation** — frozen mutation, reassignment-after-render both regressed. Same root cause.
- **Gap 7: Over-memoization** — may self-resolve as side effect of Gap 11. Investigate after.
- **Gap 6 codegen: Ternary reconstruction** — P4 cosmetic only, no impact.

## Active Work

(none)

## P1 -- Conformance: Scope/Memoization Divergences (621 fixtures, largest category)

- [ ] Under-memoization: 404 fixtures with fewer slots than upstream (root cause: `last_use_map` too wide) — [scope-inference.md](scope-inference.md)#gap-11-under-memoization
- [ ] Over-memoization: 175 fixtures with more slots than upstream — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence

## P2 -- Conformance: False Bail-outs (205 fixtures)

### Safe to attempt (independent of scope inference):
- [ ] 63 silent bail-outs (compile but 0 scopes, no error) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs
- [ ] 26 false "frozen mutation" errors — [validation-gaps.md](validation-gaps.md)#gap-5b-false-frozen-mutation
- [ ] 16 false "reassigned after render" errors — [validation-gaps.md](validation-gaps.md)#gap-5c-false-reassigned-after-render
- [ ] 14 false "ref access in render" errors — [validation-gaps.md](validation-gaps.md)#gap-5d-false-ref-access-in-render
- [ ] 28 other false bail-outs (variable reassignment, hooks, setState) — [validation-gaps.md](validation-gaps.md)#gap-5e-other-false-bail-outs

### BLOCKED on scope inference (do not attempt):
- [ ] 58 false "memoization preservation" errors — [validation-gaps.md](validation-gaps.md)#gap-5a-false-memoization-preservation — REVERTED attempt caused -28 regression

## P2 -- Conformance: Output Format Divergences

- [ ] Named variable preservation: use original names instead of temps where upstream does — [codegen-emission.md](codegen-emission.md)#gap-12-named-variable-preservation

## P3 -- Render Divergences (3 remaining)

- [ ] command-menu: active item class divergence — [codegen-emission.md](codegen-emission.md)#gap-15-remaining-render-divergences
- [ ] canvas-sidebar: minor content difference — [codegen-emission.md](codegen-emission.md)#gap-15-remaining-render-divergences
- [ ] multi-step-form: "0/0 fields" instead of "0/1 fields" — [codegen-emission.md](codegen-emission.md)#gap-15-remaining-render-divergences

## P4 -- Code Quality

- [ ] Ternary expression reconstruction (if/else instead of `?:`) — [codegen-emission.md](codegen-emission.md)#gap-6-ternary-expression-reconstruction
