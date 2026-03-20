# Ref Access Validator: Per-Body `directly_called` Implementation

## Overview

Fix the ref-access validator to compute `directly_called` per-function-body, not just at top level. This resolves 3 regression failures (false negatives) that prevented fixing 12 false-positive bail-outs.

**Status:** 14 false-positive bail-outs, with 12-fixture fix that regressed 3 error.* fixtures
**Goal:** +14 net conformance (fix all 14, keep the 3 errors intact)

---

## File: `validate_no_ref_access_in_render.rs`

### Change 1: Update `check_nested_ref_access()` signature (line 248)

**Before:**
```rust
fn check_nested_ref_access(
    hir: &HIR,
    outer_ref_names: &FxHashSet<String>,
    non_render_ids: &FxHashSet<IdentifierId>,
) -> bool {
```

**After:**
```rust
fn check_nested_ref_access(
    hir: &HIR,
    outer_ref_names: &FxHashSet<String>,
    non_render_ids: &FxHashSet<IdentifierId>,
    directly_called_in_parent: &FxHashSet<IdentifierId>,
) -> bool {
```

**Why:** Pass the parent scope's directly-called IDs so we can determine if the current body is render-time.

---

### Change 2: Compute `directly_called` for this body (after line 254)

**Add after line 254:**
```rust
    let directly_called_in_this_body = crate::validation::function_context::collect_directly_called_fe_ids(hir);
```

**Why:** Identify which FEs in this body are directly called at render time.

---

### Change 3: Update recursive call signature (lines 324-328)

**Before:**
```rust
                    if !non_render_ids.contains(&instr.lvalue.identifier.id)
                        && check_nested_ref_access(
                            &lowered_func.body,
                            &local_ref_names,
                            non_render_ids,
                        )
                    {
```

**After:**
```rust
                    let fe_id = instr.lvalue.identifier.id;
                    if !non_render_ids.contains(&fe_id)
                        && !directly_called_in_this_body.contains(&fe_id)
                        && check_nested_ref_access(
                            &lowered_func.body,
                            &local_ref_names,
                            non_render_ids,
                            &directly_called_in_this_body,
                        )
                    {
```

**Why:**
- Skip checking if FE is in non-render contexts (already skipped, no change)
- **NEW:** Skip checking if FE is directly called in the current body (render-time call)
- Pass down `directly_called_in_this_body` for the nested recursion

---

### Change 4: Update top-level call (lines 104-108)

**Before:**
```rust
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func } => {
                    if !non_render_ids.contains(&instr.lvalue.identifier.id)
                        && check_nested_ref_access(&lowered_func.body, &ref_names, &non_render_ids)
                    {
```

**After:**
```rust
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func } => {
                    let directly_called_top_level = crate::validation::function_context::collect_directly_called_fe_ids(hir);
                    if !non_render_ids.contains(&instr.lvalue.identifier.id)
                        && check_nested_ref_access(&lowered_func.body, &ref_names, &non_render_ids, &directly_called_top_level)
                    {
```

**Why:** Provide the top-level `directly_called` set to the recursive checker.

---

## Summary of Changes

| Location | Change | Reason |
|----------|--------|--------|
| Line 248 | Add `directly_called_in_parent` parameter | Track parent scope's direct calls |
| Line 254+ | Add `directly_called_in_this_body` computation | Identify render-time FEs in current body |
| Line 323+ | Add check `!directly_called_in_this_body.contains(&fe_id)` | Skip rendering FEs directly called here |
| Line 107 | Compute and pass `directly_called_top_level` | Provide direct-call info to recursion |

---

## Testing Strategy

### Verify 3 Regressions Are Fixed (Should Now Error)

1. `error.invalid-ref-in-callback-invoked-during-render.js` — `renderItem` directly called in `.map()` callback
2. `error.invalid-aliased-ref-in-callback-invoked-during-render-.js` — aliased ref in `.map()` callback
3. `error.capture-ref-for-mutation.tsx` — chained call `handleKey('left')()` during render

All three should emit "Cannot access refs during render" errors (expected).

### Verify 12 False-Positives Are Fixed (Should Now Pass)

Examples from 14 fixtures:
- `allow-ref-access-in-callback-passed-to-jsx-indirect.tsx` — ref access in event handler
- `ref-callback-as-prop.js` — ref callback passed to child
- Any other fixture with ref access only in non-render contexts (effects, event handlers, returned callbacks)

### Run Full Conformance Suite

```bash
cargo test --test conformance_tests -- --nocapture
```

Expected net: +14 conformance (or +12 if some other issue arises with the 2 remaining false positives).

---

## Why This Works

**The Problem:**
When checking if a FE should be skipped, the old code only checked if it was in `non_render_ids` (top-level hook/event contexts). But it missed FEs that are directly called WITHIN nested function bodies.

**The Solution:**
By computing `collect_directly_called_fe_ids()` for each nested HIR, we now identify:
- FEs directly called at render time WITHIN the nested body
- These are "render-time calls" and must have ref access validated
- Wrapping them in the `!directly_called_in_this_body` check prevents false negatives

**Example (chained call):**
```javascript
const handleKey = direction => () => {
  ref.current = ...;  // Error: directly called in render
};
handleKey('left')();  // <- This is a direct call within render
```

Before: We'd check `handleKey`'s body without recognizing that the inner FE is directly called.
After: We compute `directly_called_in_this_body` for `handleKey`'s body, find the inner FE is directly called, skip the `!directly_called_in_this_body` check, and recurse to find the error.

---

## Code Quality Notes

- No new dependencies; uses existing `collect_directly_called_fe_ids` from `function_context.rs`
- Maintains current logic for `non_render_ids` (unchanged)
- Preserves the narrow `collect_non_render_callback_ids()` (do NOT switch to `collect_post_render_fn_ids()`)
- Per-body computation adds O(body_size) cost per recursion level, acceptable for typical nesting depth (3-5)
