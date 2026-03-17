# Implementation Journal

> Chronological log of implementation sessions. Each file covers ~50 entries before splitting.

| File             | Entries | Phases | Notes                                                                  |
| ---------------- | ------- | ------ | ---------------------------------------------------------------------- |
| [001.md](001.md) | 85      | 1–85   | Full compiler: HIR foundation → 25.3% conformance. Phases 79–85: scope-aware stable IdentifierIds (revert + re-implement with scope stack), param-ID reactive seeding with mutable value gate (silent bail-outs 94→70), named instruction lvalues + cross-scope LoadLocal inlining (same-slot +48), post-SSA LoadLocal temp inlining pass (+35 conformance, new high-water 437), hook hoisting via scope splitting (silent bail-outs 157→66), conformance hardening (free variable exclusion, DeclareLocal hoisting, method allowlist, ref exclusion: 404→417), test infrastructure (silent bail-out listing, slot diff distribution, diff context) |

## Archive

| File                           | Notes                              |
| ------------------------------ | ---------------------------------- |
| [2026-03-11.md](2026-03-11.md) | Initial project planning session   |
