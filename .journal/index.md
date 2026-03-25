# Implementation Journal

> Chronological log of implementation sessions. Each file covers ~50 entries before splitting.

| File             | Entries | Phases | Notes                                                                  |
| ---------------- | ------- | ------ | ---------------------------------------------------------------------- |
| [001.md](001.md) | 142     | 1–142  | Full compiler: HIR foundation → 25.8% conformance (443/1717). Recent: Stage 3a comprehensive slot diff and failure categorization (investigation only — 1274 divergences categorized into 5 buckets, key findings: PromoteUsedTemporaries missing, declaration hoisting differences, 688 slot-diffs dominate), Stage 4e-B hooks-in-loop detection via Terminal::Branch (+1, all loops lower to Branch+Goto in our HIR), Stage 4e-A upstream error fixture bail-outs (+7, hoisted function decls in dead blocks, fbt params, default-param arrow expressions, catch destructuring, hook spread args), Stage 4d frozen-mutation false negative fixes (+9, phi propagation, alias tracking, property-load/iterator freeze, name-based fallback), Stage 4c Todo error detection (+15, try-finally, computed keys, value-blocks-in-try, throw-in-try, fbt locals), Stage 2a extended investigation (69 bail-outs remaining, full quadrant analysis), Stage 2c _exp suffix fix, Stage 2a/2b bail-out fixes (+1), Stage 1c codegen fixes — return undefined stripping (+5), temp renumbering (+2), expected file rebaseline with compilationMode:"all" (+9 net), validation fixes, mutation range propagation fixes (+93 fixtures) |

## Archive

| File                           | Notes                              |
| ------------------------------ | ---------------------------------- |
| [2026-03-11.md](2026-03-11.md) | Initial project planning session   |
