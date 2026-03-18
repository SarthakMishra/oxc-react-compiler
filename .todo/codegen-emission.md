# Codegen Emission Bugs

All four issues live in `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`.

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
