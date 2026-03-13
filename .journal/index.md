# Implementation Journal

> Chronological log of implementation sessions. Each file covers ~50 entries before splitting.

| File             | Entries | Phases | Notes                                                                  |
| ---------------- | ------- | ------ | ---------------------------------------------------------------------- |
| [001.md](001.md) | 37      | 1–37   | Full compiler: HIR foundation, BuildHIR, SSA, optimization, inference, reactive scopes, validation, pipeline, lint rules, testing, end-to-end wiring, structured diagnostic categories, source map exposure, codegen correctness & memoization pipeline foundation, E2E dual-mode rendering tests, sprout eval harness, benchmark fixture pipeline, benchmark harness v2 with timed NAPI, correctness analysis, CI pipeline, error diagnostic full coverage, post-codegen semantic validation, real-world benchmark fixtures, README correctness scoring, Babel differential analysis and snapshots, headless render comparison, P0 critical bug fixes (destructured params + dependency filter), conformance rate 13.4% → 21.8% (TS type stripping, validation bail-out, zero-cache-slot skip, divergence analysis), conformance 81→84 (exhaustive operand walk, ForIn/ForOf codegen, normalization), conformance hardening (import sorting, known-failure triage, divergence analysis — 86/1717, 0 unexpected), clippy hardening + OutputMode::Lint + upstream error matching + hooks-as-values validation (86→255/1717, 5.0% → 14.8%), recursive temp use-counting for cross-scope expression inlining (304/1717, foundational for Gap 2/5/6), JSX syntax preservation in codegen (emit `<div>{count}</div>` not `_jsx(...)`, 23 snapshots updated), sentinel scope infrastructure (is_allocating field + helper, scope creation gated pending P2 validation gaps, 304/1717 unchanged) |

## Archive

| File                           | Notes                              |
| ------------------------------ | ---------------------------------- |
| [2026-03-11.md](2026-03-11.md) | Initial project planning session   |
