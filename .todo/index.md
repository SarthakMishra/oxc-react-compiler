# Backlog Index

> Last updated: 2026-03-20 (post Phase 106, optional chaining)

Conformance: **451/1717 (26.3%)**. Render equivalence: **96% (24/25)**. All tests pass, 0 panics.

Key breakdown of diverged fixtures:
- ~244 "both compile, slots match" (output format only)
- ~628 "both compile, slots differ" (scope/memoization divergence)
- ~158 "we bail, they compile" (false bail-outs, down from 174)
- ~147 "we compile, they don't" (we over-compile -- usually fine)
- ~89 "both no memo, format diff"
- 28 silent bail-outs (compile but 0 scopes, no error)

### Lessons learned

- **Validation relaxation without scope fixes causes regressions.** Attempted relaxing `ValidatePreservedManualMemoization` -- conformance dropped 413->385. REVERTED. Validation fixes MUST be paired with scope inference improvements.
- **Under-memoization root cause:** BFS mutation propagation produces narrower ranges than upstream's abstract interpreter. Our `effective_range` approximation compensates. Fixing requires porting upstream's full abstract interpreter.
- **Validation false positives:** Reassignment validator FIXED via `function_context.rs` (16 fewer bail-outs). Ref-access validator needs per-body `directly_called` computation to avoid 3 `error.*` regressions.

## Do NOT Attempt (until prerequisites are met)

- **Gap 11: Narrow mutable ranges** -- 4 attempts reverted (96%→36% render regression each). Requires porting upstream's full aliasing effect pipeline.
- **Gap 5a: Memoization preservation validation** -- -28 conformance regression. BLOCKED on Gap 11.
- **Gap 7: Over-memoization** -- may self-resolve with Gap 11.
- **Ternary reconstruction** -- P4 cosmetic only.

## Recently Completed

- [x] Scope declaration rename fix (`is_last_assignment_in_scope`) → +8 conformance
- [x] Reassignment validator rewrite (`function_context.rs`) → 16 fewer bail-outs
- [x] LoadLocal counted as read in rename eligibility → correctness fix
- [x] Optional chaining support (HIR `optional` flag + codegen `?.`) → +6 conformance

## P1 -- Scope/Memoization Divergences (628 fixtures — BLOCKED)

- [ ] Under-memoization: ~404 fixtures — [scope-inference.md](scope-inference.md)#gap-11
- [ ] Over-memoization: ~175 fixtures — [scope-inference.md](scope-inference.md)#gap-7

## P2 -- False Bail-outs (~158 fixtures)

### Ready to implement:
- [ ] ~29 false "frozen mutation" errors — [validation-gaps.md](validation-gaps.md)#gap-5b
- [ ] 28 silent bail-outs (0 scopes, no error) — [validation-gaps.md](validation-gaps.md)#gap-6
- [ ] 28 other false bail-outs (hooks, setState, etc.) — [validation-gaps.md](validation-gaps.md)#gap-5e
- [ ] 14 false "ref access in render" — needs per-body `directly_called` — [validation-gaps.md](validation-gaps.md)#gap-5d

### BLOCKED on scope inference:
- [ ] 58 false "memoization preservation" errors

## P2 -- Output Format Divergences

- [ ] Named variable preservation: ~34 fixtures (root cause: codegen `build_inline_map`) — [codegen-emission.md](codegen-emission.md)#gap-12
- [x] ~~Optional chaining: 15 fixtures~~ → 6 fixed, 9 remaining (scope dependency paths) — [codegen-emission.md](codegen-emission.md)#gap-16

## P3 -- Render Divergences (BLOCKED)

- [ ] canvas-sidebar — BLOCKED on scope inference — [codegen-emission.md](codegen-emission.md)#gap-15
