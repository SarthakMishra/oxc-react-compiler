# Codegen Emission Bugs

All issues live in `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` and `crates/oxc_react_compiler/src/reactive_scopes/build_reactive_function.rs`.

---

## Gap 1: Duplicate Declarations in `codegen_scope` ✅

~~**File:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`~~
~~**Priority:** P0 -- breaks 14/16 renders~~

**Completed** (commit `02e0038`): Pre-declare ALL scope output variables at function level before any scope guards, matching upstream compiler behavior. Register StoreLocal targets in `declared` set to prevent re-declaration. Add Destructure to the dead-temp exclusion list so destructuring instructions are preserved.

---

## Gap 2: Hook Destructuring Codegen ✅

~~**File:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`~~
~~**Priority:** P0 -- wrong values rendered~~

**Completed** (commit `02e0038`): Fixed by the scope splitting bug fix in `prune_scopes.rs` -- the `past_hook` flag was being reset when encountering non-scoped instructions between scoped ones, causing post-hook instructions to keep the pre-hook scope ID. This led to hook destructuring results being assigned to the wrong scope's cache slots. With the flag maintained across gaps, hook return values now flow correctly through scope boundaries.

---

## Gap 3: Variable Ordering / Use-Before-Declare ✅

~~**File:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`~~
~~**Priority:** P0 -- runtime errors~~

**Completed** (commit `02e0038`): Resolved by pre-declaring all scope output variables at function level before any scope guards. Since all variables are declared upfront with `let`, guards can reference them in any order without use-before-declare errors.

---

## Gap 4: Assignment vs Re-declaration for Pre-declared Variables ✅

~~**File:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`~~
~~**Priority:** P1 -- breaks compiled output~~

**Completed** (commit `02e0038`): Changed all StoreLocal, Destructure, and general instruction declarations to use `let` instead of `const` to prevent "Assignment to constant variable" errors from scope reload logic. The `declared` set is now checked before emitting declarations, and pre-declared variables get assignment form.

---

## Gap 5: Logical Expression Flattening

**Priority:** P0 -- causes runtime crashes and wrong output in 8+ fixtures

**Current state:** In `build_reactive_function.rs`, `Terminal::Logical` is handled by inlining the right-side block's instructions directly into the parent block (lines 262-270). This destroys short-circuit semantics. The original expression `a && b` or `a ?? b` becomes two sequential assignments:

```js
// Source: val != null && String(val).toLowerCase().includes(filter)
// Generated (WRONG):
t7 = val != null;                                    // left side
t7 = String(val).toLowerCase().includes(lowerFilter); // right side ALWAYS runs, overwrites

// Source: a[sortKey] ?? ""
// Generated (WRONG):
t4 = a[sortKey];   // left side
t4 = "";            // ALWAYS overwrites with fallback
```

This single bug accounts for:
- **color-picker** crash: `isOpen && <JSX>` and `showCustom && <JSX>` -- JSX always renders even when condition is false, accessing undefined props
- **availability-schedule** crash: similar conditional rendering patterns
- **data-table** crash: `?? ""` fallback always executes, then `.localeCompare` on empty string succeeds but `.filter` on wrong value fails
- **avatar-group** wrong output: `max = 3` default parameter via `??` gets overwritten
- **search-input** wrong output: conditional content always renders
- Multiple other fixtures with wrong values from `&&`, `||`, `??` expressions

**What's needed:**

The `Terminal::Logical` has all the information needed to reconstruct the expression:
- `operator: LogicalOp` (And, Or, NullishCoalescing)
- `left: BlockId` (block that computes the left side)
- `right: BlockId` (block that computes the right side, should only execute conditionally)
- `result: Option<Place>` (the place that receives the result)
- `fallthrough: BlockId`

Two possible approaches:

### Approach A: Reconstruct in build_reactive_function (recommended)

Instead of flattening `Terminal::Logical` by inlining its right block, create a new `ReactiveTerminal::Logical` variant that preserves the operator, left block, right block, and result place. Then in `codegen_terminal`, emit:

```js
// For And: result = left_result && (() => { ...right_block...; return right_result; })()
// Or more practically, use the if/ternary pattern:
// For And: t7 = (left_expr) && (right_expr)
// For Or:  t7 = (left_expr) || (right_expr)
// For ??: t7 = (left_expr) ?? (right_expr)
```

The key insight is that the left block computes a value and stores it to the result place, then the right block (which should only run conditionally) also stores to the same result place. The codegen must emit the result place assignment as `result = left_value OP right_value` where right_value is only evaluated when the operator demands it.

### Approach B: Fix the flattening with conditional guards

Keep the flattening approach but wrap the right-block instructions in an `if` guard:
```js
t7 = val != null;           // left side
if (t7) {                    // for && operator
  t7 = String(val)...;      // right side only if left is truthy
}
```

This is simpler but produces less idiomatic output.

**Upstream files:**
- `src/ReactiveScopes/BuildReactiveFunction.ts` -- look for how `Logical` terminal is handled
- `src/ReactiveScopes/CodegenReactiveFunction.ts` -- look for `LogicalExpression` codegen

**Depends on:** None -- this is the highest-impact fix available

---

## Gap 6: Ternary Expression Reconstruction

**Priority:** P0 -- partially addressed but has same pattern as Gap 5

**Current state:** `Terminal::Ternary` is converted to `ReactiveTerminal::If` in `build_reactive_function.rs` and emitted as an `if/else` statement in codegen. This is functionally correct for statement-position ternaries but produces wrong code when the ternary is an expression whose result is assigned to a variable.

The `Terminal::Ternary` has a `result: Option<Place>` field that indicates the place receiving the ternary result. When present, the codegen should emit `result = test ? consequent_expr : alternate_expr` instead of an `if/else` statement. Currently, the result place is ignored during the Ternary-to-If conversion.

For statement-position ternaries (where `result` is None or unused), the current `if/else` emission is correct. The fix is specifically for expression-position ternaries that assign to a result place.

**What's needed:**

- Preserve the `result` place when converting `Terminal::Ternary` to `ReactiveTerminal::If` (or create a `ReactiveTerminal::Ternary` variant)
- In codegen, when a ternary has a result place, emit the conditional expression form
- When a ternary has no result place (statement position), keep the `if/else` form

**Upstream files:**
- `src/ReactiveScopes/CodegenReactiveFunction.ts`

**Depends on:** Likely shares implementation approach with Gap 5
