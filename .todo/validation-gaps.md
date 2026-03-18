# Validation & Coverage Gaps

These issues reduce conformance coverage but do not break the core compilation pipeline for patterns we do handle.

---

## Gap 5a: False "Memoization Preservation" Errors ~~(58 fixtures)~~ (4 remaining) -- MOSTLY FIXED

**Priority:** P3 (reduced from P2)

~~**Current state:** 58 conformance fixtures fail because we emit a false `Existing memoization could not be preserved` error and bail out, while upstream compiles them successfully.~~

**Completed (Phase 94):** Replaced overly strict scope-matching check (start_scope != finish_scope) with inner-scope tracking. The validation now checks whether a reactive scope exists between StartMemoize and FinishMemoize, or whether FinishMemoize is inside a scope. This reduced false bail-outs from 58 to 4.

**Remaining (4 fixtures):** These are cases where no scope was created between Start/Finish. Likely caused by scope inference gaps (the computation is not reactive enough to warrant a scope).

**Remaining (28 error fixtures):** Upstream's `validateInferredDep` + `compareDeps` dependency comparison check is not implemented. These fixtures correctly bail in upstream because inferred deps don't match manual deps, but we can't detect this. Added to known-failures. Implementing the dependency comparison requires porting the temporaries/ManualMemoDependency infrastructure.

**Upstream:** `src/Validation/ValidatePreservedManualMemoization.ts`
**Rust module:** `crates/oxc_react_compiler/src/validation/validate_preserved_manual_memoization.rs`

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
