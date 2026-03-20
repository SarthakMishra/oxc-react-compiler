# Validation & Coverage Gaps

These issues reduce conformance coverage but do not break the core compilation pipeline for patterns we do handle.

**Status summary:** Gap 5a is BLOCKED on scope inference (proven by reverted attempt). Gaps 5c+5d share a root cause with a planned fix. Gaps 5b, 5e, 6 are independently addressable.

---

## Gap 5c: False "Reassigned After Render" Errors — DONE

**Status:** FIXED. Reassignment validator rewritten with ID-based forward alias chain tracking via `validation/function_context.rs`. Result: 16 fewer bail-outs (174→158), 0 regressions. See commit `2ec7217`.

The shared module provides:
- `collect_directly_called_fe_ids`: ID-based forward alias chain tracking
- `collect_post_render_fn_ids`: Hook args, JSX props, returns + transitive fixpoint
- `has_self_shadowing`: Detects FEs whose variable name is shadowed internally

---

## Gap 5d: False "Ref Access in Render" Errors (14 fixtures → 2 remaining)

**Priority:** P2 — partially addressed, 2 false positives remain

**Current state:** Attempted using `collect_directly_called_fe_ids` from `function_context.rs` to skip checking FEs that escape render. This fixed 12 of 14 false positives but caused 3 `error.*` regressions (net -3 conformance). Reverted.

**The 3 regressions involve indirect render-time calls:**
1. `error.invalid-ref-in-callback-invoked-during-render.js` — `renderItem` called inside a `.map(item => renderItem(item))` callback. The `renderItem` FE is not in top-level `directly_called` because the call happens inside a nested arrow function.
2. `error.invalid-aliased-ref-in-callback-invoked-during-render-.js` — same pattern with aliased callback.
3. `error.capture-ref-for-mutation.tsx` — `handleKey('left')()` chained call. The inner closure returned by `handleKey` is called directly but isn't tracked as a separate FE in `directly_called`.

**What's needed:** Compute `directly_called` per-function-body, not just top-level HIR. When recursing into a directly-called FE's body, nested FEs that are themselves called within that body should also be considered render-time. This requires running `collect_directly_called_fe_ids` on each nested HIR, not just the component's top-level HIR.

**Remaining false positives (2):**
- `ref-current-aliased-no-added-to-dep.js`
- `valid-setState-in-useEffect-via-useEffectEvent-with-ref.js`

**Upstream:** `src/Validation/ValidateNoRefAccessInRender.ts`

---

## Gap 5a: False "Memoization Preservation" Errors (58 fixtures)

**Priority:** P2 — BLOCKED on scope inference

**Current state:** 58 conformance fixtures fail because we emit a false `Existing memoization could not be preserved` error and bail out, while upstream compiles them successfully.

**Attempted fix (REVERTED):** In `bbbbc1d`, replaced scope-matching with inner-scope tracking. Conformance dropped 413->385 (-28). Reverted in `4a082dc`.

**Depends on:** Scope inference improvements (Gap 11 in scope-inference.md) — HARD DEPENDENCY proven by failed attempt

---

## Gap 5b: False "Frozen Mutation" Errors (~29 fixtures)

**Priority:** P2

**Current state:** ~29 fixtures bail with frozen mutation errors. Prior fixes: SSA-versioned keys (`ca2374d`), IIFE detection, PrefixUpdate exemption, effect/callback hook lambda exemptions.

**Investigation (Phase 106):** The 29 fixtures have diverse root causes across 5 different check paths in the validator. Not a single-category fix. Attempted unfreezing rest/spread destructure elements — correct in principle but didn't fix any fixtures (the freeze comes from different paths than `collect_frozen_ids_from_destructure`).

**Key patterns:**
- 6x IIFE capture/alias patterns — IIFE detection works but aliasing effects on outer variables still trigger
- 4x switch fall-through — mutation after JSX in switch cases with fall-through
- 3x new-mutability — transitive mutation through identity/propertyload functions
- 2x method call results — `props.object.makeObject()` result wrongly inherits frozen status
- 2x rest/spread allocations — `{...rest}` or `[...arr]` creates new object, shouldn't be frozen
- 12x misc patterns (ref callbacks, loop collections, parameter mutations, etc.)

**What's needed:** Each fixture needs individual diagnosis to determine WHICH of the 5 checks (MutateFrozen effect, Freeze effect propagation, MethodCall on frozen, PropertyStore on frozen, Mutate effect on frozen) is triggering, then targeted exemption.

**Upstream:** `src/Validation/ValidateFrozenValues.ts`
**Depends on:** None, but requires per-fixture investigation

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
