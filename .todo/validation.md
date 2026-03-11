# Validation Passes

> All 14+ validation passes that check React rules, hooks usage, and code correctness.
> These run at various points in the pipeline (Phase 1, 4, 6, 7, 11).
> Upstream: `src/Validation/Validate*.ts`
> Rust modules: `crates/oxc_react_compiler/src/validation/*.rs`

---

### Gap 1: ValidateContextVariableLValues

**Upstream:** `src/Validation/ValidateContextVariableLValues.ts`
**Pipeline position:** Pass #3, Phase 1
**Current state:** No file exists yet.
**What's needed:**

Validate that context variables (captured outer variables) are not reassigned:

- Walk all `StoreContext` instructions
- Check if the target context variable is declared as `const` in the outer scope
- Emit error if a `const` context variable is being reassigned
- This catches bugs where the lowering produces invalid context writes

Runs early (pass #3) because it validates the HIR structure before further optimization.

**Depends on:** BuildHIR (must have valid HIR with context variables)

---

### Gap 2: ValidateUseMemo

**Upstream:** `src/Validation/ValidateUseMemo.ts`
**Pipeline position:** Pass #4, Phase 1
**Current state:** No file exists yet.
**What's needed:**

Validate useMemo/useCallback usage patterns:

- `useMemo(() => expr, [deps])` — callback must return a value
- `useMemo(() => { ... })` — must not be void (no return)
- `useMemo(async () => ...)` — must not be async
- `useCallback` — similar validations
- Emit warnings/errors for invalid patterns

**Depends on:** BuildHIR

---

### Gap 3: DropManualMemoization

**Upstream:** `src/HIR/DropManualMemoization.ts` (inferred)
**Pipeline position:** Pass #5, Phase 1
**Current state:** No file exists yet.
**What's needed:**

Conditionally remove `StartMemoize`/`FinishMemoize` instruction pairs:

- If `EnvironmentConfig.enable_preserve_existing_memoization_guarantees` is false:
  - Remove all `StartMemoize` and `FinishMemoize` instructions
  - The compiler will determine memoization from scratch
- If true:
  - Keep the markers for `ValidatePreservedManualMemoization` (pass #61)

**Depends on:** BuildHIR (manual memoization markers)

---

### Gap 4: ValidateHooksUsage

**Upstream:** `src/Validation/ValidateHooksUsage.ts`
**Pipeline position:** Pass #12, Phase 4
**Current state:** `validation/validate_hooks_usage.rs` is a stub.
**What's needed:**

Full Rules of Hooks validation on the HIR:

- Hooks must be called at the top level of the function (not inside conditionals, loops, or nested functions)
- Walk the CFG and check that all paths to the function exit call the same hooks in the same order
- Detect conditional hook calls:
  - Hook call inside an `If` terminal's branch but not the other
  - Hook call inside a loop body
  - Hook call inside a try/catch
  - Hook call inside a nested function expression
- Detect hooks called after early returns
- Emit specific error messages indicating which rule is violated
- Configurable via `EnvironmentConfig.validate_hooks_usage`

**Depends on:** SSA (for precise control flow analysis)

---

### Gap 5: ValidateNoCapitalizedCalls

**Upstream:** `src/Validation/ValidateNoCapitalizedCalls.ts`
**Pipeline position:** Pass #13, Phase 4
**Current state:** No file exists yet.
**What's needed:**

Warn when PascalCase functions are called as regular functions (not as JSX):

- Detect `CallExpression` where callee name starts with uppercase
- These should typically be `<Component />` not `Component()`
- Exception: known non-component PascalCase functions (e.g., `Object.keys`, `Array.from`)
- Configurable via config

**Depends on:** SSA

---

### Gap 6: ValidateLocalsNotReassignedAfterRender

**Upstream:** `src/Validation/ValidateLocalsNotReassignedAfterRender.ts`
**Pipeline position:** Pass #21, Phase 6
**Current state:** No file exists yet.
**What's needed:**

Validate that local variables are not reassigned after the render phase:

- "Render phase" ends when the component returns its JSX
- After render, locals should not be mutated (they may have been captured by refs, effects, etc.)
- Uses mutation range information from pass #20
- Emit errors for violations

**Depends on:** InferMutationAliasingRanges

---

### Gap 7: AssertValidMutableRanges

**Upstream:** `src/Inference/AssertValidMutableRanges.ts` (inferred)
**Pipeline position:** Pass #22, Phase 6
**Current state:** No file exists yet.
**What's needed:**

Debug/development assertion pass:

- Verify that all `MutableRange` values are well-formed (start <= end)
- Verify that ranges don't extend beyond the function body
- Verify consistency between range and actual mutation effects
- Optional pass (disabled in production builds)

**Depends on:** InferMutationAliasingRanges

---

### Gap 8: ValidateNoRefAccessInRender

**Upstream:** `src/Validation/ValidateNoRefAccessInRender.ts`
**Pipeline position:** Pass #23, Phase 6
**Current state:** `validation/validate_no_ref_access_in_render.rs` is a stub.
**What's needed:**

Detect ref.current access during render:

- Identify ref values (from `useRef()` return, or type-inferred as ref)
- Check for `.current` property access on refs during render
- `ref.current` during render is unstable and should not be used as a dependency
- Configurable via `EnvironmentConfig.validate_ref_access_during_render`
- Uses effect/type information to identify refs

**Depends on:** InferMutationAliasingRanges, InferTypes

---

### Gap 9: ValidateNoSetStateInRender

**Upstream:** `src/Validation/ValidateNoSetStateInRender.ts`
**Pipeline position:** Pass #24, Phase 6
**Current state:** `validation/validate_no_set_state_in_render.rs` is a stub.
**What's needed:**

Detect unconditional setState calls during render:

- Identify state setter functions (second element of `useState()` return)
- Check for calls to setters outside of event handlers, effects, and callbacks
- Unconditional setState in render causes infinite re-render loops
- Configurable via `EnvironmentConfig.validate_no_set_state_in_render`

**Depends on:** InferMutationAliasingRanges, InferTypes

---

### Gap 10: ValidateNoDerivedComputationsInEffects

**Upstream:** `src/Validation/ValidateNoDerivedComputationsInEffects.ts`
**Pipeline position:** Pass #25, Phase 6
**Current state:** No file exists yet.
**What's needed:**

Detect patterns like `useEffect(() => setState(f(dep)), [dep])`:

- This is a common anti-pattern where derived state is computed in an effect
- Should instead be `const derived = useMemo(() => f(dep), [dep])`
- Identify effect callbacks that compute derived state and set it
- Configurable

**Depends on:** InferMutationAliasingRanges, InferTypes

---

### Gap 11: ValidateNoSetStateInEffects

**Upstream:** `src/Validation/ValidateNoSetStateInEffects.ts`
**Pipeline position:** Pass #26, Phase 6 (lint mode only)
**Current state:** `validation/validate_no_set_state_in_render.rs` exists but this is a different validation.
**What's needed:**

Detect synchronous setState in effect bodies:

- Pattern: `useEffect(() => { setState(value); })` — synchronous setState in effect
- This is usually a mistake (should be async or conditional)
- Lint-mode-only validation
- Need to create `validation/validate_no_set_state_in_effects.rs`

**Depends on:** InferTypes

---

### Gap 12: ValidateNoJSXInTryStatement

**Upstream:** `src/Validation/ValidateNoJSXInTryStatement.ts`
**Pipeline position:** Pass #27, Phase 6 (lint mode only)
**Current state:** No file exists yet for the compiler validation (lint rule exists separately).
**What's needed:**

Detect JSX expressions inside try/catch blocks:

- React doesn't support error recovery for JSX rendering errors via try/catch
- Should use Error Boundaries instead
- Walk the CFG and check for `JsxExpression` instructions inside `Try` terminal blocks
- Lint-mode-only validation

**Depends on:** BuildHIR

---

### Gap 13: ValidateNoFreezingKnownMutableFunctions

**Upstream:** `src/Validation/ValidateNoFreezingKnownMutableFunctions.ts`
**Pipeline position:** Pass #28, Phase 6
**Current state:** No file exists yet.
**What's needed:**

Detect when known mutable functions are frozen by the compiler:

- Some functions are inherently mutable (e.g., they close over mutable state)
- If the compiler would freeze such a function (e.g., as a prop), emit a warning
- Uses effect analysis to determine function mutability

**Depends on:** InferMutationAliasingEffects

---

### Gap 14: ValidateExhaustiveDependencies

**Upstream:** `src/Validation/ValidateExhaustiveDependencies.ts`
**Pipeline position:** Pass #30, Phase 7
**Current state:** No file exists yet.
**What's needed:**

Validate that useMemo/useCallback/useEffect dependency arrays are exhaustive:

- Compare manually specified deps with compiler-inferred deps
- Report missing dependencies
- Report unnecessary dependencies
- Provide autofix suggestions
- Configurable via `EnvironmentConfig.validate_exhaustive_memo_dependencies` and `validate_exhaustive_effect_dependencies`

**Depends on:** InferReactivePlaces (to know which values are reactive dependencies)

---

### Gap 15: ValidateStaticComponents

**Upstream:** `src/Validation/ValidateStaticComponents.ts`
**Pipeline position:** Pass #32, Phase 7 (lint mode only)
**Current state:** No file exists yet.
**What's needed:**

Detect components defined inline during render:

- Pattern: `function Parent() { function Child() { ... } return <Child /> }`
- `Child` is recreated every render, losing state
- Should be defined outside or memoized
- Lint-mode-only validation

**Depends on:** InferReactivePlaces

---

### Gap 16: ValidatePreservedManualMemoization

**Upstream:** `src/Validation/ValidatePreservedManualMemoization.ts`
**Pipeline position:** Pass #61, Phase 11
**Current state:** No file exists yet.
**What's needed:**

Verify that the compiler's memoization is at least as good as manual memoization:

- Compare `StartMemoize`/`FinishMemoize` markers with the compiler's reactive scopes
- If manual useMemo had deps `[a, b]` but the compiler's scope has deps `[a, b, c]`, warn
- If manual memoization would be invalidated in cases where the compiler's would not, that's fine
- The reverse is a problem (compiler is less stable than manual)
- Only runs if `enable_preserve_existing_memoization_guarantees` is true
- Operates on ReactiveFunction (pass #61, after all RF optimizations)

**Depends on:** All RF optimization passes, Manual memoization markers from BuildHIR
