# Next Session Plan

> Written: 2026-03-20 (post Phase 106 + optional chaining + frozen mutation investigation).

## Current Metrics

| Metric | Value |
|--------|-------|
| Conformance | 451/1717 (26.3%) |
| Render equivalence | 96% (24/25) |
| Bail-outs | 158 (down from 174) |
| E2E coverage | 95-100% across 4 Vite projects |

## Session Progress

| Change | Impact |
|--------|--------|
| Scope declaration rename fix | +8 conformance (437→445) |
| Reassignment validator rewrite | 16 fewer bail-outs |
| LoadLocal read count fix | Correctness |
| Optional chaining (HIR + codegen) | +6 conformance (445→451) |

## What's Left — Honest Assessment

All remaining tasks require significantly more effort per fixture:

### Frozen Mutation (Gap 5b, 29 fixtures) — HARD
Investigated: 29 fixtures across 5 different check paths and 6+ distinct patterns (IIFE aliases, switch fall-through, method call results, rest/spread allocations, transitive mutations, misc). Each fixture needs individual diagnosis. Not a bulk fix.

### Silent Bail-outs (Gap 6, 28 fixtures) — MEDIUM
Categorized: default params (5), Flow syntax (5), gating (4), ref-related (5), other (9). Default params are the most tractable sub-group — requires adding parameter default lowering to HIR builder.

### Ref-Access Validator (Gap 5d, 14 fixtures) — HARD
Proved to cause 3 `error.*` regressions. Needs per-function-body `directly_called` computation.

### Named Variable Preservation (Gap 12, 34 fixtures) — HARD
Root cause in codegen's `build_inline_map`, not `rename_variables`. Broad refactoring.

### BLOCKED Items
- Canvas-sidebar render divergence — scope inference (64 vs 70 slots)
- Under-memoization (404 fixtures) — requires full abstract interpreter port
- Memoization preservation (58 fixtures) — blocked on scope inference

## Recommended Next Tasks

1. **Silent bail-outs: default param patterns (5 fixtures)** — add `x = default_value` parameter lowering to `lower_function_params` in `build.rs`. Upstream converts default params to ternary: `x === undefined ? default : x`.

2. **Frozen mutation: per-fixture triage** — add debug instrumentation to identify which of the 5 checks fires for each fixture, then fix the most common check.

3. **Optional chaining remaining (9 fixtures)** — requires changes to scope dependency path tracking, not just codegen.

## Build & Test

```bash
cargo test --test conformance_tests -- --nocapture  # Conformance
cargo test                                            # All tests
cd napi/react-compiler && npx @napi-rs/cli build --release && cd ../.. && cd benchmarks && npm run render:compare  # Full validation
```
