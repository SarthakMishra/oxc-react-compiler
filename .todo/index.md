# Backlog Index

> Last updated: 2026-03-20 (post Phase 106)

Conformance: **445/1717 (25.9%)**. Render equivalence: **96% (24/25)**. Correctness: **93.8%**. All 196 tests pass, 0 panics.

Key breakdown of diverged fixtures:
- ~250 "both compile, slots match" (output format only)
- ~628 "both compile, slots differ" (scope/memoization divergence)
- ~158 "we bail, they compile" (false bail-outs, down from 174)
- ~147 "we compile, they don't" (we over-compile -- usually fine)
- ~89 "both no memo, format diff"
- 28 silent bail-outs (compile but 0 scopes, no error)

### Lessons learned

- **Validation relaxation without scope fixes causes regressions.** Attempted relaxing `ValidatePreservedManualMemoization` -- conformance dropped 413->385 because we compiled programs incorrectly instead of safely bailing. REVERTED in `4a082dc`. Validation fixes MUST be paired with corresponding scope inference improvements.
- **Under-memoization root cause identified:** BFS mutation propagation produces narrower ranges than upstream's abstract interpreter. Our `effective_range` approximation compensates. Fixing requires porting upstream's full abstract interpreter state machine.
- **Narrow mutable ranges require upstream's full effect inference.** Attempted 4 times, all reverted (96%→36% render regression each time).
- **Validation false positives:** Reassignment validator FIXED via `function_context.rs` (16 fewer bail-outs). Ref-access validator partially investigated — fix requires per-body `directly_called` computation to avoid 3 `error.*` regressions. See [validation-gaps.md](validation-gaps.md).

## Do NOT Attempt (until prerequisites are met)

- **Gap 11: Narrow mutable ranges** -- proven to cause 88%->36% render regression (4 attempts reverted). Requires porting upstream's full aliasing effect pipeline.
- **Gap 5a: Memoization preservation validation** -- proven to cause -28 conformance regression without scope inference fixes. BLOCKED on Gap 11.
- **Gap 7: Over-memoization** -- may self-resolve as side effect of Gap 11. Investigate after.
- **Gap 6 codegen: Ternary reconstruction** -- P4 cosmetic only, no impact.

## Recently Completed: Reassignment Validator Fix

Reassignment validator rewritten with ID-based alias tracking (`function_context.rs`). 16 fewer bail-outs (174→158), 0 regressions. The ref-access validator (14 fixtures) still needs a separate fix using the shared `collect_directly_called_fe_ids` utility — see [validation-gaps.md](validation-gaps.md).

## P1 -- Conformance: Scope/Memoization Divergences (622 fixtures, largest category)

- [ ] Under-memoization: ~404 fixtures with fewer slots than upstream — [scope-inference.md](scope-inference.md)#gap-11
- [ ] Over-memoization: ~175 fixtures with more slots than upstream — [scope-inference.md](scope-inference.md)#gap-7

## P2 -- Conformance: False Bail-outs (~174 fixtures)

### Ready to implement:
- [x] ~~26 false "reassigned after render"~~ — DONE: 16 fewer bail-outs via `function_context.rs`
- [ ] 14 false "ref access in render" — needs separate fix using `collect_directly_called_fe_ids`
- [ ] ~29 false "frozen mutation" errors — [validation-gaps.md](validation-gaps.md)#gap-5b
- [ ] 28 silent bail-outs (compile but 0 scopes, no error) — [validation-gaps.md](validation-gaps.md)#gap-6
- [ ] 28 other false bail-outs (variable reassignment, hooks, setState) — [validation-gaps.md](validation-gaps.md)#gap-5e

### BLOCKED on scope inference (do not attempt):
- [ ] 58 false "memoization preservation" errors — REVERTED attempt caused -28 regression

## P2 -- Conformance: Output Format Divergences

- [ ] Named variable preservation: ~34 remaining fixtures (root cause: codegen `build_inline_map`, not `rename_variables`) — [codegen-emission.md](codegen-emission.md)#gap-12
- [ ] Optional chaining in codegen: 15 fixtures (HIR lacks `optional` flag) — [codegen-emission.md](codegen-emission.md)#gap-16

## P3 -- Render Divergences (1 remaining — BLOCKED)

- [ ] 1 remaining render divergence (canvas-sidebar) — BLOCKED on scope inference — [codegen-emission.md](codegen-emission.md)#gap-15
