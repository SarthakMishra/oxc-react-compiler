# Validation & Coverage Gaps

These issues reduce conformance coverage but do not break the core compilation pipeline for patterns we do handle.

---

## Gap 5a: False "Memoization Preservation" Errors (58 fixtures)

**Priority:** P2 -- BLOCKED on scope inference

**Current state:** 58 conformance fixtures fail because we emit a false `Existing memoization could not be preserved` error and bail out, while upstream compiles them successfully.

**Attempted fix (REVERTED):** In `bbbbc1d`, replaced scope-matching with inner-scope tracking in `validate_preserved_manual_memoization.rs`. This relaxed the validation but caused a net regression: conformance dropped 413->385 (-28), "we compile, they don't" increased 138->171 (+33). The problem: relaxing validation without fixing scope inference means we compile programs incorrectly (wrong memoization) instead of safely bailing out. Reverted in `4a082dc`.

**What's needed:**
- Fix scope inference FIRST (under-memoization root cause: `last_use_map` too wide)
- Only then revisit validation relaxation -- once scopes are correct, relaxing validation will produce correct output
- The inner-scope tracking approach may still be valid after scope inference is fixed

**Upstream:** `src/Validation/ValidatePreservingMemoization.ts`
**Depends on:** Scope inference improvements (Gap 11 in scope-inference.md) -- HARD DEPENDENCY proven by failed attempt

---

## Gap 5b: False "Frozen Mutation" Errors (26 fixtures)

**Priority:** P2

**Current state:** 26 fixtures fail with false `This value cannot be modified` errors. Recent fix in `ca2374d` (SSA-versioned keys) may have addressed some but likely not all.

**What's needed:**
- Audit remaining false positives against upstream `ValidateFrozenValues`
- Check if mutable range computation is too narrow

**Upstream:** `src/Validation/ValidateFrozenValues.ts`
**Depends on:** None

---

## Gap 5c: False "Reassigned After Render" Errors (16 fixtures)

**Priority:** P2

**Current state:** 16 fixtures bail with `Local variable "x" is assigned during render but reassigned` errors that upstream does not produce.

**What's needed:**
- Audit the reassignment-after-render validation logic
- Likely need to check if the reassignment is inside a callback/effect (which is allowed)

**Upstream:** `src/Validation/ValidateNoRefAccessInRender.ts` (or related)
**Depends on:** None

---

## Gap 5d: False "Ref Access in Render" Errors (14 fixtures)

**Priority:** P2

**Current state:** 14 fixtures bail with `Cannot access refs during render` errors that upstream does not produce.

**What's needed:**
- Audit ref-access detection against upstream
- Check if we correctly identify effect/callback contexts where ref access is allowed

**Upstream:** `src/Validation/ValidateNoRefAccessInRender.ts`
**Depends on:** None

---

## Gap 5e: Other False Bail-outs (28 fixtures)

**Priority:** P2

**Breakdown:**
- 8x "Cannot reassign variables declared outside of the component"
- 6x "Local variable y is assigned during render but reassigned"
- 3x "Hooks may not be referenced as normal values"
- 2x "Cannot call setState during render"
- 2x "setState is called directly inside useEffect"
- 7x other

**What's needed:** Audit each category against its upstream validation pass.

---

## Gap 6: Silent Bail-outs (63 fixtures)

**Priority:** P2

**Current state:** 63 conformance fixtures produce compiled output but with 0 reactive scopes and no error message, while upstream successfully compiles them with memoization. Categories likely include:
- Try/catch blocks not fully lowered into HIR
- Sequence expressions (comma operator) not handled
- Other HIR lowering gaps

**What's needed:**
- Categorize the 63 silent failures by looking at the input patterns
- Fix the most common HIR lowering gap first

**Depends on:** None
