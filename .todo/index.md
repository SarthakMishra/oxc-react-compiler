# Backlog Index

> Last updated: 2026-03-19

Conformance: **409/1717 (23.8%)**. Render equivalence: **88% (22/25)**. All 196 tests pass, 0 panics.

Key breakdown of diverged fixtures:
- ~248 "both compile, slots match" (output format only)
- ~621 "both compile, slots differ" (scope/memoization divergence)
- ~205 "we bail, they compile" (false bail-outs)
- ~138 "we compile, they don't" (we over-compile -- usually fine)
- ~93 "both no memo, format diff"

### Lessons learned

- **Validation relaxation without scope fixes causes regressions.** Attempted relaxing `ValidatePreservedManualMemoization` (inner-scope tracking instead of scope-matching) -- conformance dropped 413->385 because we compiled programs incorrectly instead of safely bailing. REVERTED in `4a082dc`. Validation fixes MUST be paired with corresponding scope inference improvements.
- **Under-memoization root cause identified:** `last_use_map` tracks uses too broadly, preventing scope creation. Fix requires removing `last_use_map` + adding missing passes (e.g., `PropagateScopeDependenciesHIR`). This is foundational work that also unblocks validation relaxation.
- **last_use_map removal causes catastrophic render regression.** Attempted 3-step approach: (1) remove end-clamping in align_scopes, (2) add PropagateScopeDependenciesHIR pre-pass, (3) remove last_use_map. Render dropped 88%→24% (22/25→6/25). REVERTED all 3 commits. The codegen and scope inference are deeply coupled to the wide ranges produced by last_use_map — narrowing ranges breaks scope body construction, destructure hoisting, and cache slot generation. This is NOT incrementally fixable with the current architecture.
- **Frozen mutation + reassignment-after-render relaxation also regresses.** Attempted relaxing both validations (locally-created object exemption for frozen mutations, render-time function exemption for reassignment) -- both caused net conformance drops because we compile programs incorrectly without proper scope inference. Same root cause as validation lesson #1.

## Do NOT Attempt (until prerequisites are met)

- **Gap 11: last_use_map removal** — proven to cause 88%→24% render regression. The entire codegen/scope pipeline depends on wide ranges. NOT incrementally fixable — requires full architecture rework matching upstream's pass structure.
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
