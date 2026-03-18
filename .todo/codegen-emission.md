# Codegen Emission Bugs

Issues in `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` and `crates/oxc_react_compiler/src/reactive_scopes/build_reactive_function.rs`.

Completed: Gaps 1-5, 6b, 9. Remaining: 2 render failures (Gaps 7-8), 1 correctness improvement (Gap 6), fixture bug tracking.

---

## Gap 1: Duplicate Declarations in `codegen_scope` ✅

**Completed** (commit `02e0038`): Pre-declare ALL scope output variables at function level before any scope guards, matching upstream compiler behavior.

---

## Gap 2: Hook Destructuring Codegen ✅

**Completed** (commit `02e0038`): Fixed by scope splitting bug fix in `prune_scopes.rs`.

---

## Gap 3: Variable Ordering / Use-Before-Declare ✅

**Completed** (commit `02e0038`): Resolved by pre-declaring all scope output variables at function level.

---

## Gap 4: Assignment vs Re-declaration for Pre-declared Variables ✅

**Completed** (commit `02e0038`): Changed all StoreLocal, Destructure, and general instruction declarations to use `let`.

---

## Gap 5: Logical Expression Flattening ✅

**Completed**: Added `ReactiveTerminal::Logical` variant. Render equivalence improved from 40% to 68%.

---

## Gap 6: Ternary Expression Reconstruction

**Priority:** P4 -- functionally correct but produces `if/else` instead of `?:` for expression-position ternaries

**Current state:** `Terminal::Ternary` is converted to `ReactiveTerminal::If` and emitted as `if/else`. This is functionally correct but diverges from upstream output form. The `result: Option<Place>` field (which indicates expression-position ternaries that should emit `test ? consequent : alternate`) is ignored.

**What's needed:**
- Preserve the `result` place when converting `Terminal::Ternary` to `ReactiveTerminal::If` (or create a `ReactiveTerminal::Ternary` variant)
- In codegen, when a ternary has a result place, emit conditional expression form
- When no result place (statement position), keep `if/else` form

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Depends on:** None

---

## Gap 6b: ForOf/ForIn Loop Codegen ✅

**Completed**: Added codegen support for `for...of` and `for...in` loop forms.

---

## Gap 7: availability-schedule Arithmetic

**Priority:** P1 -- renders but wrong output (2 issues)

**Current state:** The availability-schedule fixture renders but produces wrong arithmetic results.

**Issue 1 -- Missing `continue` statement:** Loop codegen does not emit `continue` statements. The upstream compiler emits `continue` for `Terminal::Continue` (or equivalent). Our codegen either drops it or converts it to an unconditional goto that falls through.

**Issue 2 -- Operator precedence:** The expression `(a - b) / c` is emitted as `a - b / c`, which evaluates differently due to operator precedence. The codegen needs parenthesization logic for binary expressions where the sub-expression has lower precedence than the parent.

**What's needed:**
- Add `continue` support to loop terminal codegen
- Add operator precedence awareness to binary expression codegen (parenthesize when child precedence < parent precedence)

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Depends on:** None

---

## Gap 8: canvas-sidebar Missing Return

**Priority:** P1 -- renders empty/wrong content (missing return statement)

**Current state:** The canvas-sidebar fixture's compiled output is missing its main `return` statement, causing the component to return `undefined`. A deep-work investigation identified this as the main return being dropped during codegen.

**What's needed:**
- Investigate which terminal/block path drops the return statement
- Likely a codegen issue where a `Terminal::Return` with a value is not emitted, or the return value is assigned to a temporary that is then not returned
- Check if this is related to scope boundary handling (the return may be inside a reactive scope that swallows it)

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Depends on:** None

---

## Gap 9b: booking-list localeCompare ✅ (fixture bug)

~~**Priority:** P2 -- 1/2 test cases match~~

**Resolved**: The failing test case crashes for ALL compilers (Original, Babel, OXC) with `Cannot read properties of undefined (reading 'localeCompare')`. This is a fixture bug (the test passes undefined props), not a compiler bug. The one valid test case matches correctly. Effective result: 1/1 valid test cases pass.

---

## Fixture Bugs

The following fixtures crash for ALL compilers (Original React, Babel plugin, and OXC) due to test fixture issues (undefined props, missing data). These are NOT compiler bugs.

| Fixture | Error | Notes |
|---------|-------|-------|
| data-table | `Cannot read properties of undefined (reading 'length')` | Crashes for Original + Babel + OXC |
| time-slot-picker | `Cannot read properties of undefined (reading 'filter')` | Crashes for all, 2 test cases |
| command-menu | `Cannot read properties of undefined (reading '0')` | Crashes for all |
| multi-step-form | `Cannot read properties of undefined (reading '0')` | Crashes for all (1 case) |
| booking-list | `Cannot read properties of undefined (reading 'localeCompare')` | Crashes for all (1 case, other case passes) |

**Recommendation:** Fix these test fixtures by providing valid default props, OR add a `@knownFixtureBug` marker and exclude them from the render equivalence denominator. Either approach is acceptable; fixing the fixtures is preferred for cleaner metrics.
