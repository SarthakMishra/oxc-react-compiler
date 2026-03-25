# Implementation Journal

> Chronological log of implementation sessions. Each file covers ~50 entries before splitting.

| File             | Entries | Phases | Notes                                                                  |
| ---------------- | ------- | ------ | ---------------------------------------------------------------------- |
| [001.md](001.md) | 145     | 1–145  | Full compiler: HIR foundation → 26.2% conformance (450/1717). Recent: Stage 3b scope inference investigation (no code changes — attempted is_allocating_instruction fix caused -5 net, reverted; root cause is union-find over-merging via last_use_map, not too-few sentinel scopes; deficit distribution and we-compile-they-dont breakdown fully documented), Stage 1d Phase 1 lazy scope declaration placement (+6, removed eager collect_all_scope_declarations pre-pass), Fix validate_no_ref_access_in_render post-inline_load_local_temps (+1, name+type fallback detection), Stage 3a comprehensive slot diff and failure categorization (investigation only — 1274 divergences in 5 buckets), Stage 4e-B hooks-in-loop detection via Terminal::Branch (+1), Stage 4e-A upstream error fixture bail-outs (+7), Stage 4d frozen-mutation false negative fixes (+9), Stage 4c Todo error detection (+15), Stage 2a/2b/2c bail-out fixes, Stage 1c codegen fixes, mutation range propagation fixes (+93 fixtures) |

## Archive

| File                           | Notes                              |
| ------------------------------ | ---------------------------------- |
| [2026-03-11.md](2026-03-11.md) | Initial project planning session   |
