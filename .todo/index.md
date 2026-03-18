# Backlog Index

> Last updated: 2026-03-18

The compiled output is structurally 93.8% correct (scope/dependency analysis is mostly right), but code emission bugs in `codegen.rs` break 14/16 render benchmarks. Fixing P0 codegen issues is the single highest-leverage effort.

## P0 -- Codegen Emission Bugs (Blocks All Renders)

- [ ] Fix duplicate variable declarations in scope emission — [codegen-emission.md](codegen-emission.md)#gap-1-duplicate-declarations-in-codegen_scope
- [ ] Fix useState/hook destructuring codegen — [codegen-emission.md](codegen-emission.md)#gap-2-hook-destructuring-codegen
- [ ] Fix variable ordering / use-before-declare in guards — [codegen-emission.md](codegen-emission.md)#gap-3-variable-ordering-use-before-declare
- [ ] Fix scope body to use assignments for pre-declared variables — [codegen-emission.md](codegen-emission.md)#gap-4-assignment-vs-re-declaration-for-pre-declared-variables

## P2 -- Validation & Coverage Gaps

- [ ] Fix 208 false bail-outs (over-conservative validation) — [validation-gaps.md](validation-gaps.md)#gap-5-false-bail-outs-208-fixtures
- [ ] Fix 66 silent bail-outs (missing compilable patterns) — [validation-gaps.md](validation-gaps.md)#gap-6-silent-bail-outs-66-fixtures

## P3 -- Scope Inference

- [ ] Fix over-memoization / slot count divergence — [scope-inference.md](scope-inference.md)#gap-7-over-memoization-slot-count-divergence
