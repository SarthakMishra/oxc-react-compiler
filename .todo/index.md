# Backlog Index

> Last updated: 2026-03-19

Conformance: **413/1717 (24.1%)**. Render equivalence: **88% (22/25)**. All 196 tests pass, 0 panics. Correctness: 93.8%.

Key breakdown of diverged fixtures:
- ~248 "both compile, slots match" (output format only)
- ~621 "both compile, slots differ" (scope/memoization divergence)
- ~205 "we bail, they compile" (false bail-outs)
- ~138 "we compile, they don't" (we over-compile -- usually fine)
- ~93 "both no memo, format diff"

### Lessons learned

- **Validation relaxation without scope fixes causes regressions.** Attempted relaxing `ValidatePreservedManualMemoization` (inner-scope tracking instead of scope-matching) -- conformance dropped 413->385 because we compiled programs incorrectly instead of safely bailing. REVERTED in `4a082dc`. Validation fixes MUST be paired with corresponding scope inference improvements.
- **Under-memoization root cause identified:** `last_use_map` tracks uses too broadly, preventing scope creation. Fix requires removing `last_use_map` + adding missing passes (e.g., `PropagateScopeDependenciesHIR`). This is foundational work that also unblocks validation relaxation.

## Active Work

(none)

## P1 -- Conformance: Scope/Memoization Divergences (621 fixtures, largest category)

- [ ] Under-memoization: 404 fixtures with fewer slots than upstream (root cause: `last_use_map` too wide) — [scope-inference.md](scope-inference.md)#gap-11-under-memoization
- [ ] Over-memoization: 175 fixtures with more slots than upstream — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence

## P2 -- Conformance: False Bail-outs (205 fixtures) -- BLOCKED on scope inference

> **Note:** Relaxing validation without fixing scope inference causes net regressions (proven by reverted attempt). These items should only be attempted after scope inference improvements land.

- [ ] 63 silent bail-outs (compile but 0 scopes, no error) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs
- [ ] 58 false "memoization preservation" errors — [validation-gaps.md](validation-gaps.md)#gap-5a-false-memoization-preservation
- [ ] 26 false "frozen mutation" errors — [validation-gaps.md](validation-gaps.md)#gap-5b-false-frozen-mutation
- [ ] 16 false "reassigned after render" errors — [validation-gaps.md](validation-gaps.md)#gap-5c-false-reassigned-after-render
- [ ] 14 false "ref access in render" errors — [validation-gaps.md](validation-gaps.md)#gap-5d-false-ref-access-in-render
- [ ] 28 other false bail-outs (variable reassignment, hooks, setState) — [validation-gaps.md](validation-gaps.md)#gap-5e-other-false-bail-outs

## P2 -- Conformance: Output Format Divergences

- [ ] Named variable preservation: use original names instead of temps where upstream does — [codegen-emission.md](codegen-emission.md)#gap-12-named-variable-preservation

## P3 -- Render Divergences (3 remaining)

- [ ] command-menu: active item class divergence — [codegen-emission.md](codegen-emission.md)#gap-15-remaining-render-divergences
- [ ] canvas-sidebar: minor content difference — [codegen-emission.md](codegen-emission.md)#gap-15-remaining-render-divergences
- [ ] multi-step-form: "0/0 fields" instead of "0/1 fields" — [codegen-emission.md](codegen-emission.md)#gap-15-remaining-render-divergences

## P4 -- Code Quality

- [ ] Ternary expression reconstruction (if/else instead of `?:`) — [codegen-emission.md](codegen-emission.md)#gap-6-ternary-expression-reconstruction
