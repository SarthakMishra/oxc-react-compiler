# Reactive Scopes Gaps

> The reactive scope passes construct scope boundaries, align them, merge overlapping
> scopes, and propagate dependencies. Most functions exist but one critical stub
> prevents correct scope terminal insertion.

---

## Gap 1: build_reactive_scope_terminals_hir Is a Stub

**Upstream:** `packages/babel-plugin-react-compiler/src/ReactiveScopes/BuildReactiveScopeTerminalsHIR.ts`
(~300 lines)
**Current state:** `reactive_scopes/prune_scopes.rs` function `build_reactive_scope_terminals_hir`
(lines 636-654) collects scope ranges but does nothing with them (line 654: `let _ = scope_ranges;`).
This is the pass that converts scope annotations on identifiers into actual `Terminal::Scope`
nodes in the CFG, which is required for `build_reactive_function` to wrap scoped instructions
in `ReactiveScopeBlock` nodes.
**What's needed:**
- For each ReactiveScope, find which blocks contain instructions in that scope's range
- Split blocks at scope boundaries: if a block has instructions both inside and outside a
  scope, split it into two blocks
- Insert `Terminal::Scope { scope, block, fallthrough }` terminals to wrap the scoped
  blocks
- Handle nested scopes (scope A contains scope B)
- Handle scopes that span multiple blocks (need to identify the entry/exit blocks and
  restructure the CFG)
- This is the most structurally complex pass because it modifies the CFG topology
**Depends on:** The alignment and merge passes (38-42) must run first to ensure scopes
are properly aligned and merged before terminal insertion
