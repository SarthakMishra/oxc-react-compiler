# False-Positive Bail-Outs (221 fixtures)

We reject functions that upstream compiles successfully. Each fix is
a direct 1:1 fixture gain. This is the highest-ROI category because
bail-out fixes are self-contained: remove the false positive, fixture passes.

Current breakdown (from conformance at 415/1717):
- Silent bail-outs (no error, 0 scopes): 66
- Preserve-memo validation: 54
- Frozen-mutation: 44
- Locals-reassigned-after-render: ~26
- Ref-access-during-render: 13
- Global-reassignment: 8
- Hooks-as-values: 3
- setState-during-render: 2
- Other (conditional hooks, etc.): 5

---

## useMemo/useCallback Argument Count -- DONE

~~**Diagnostic:** "useMemo/useCallback requires exactly 2 arguments but received 1"~~

**Completed**: Removed the argument-count check from `validate_use_memo.rs`.
`useMemo(fn)` without a deps array is accepted. Upstream does NOT validate
argument count. File: `crates/oxc_react_compiler/src/validation/validate_use_memo.rs`.

---

## Frozen-Mutation False Positives (44 fixtures)

**Diagnostic:** "This value cannot be modified. Modifying a value used
previously in JSX is not allowed."

**Root cause:** Our `validate_no_mutation_after_freeze.rs` uses a hybrid
effects + instruction checker (Phase 77 rewrite) plus several hardening
fixes (method allowlist, ref exclusion, call-conditional exclusion). Despite
these improvements (down from 158 to 44), significant false positives remain
because we lack mutable-range-based freeze determination.

**Upstream file:** `src/Validation/ValidateNoSetStateInRender.ts` uses
mutable ranges from `InferMutableRanges.ts`. The key check is:
`place.identifier.mutableRange.end < scope.range.start` (frozen if
mutable range ended before scope began).

**Fix strategy:** Wire the mutation aliasing pass output (mutable ranges
from `infer_mutation_aliasing_ranges.rs`) into the validator. Replace
remaining name-based freeze heuristics with range-based checks.

**Improvements already made:**
- Phase 77: Hybrid effects+instruction checker rewrite (158 -> ~104)
- Phase 80+: Method signature allowlist for frozen-mutation check
- Phase 83: Ref value exclusion from frozen-mutation detection
- Phase 85: Method allowlist + call-conditional exclusion in inner frozen check
- Net result: 158 -> 44 fixtures

**Key files:**
- `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs`
- `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`
- `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`

---

## Preserve-Memo Validation False Positives (54 fixtures)

**Diagnostic:** "Existing memoization could not be preserved."

**Root cause:** `validate_preserved_manual_memoization` (Pass 61) incorrectly
rejects fixtures that upstream accepts. When our scope analysis differs from
upstream's, the validator sees scopes that don't match the manual memo sites
and flags them. This is the largest single bail-out category now.

**Upstream:** `ValidatePreservedManualMemoization.ts`

**Fix strategy:**
1. Audit the preserve-memo validator against upstream
2. Some fixtures will fix themselves as scope analysis improves
3. Consider relaxing the validator to accept "superset" memoization

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_preserved_manual_memoization.rs`

---

## Locals-Reassigned False Positives (~26 fixtures)

**Diagnostic:** "Local variable X is assigned during render but reassigned
inside a nested function"

**Root cause:** `validate_locals_not_reassigned_after_render.rs` flags
reassignments inside closures/effect callbacks that upstream allows.

**Improvements already made:**
- Phase 73: Skip locals-reassigned check for render-only closures
- Phase 74: Distinguish reassignment from shadowing
- Reduced from 30 to ~26 fixtures

**Remaining work:** Trace callback identity through StoreLocal chains
to detect effect-callback patterns. Upstream is more precise about
distinguishing effect callbacks (where reassignment is fine) from
render-time closures (where it matters).

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_locals_not_reassigned_after_render.rs`

---

## Ref-Access False Positives (13 fixtures)

**Diagnostic:** "Cannot access refs during render"

**Improvements already made:**
- Phase 72: Skip ref-access check for effect/event handler callbacks
- Reduced from 18 to 13 fixtures

**Remaining work:** Indirect function calls, SSR-mode ref access rules,
ref access through aliased function references.

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_no_ref_access_in_render.rs`

---

## Global-Reassignment False Positives (8 fixtures)

**Diagnostic:** "Cannot reassign variables declared outside of the
component/hook"

**Improvements already made:**
- Added JSX event handler filtering, effect hook callback detection,
  useCallback body detection
- Reduced from 15 to 8 fixtures

**Remaining work:** Indirect callback patterns (callback stored in a
variable before being passed to an effect hook).

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_no_global_reassignment.rs`

---

## setState False Positives (2 remaining)

**Diagnostic:** "Cannot call setState during render"

**Improvements already made:**
- Gated name heuristic behind `enableTreatSetIdentifiersAsStateSetters`
- Added useState destructure pre-pass
- Reduced from 14 to 2 fixtures

**Remaining work:** Conditional setState detection through lambda chains.
Also 2 "setState in useEffect" false positives (synchronous setState
detection is too broad).

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_no_set_state_in_render.rs`

---

## Hooks-as-Values False Positives (3 fixtures)

**Diagnostic:** "Hooks may not be referenced as normal values"

**Root cause:** Over-flagging hook references in non-call positions.

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_hooks_usage.rs`
