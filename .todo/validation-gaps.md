# Validation & Coverage Gaps

These issues reduce conformance coverage but do not break the core compilation pipeline for patterns we do handle.

**Status summary:** Gap 5a is BLOCKED on scope inference (proven by reverted attempt). Gaps 5c+5d share a root cause with a planned fix. Gaps 5b, 5e, 6 are independently addressable.

---

## Shared Root Cause: Broken Non-Render Function Detection (Gaps 5c + 5d = ~40 fixtures)

**Priority:** P2 — highest-impact validation fix available

**Root cause:** Both `validate_no_ref_access_in_render` (14 false positives) and `validate_locals_not_reassigned_after_render` (26 false positives) need to determine whether a `FunctionExpression` executes during render or after. The current `render_only_fns` detection in the reassignment validator is broken — it always returns empty because the `id_to_fn_var` mapping fails to track function identifiers through our named-lvalue HIR's LoadLocal→CallExpression chains. The ref validator's `collect_non_render_callback_ids` is too narrow (only recognizes `useEffect`/`useCallback`/`useImperativeHandle`).

**Planned fix:** Create a shared `validation/function_context.rs` module with `collect_post_render_fn_ids()` that:

1. **Invert the approach**: instead of identifying render-only functions (hard), identify post-render functions (easier)
2. **Initial seeding**: Mark FE IDs that are arguments to any hook call (`is_hook_name`), JSX event handler props (`onXxx`), JSX `ref` props, and return values
3. **Alias propagation**: Follow LoadLocal/StoreLocal chains (up to 10 hops) to resolve FE IDs through SSA temporaries
4. **Transitive fixpoint**: If post-render FE A's body calls named variable holding FE B, then B is also post-render. Iterate until stable.

**Apply to both validators:**
- **Reassignment**: Only flag reassignments inside post-render FEs (invert current logic)
- **Ref access**: Skip ref.current checks inside post-render FEs (existing logic, but with wider/correct set)

**Files to create/modify:**
- CREATE: `crates/oxc_react_compiler/src/validation/function_context.rs`
- MODIFY: `crates/oxc_react_compiler/src/validation/mod.rs`
- MODIFY: `crates/oxc_react_compiler/src/validation/validate_locals_not_reassigned_after_render.rs`
- MODIFY: `crates/oxc_react_compiler/src/validation/validate_no_ref_access_in_render.rs`

**Key design decisions:**
- Non-hook utility functions (`invoke`, `foo`) are NOT post-render — upstream treats them as synchronous render-time callers
- ALL `use*` hook arguments are post-render (not just specific hooks)
- Return values escape and are post-render
- Async functions remain handled separately (always post-render, existing logic)

**Risk:** LOW-MEDIUM. May regress 1-2 `error.*` fixtures (where upstream flags patterns requiring value-flow analysis beyond our capability). Must verify with conformance tests.

**Upstream:**
- `src/Validation/ValidateLocalsNotReassignedAfterRender.ts`
- `src/Validation/ValidateNoRefAccessInRender.ts`

---

## Gap 5a: False "Memoization Preservation" Errors (58 fixtures)

**Priority:** P2 — BLOCKED on scope inference

**Current state:** 58 conformance fixtures fail because we emit a false `Existing memoization could not be preserved` error and bail out, while upstream compiles them successfully.

**Attempted fix (REVERTED):** In `bbbbc1d`, replaced scope-matching with inner-scope tracking. Conformance dropped 413->385 (-28). Reverted in `4a082dc`.

**Depends on:** Scope inference improvements (Gap 11 in scope-inference.md) — HARD DEPENDENCY proven by failed attempt

---

## Gap 5b: False "Frozen Mutation" Errors (~29 fixtures)

**Priority:** P2

**Current state:** ~29 fixtures bail with frozen mutation errors. Recent fixes: SSA-versioned keys (`ca2374d`), IIFE detection, PrefixUpdate exemption, effect/callback hook lambda exemptions.

**What's needed:**
- Audit remaining false positives against upstream `ValidateFrozenValues`
- Check if mutable range computation is still too narrow for remaining cases

**Upstream:** `src/Validation/ValidateFrozenValues.ts`
**Depends on:** None

---

## Gap 5e: Other False Bail-outs (28 fixtures)

**Priority:** P2

**Breakdown:**
- 8x "Cannot reassign variables declared outside of the component"
- 3x "Hooks may not be referenced as normal values"
- 3x "Cannot call setState during render"
- 2x "setState is called directly inside useEffect"
- 1x "useMemo called conditionally"
- 11x other

**What's needed:** Audit each category against its upstream validation pass.

---

## Gap 6: Silent Bail-outs (28 fixtures)

**Priority:** P2

**Current state:** 28 fixtures produce compiled output but with 0 reactive scopes and no error message. Categorized as: default param patterns (5), Flow syntax (5), gating patterns (4), ref-related (5), other (9).

**What's needed:**
- Fix the most common remaining categories (default params, Flow component syntax)

**Depends on:** None
