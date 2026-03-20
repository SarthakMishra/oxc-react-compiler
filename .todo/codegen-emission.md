# Codegen Emission Gaps

Issues in `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` and related files.

Completed: Gaps 1-5, 6b, 7, 8, 9, 9b, 11, 13, 14. Remaining: Gap 6 (ternary reconstruction, P4), Gap 12 (named variable preservation, partially fixed), Gap 15 (1 render divergence), Gap 16 (optional chaining).

---

## Gap 6: Ternary Expression Reconstruction

**Priority:** P4 — functionally correct but produces `if/else` instead of `?:` for expression-position ternaries

**Current state:** `Terminal::Ternary` is converted to `ReactiveTerminal::If` and emitted as `if/else`. The `result: Option<Place>` field is ignored.

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Depends on:** None

---

## Gap 12: Named Variable Preservation

**Priority:** P2 — ~56 remaining fixtures after Phase 106 partial fix

**Progress:**
- Phase 106: `is_last_assignment_in_scope` check in `can_rename_scope_decl` → +8 conformance
- Phase 106: `LoadLocal` now counted as read in rename eligibility → correctness fix (no conformance change)

**Remaining (~34 fixtures):** Investigation confirmed the remaining `const t0 vs const <name>` divergences do NOT come from the `rename_variables` pass — that pass now correctly blocks renames for variables with reads or non-last-position assignments. The 34 remaining cases come from **codegen's `build_inline_map`** which creates temps for intermediate expressions. When a scope declaration's identifier flows through `StoreLocal → LoadLocal → CallExpression arg`, the codegen inlines the value through a temp rather than preserving the original variable name.

Only 4 of the 34 have the `} ; const t0` post-scope rename pattern. The other 30 have temps in different positions, confirming the root cause is the inline map, not `rename_variables`.

**What's needed:**
- Study how codegen's `build_inline_map` decides to inline vs emit through named variables
- Upstream's `CodegenReactiveFunction.ts` preserves original names by tracking which intermediates correspond to user-declared variables
- The fix requires teaching `build_inline_map` to NOT inline through a temp when the source is a named scope declaration

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`

---

## Gap 15: Remaining Render Divergence (1 fixture) — BLOCKED

**Priority:** P3 — BLOCKED on scope inference (Gap 11)

**Current state:** canvas-sidebar (1/25) shows render divergence. Investigation confirmed this is a scope inference issue, NOT a codegen bug: OXC produces 64 cache slots with sentinel checks while Babel produces 70 slots with dependency-based checks. The memoization strategy is fundamentally different.

**Depends on:** Gap 11 (under-memoization / full abstract interpreter port)

---

## Gap 16: Optional Chaining in Codegen (15 fixtures)

**Priority:** P2 — 15 conformance fixtures diverge

**Current state:** Our HIR doesn't carry an `optional: bool` flag on `CallExpression`, `MethodCall`, or `PropertyLoad`. So `foo?.bar` is emitted as `foo.bar` and `foo?.(args)` is emitted as `foo(args)`.

**What's needed:**
- Add `optional: bool` to `CallExpression`, `MethodCall`, `PropertyLoad`, and `ComputedLoad` in HIR types
- Propagate the optional flag from OXC AST during HIR building
- Use it in codegen to emit `?.` syntax

**Upstream:** The upstream React compiler decomposes optional chains into branches but re-synthesizes `?.` in codegen output.
**Depends on:** None (structural HIR change)
