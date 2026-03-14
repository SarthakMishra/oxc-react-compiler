# False-Positive Bail-Outs (256 fixtures)

We reject functions that upstream compiles successfully. Each fix is
a direct 1:1 fixture gain. This is the highest-ROI category because
bail-out fixes are self-contained: remove the false positive, fixture passes.

## Frozen-Mutation False Positives (162 fixtures)

**Diagnostic:** "This value cannot be modified. Modifying a value used
previously in JSX is not allowed."

**Root cause:** Our `validate_no_mutation_after_freeze.rs` uses name-based
tracking to determine which values are "frozen" (immutable after first use
in JSX/hook call). Upstream uses `InferMutableRanges` output -- a value is
frozen only when its mutable range has ended. Our heuristic over-freezes
values that upstream's alias analysis considers still mutable.

**Upstream file:** `src/Validation/ValidateNoSetStateInRender.ts` uses
mutable ranges from `InferMutableRanges.ts`. The key check is:
`place.identifier.mutableRange.end < scope.range.start` (frozen if
mutable range ended before scope began).

**Fix strategy:** Wire the mutation aliasing pass output (mutable ranges
from `infer_mutation_aliasing_ranges.rs`) into the validator. Replace
name-based freeze heuristics with range-based checks. The BFS graph
rewrite and `refine_effects` phase are already done -- what's missing
is exposing mutable range data to the validator.

**Implementation:**
1. After `infer_mutation_aliasing_ranges` runs, export a map:
   `IdentifierId -> MutableRange { start: InstructionId, end: InstructionId }`
2. Thread this through the pipeline to `validate_no_mutation_after_freeze`
3. Replace `frozen_values: FxHashSet<String>` with range-based checks:
   a value is frozen at instruction I if `mutable_range.end < I`
4. Keep the existing name-based tracking as a fallback for values not
   covered by the aliasing pass

**Key files:**
- `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs`
- `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`
- `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`

**Sample fixtures (all should compile, we bail):**
- `allow-mutating-ref-in-callback-passed-to-jsx.tsx`
- `allow-ref-access-in-effect.js`
- `call-with-independently-memoizable-arg.js`
- `component.js`
- `dependencies.js`

## Frozen-Mutation False Positive on Hooks Without JSX

**Diagnostic:** "This value cannot be modified. Modifying a value used
previously in JSX is not allowed."

**Root cause:** The `validate_no_mutation_after_freeze` pass pre-freezes
component params and hook arguments at function entry (added in Phase 68).
In hooks that do NOT contain any JSX, this causes false positives: param
mutations that upstream considers fine are flagged because our validator
freezes params unconditionally, whereas upstream only freezes values after
they flow into JSX or a hook call. This surfaces as a pre-existing test
regression in `codegen_valid_hook` (the test expects compilation to
succeed but we bail out with a frozen-mutation error).

**Upstream:** `src/Validation/ValidateNoSetStateInRender.ts` -- freeze is
based on mutable range expiry from `InferMutableRanges.ts`, not on
unconditional param pre-freeze. A hook param is only frozen after its
mutable range ends, which for simple mutations before any JSX usage means
it stays mutable.

**Fix strategy:** Either:
1. Gate param pre-freeze on whether the function contains JSX (quick fix), or
2. Wire actual mutable range data from `infer_mutation_aliasing_ranges.rs`
   into the validator (correct long-term fix, aligns with the main
   frozen-mutation gap above).

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_no_mutation_after_freeze.rs`
- `crates/oxc_react_compiler/tests/pass_unit_tests.rs` (codegen_valid_hook test)

---

## Locals-Reassigned False Positives (30 fixtures)

**Diagnostic:** "Local variable X is assigned during render but reassigned
inside a nested function"

**Root cause:** `validate_locals_not_reassigned_after_render.rs` flags
reassignments inside closures/effect callbacks that upstream allows. Upstream
is more precise about distinguishing effect callbacks (where reassignment is
fine) from render-time closures (where it matters).

**Upstream file:** `src/Validation/ValidateLocalsNotReassignedAfterRender.ts`

**Fix strategy:** The upstream validation only fires when a variable is:
1. Assigned during render AND
2. Reassigned in a function that is NOT an effect callback

Our implementation likely fails to recognize effect callbacks passed through
intermediate variables or method calls. Need to trace callback identity
through StoreLocal chains to detect effect-callback patterns.

**Sample fixtures:**
- `capturing-func-alias-receiver-mutate.js`
- `context-variable-reassigned-two-lambdas.js`
- `declare-reassign-variable-in-closure.js`
- `lambda-reassign-primitive.js`
- `repro-hoisting.js`

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_locals_not_reassigned_after_render.rs`

## Ref-Access False Positives (18 fixtures)

**Diagnostic:** "Cannot access refs during render"

**Root cause:** `validate_no_ref_access_in_render.rs` detects ref access
in contexts where upstream allows it (effect callbacks, event handlers,
callbacks passed to JSX). Our implementation likely fails to trace
ref access through indirect function calls or doesn't recognize all
valid "non-render" contexts.

**Upstream file:** `src/Validation/ValidateNoRefAccessInRender.ts`

**Fix strategy:** Audit the ref-access validation against upstream. Key
patterns to check:
- Ref access inside callbacks passed to JSX props (event handlers)
- Ref access inside useEffect/useCallback bodies
- Ref access through aliased function references
- SSR-mode ref access rules (4 SSR fixtures in this list)

**Sample fixtures:**
- `allow-mutating-ref-property-in-callback-passed-to-jsx.tsx`
- `ref-current-not-added-to-dep.js`
- `ssr/ssr-use-reducer.js`

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_no_ref_access_in_render.rs`

## useMemo/useCallback Argument Count (17 fixtures)

**Diagnostic:** "useMemo/useCallback requires exactly 2 arguments but received 1"

**Root cause:** `validate_use_memo.rs` rejects `useMemo(fn)` and
`useCallback(fn)` calls with only 1 argument (no deps array). Upstream
allows this -- a missing deps array means "recompute every render" and
is valid React code.

**Upstream file:** `src/Validation/ValidateUseMemo.ts` -- upstream does
NOT reject missing deps lists. It only validates the deps array if one
IS provided.

**Fix strategy:** Remove the argument-count check from `validate_use_memo.rs`.
`useMemo(fn)` without a deps array should be accepted. The compiler should
still compile these functions -- they just won't benefit from memoization
of that particular useMemo call.

**This is likely a 5-line fix yielding 17 fixtures.**

**Sample fixtures:**
- `useMemo-with-optional.js`
- `preserve-memo-validation/useCallback-with-no-depslist.ts`
- `preserve-memo-validation/useMemo-with-no-depslist.ts`
- `useCallback-set-ref-nested-property.js`
- `repro.js`

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_use_memo.rs`

## Global-Reassignment False Positives (15 fixtures) -- Partially Fixed

**Diagnostic:** "Cannot reassign variables declared outside of the
component/hook"

**Improvements made:** Added JSX event handler filtering, effect hook
callback detection (useEffect/useLayoutEffect/useInsertionEffect first arg),
and useCallback body detection to `validate_no_global_reassignment.rs`.
Eliminated unnecessary clones and fixed dead allocation. The validator now
builds a set of "safe callback" function IDs and skips global reassignment
errors inside those functions.

**Upstream file:** `src/Validation/ValidateNoGlobalReassignment.ts`

**Remaining work:** Some fixtures may still fail due to indirect callback
patterns (e.g., callback stored in a variable before being passed to
an effect hook). Need to verify how many of the 15 now pass and update
the count.

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_no_global_reassignment.rs`

## setState False Positives (14 fixtures)

**Diagnostic:** "Cannot call setState during render"

**Root cause:** `validate_no_set_state_in_render.rs` flags setState calls
in contexts where upstream allows them. Upstream distinguishes between
direct render-time setState (illegal) and setState inside effects/callbacks
(legal). Our implementation may not correctly identify all callback contexts.

**Upstream file:** `src/Validation/ValidateNoSetStateInRender.ts`

**Fix strategy:** The 14 fixtures suggest our setState detection fires
inside nested scopes, callbacks, or object methods where it shouldn't.
Need to trace whether the setState call is reachable during render vs
only reachable from effect callbacks.

**Sample fixtures:**
- `aliased-nested-scope-fn-expr.tsx`
- `object-method-maybe-alias.js`
- `try-catch-in-nested-scope.ts`

**Key files:**
- `crates/oxc_react_compiler/src/validation/validate_no_set_state_in_render.rs`
