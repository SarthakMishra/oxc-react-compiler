# Implementation Journal

> Chronological log of implementation sessions. Each file covers ~50 entries before splitting.

| File             | Entries | Phases | Notes                                                                  |
| ---------------- | ------- | ------ | ---------------------------------------------------------------------- |
| [001.md](001.md) | 147     | 1–147  | Full compiler: HIR foundation → 26.0% conformance (447/1717). Recent: Stage 1d/2b codegen polish — dynamic gating directive parsing, empty catch codegen, computed key bail-out removal, const/let/var keyword selection (+6, 441→447), Stage 4e-D for-loop-in-try detection (Terminal::For structured lowering) and file-level bail propagation fix (ANY_FUNCTION_BAILED thread-local, +3), Stage 3b scope inference investigation (no code changes — attempted is_allocating_instruction fix caused -5 net, reverted; root cause is union-find over-merging via last_use_map), Stage 1d Phase 1 lazy scope declaration placement (+6), Fix validate_no_ref_access_in_render post-inline_load_local_temps (+1), Stage 3a comprehensive slot diff and failure categorization (investigation only), Stage 4e-B hooks-in-loop detection via Terminal::Branch (+1), Stage 4e-A upstream error fixture bail-outs (+7), Stage 4d frozen-mutation false negative fixes (+9), Stage 4c Todo error detection (+15), Stage 2a/2b/2c bail-out fixes, Stage 1c codegen fixes, mutation range propagation fixes (+93 fixtures) |

## Archive

| File                           | Notes                              |
| ------------------------------ | ---------------------------------- |
| [2026-03-11.md](2026-03-11.md) | Initial project planning session   |
