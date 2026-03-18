# Codegen Emission Bugs

Issues in `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` and `crates/oxc_react_compiler/src/reactive_scopes/build_reactive_function.rs`. Gaps 1-5, 6b, and 9 are completed. Remaining work is fixture-specific correctness issues.

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

## Gap 5: Logical Expression Flattening ✅

~~**Priority:** P0 -- causes runtime crashes and wrong output in 8+ fixtures~~

**Completed**: Added `ReactiveTerminal::Logical` variant with operator, right block, result place. Changed `build_reactive_function.rs` to create structured variant instead of flattening. Codegen wraps right block in `if` conditional based on operator: `if (result)` for `&&`, `if (!result)` for `||`, `if (result == null)` for `??`. Updated all 12 match sites across codegen.rs, merge_scopes.rs, prune_scopes.rs, and validate_preserved_manual_memoization.rs. Render equivalence improved from 40% (10/25) to 68% (17/25).

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

**Depends on:** None (Gap 5 logical expression flattening is now done)

---

## Gap 6b: ForOf/ForIn Loop Codegen ✅

~~**Priority:** P1 -- missing loop forms in codegen~~

**Completed**: Added codegen support for `for...of` and `for...in` loop forms. These were previously not emitted, causing missing loop bodies in compiled output.

---

## Gap 7: availability-schedule Arithmetic

**Priority:** P1 -- renders but wrong output

**Current state:** The availability-schedule fixture renders but produces wrong arithmetic results. Two issues identified:
- Missing `continue` statement in loop codegen
- Operator precedence not preserved correctly in emitted expressions

**What's needed:**
- Add `continue` support to loop terminal codegen
- Audit expression codegen for operator precedence (may need parenthesization logic)

**Upstream files:**
- `src/ReactiveScopes/CodegenReactiveFunction.ts`

**Depends on:** None

---

## Gap 8: canvas-sidebar Missing Return

**Priority:** P1 -- renders but missing return statement

**Current state:** The canvas-sidebar fixture renders but is missing a return statement in the compiled output, causing the component to return `undefined` instead of its JSX tree in certain code paths.

**What's needed:**
- Investigate which terminal/block path drops the return statement
- Likely a codegen issue where a terminal with a return value is emitted as a void statement

**Upstream files:**
- `src/ReactiveScopes/CodegenReactiveFunction.ts`

**Depends on:** None

---

## Gap 9b: booking-list localeCompare

**Priority:** P2 -- 1/2 test cases match

**Current state:** The booking-list fixture matches on one test case but fails on the other with `localeCompare` being undefined. This suggests a scope output variable is not correctly assigned in one code path, causing the sort comparator to receive undefined instead of a string.

**What's needed:**
- Investigate which scope output is uninitialized in the failing test case
- May be an edge case of scope output variable assignment

**Upstream files:**
- `src/ReactiveScopes/CodegenReactiveFunction.ts`

**Depends on:** None
