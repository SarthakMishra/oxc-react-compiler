# Implementation Journal

> Chronological log of implementation sessions. Each file covers ~50 entries before splitting.

| File             | Entries | Phases | Notes                                                                  |
| ---------------- | ------- | ------ | ---------------------------------------------------------------------- |
| [001.md](001.md) | 98      | 1–98   | Full compiler: HIR foundation → 25.3% conformance. Recent: remove last_use_map (Step 2, -7 matched but distribution improved), scope inference end-clamping fix + PropagateScopeDependenciesHIR pre-pass, MethodCall receiver MutateTransitiveConditionally effect, call-as-allocating mutable-range refinement (+5 conformance, 404->409) |

## Archive

| File                           | Notes                              |
| ------------------------------ | ---------------------------------- |
| [2026-03-11.md](2026-03-11.md) | Initial project planning session   |
