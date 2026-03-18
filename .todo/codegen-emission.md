# Codegen Emission Bugs

All four issues live in `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`. These are the highest-priority items in the entire backlog: the scope and dependency analysis is 93.8% structurally correct, but broken code emission causes 14/16 render benchmarks to fail at runtime.

Fixing these four gaps is expected to take the render equivalence score from ~4% to a dramatically higher number.

---

## Gap 1: Duplicate Declarations in `codegen_scope`

**File:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
**Priority:** P0 -- breaks 14/16 renders

**Current state:** The pre-declaration hoisting stage emits `let x;` at the top of the function for variables that will be assigned inside scope blocks. However, when those same variables appear inside a scope body, the emitter re-declares them with `const x = ...` or `let x = ...`, producing a duplicate declaration that causes a runtime `SyntaxError`.

**What's needed:**

- The `declared` tracking set must be consulted when emitting instructions inside scope bodies, not only during the hoisting pass
- If a variable is already in the `declared` set, emit an assignment (`x = value`) instead of a declaration (`const x = value`)
- Add test coverage: any fixture that has variables declared in a scope body and also hoisted should produce valid JS

**Evidence:** 14/16 render benchmarks fail with duplicate declaration errors. The structural analysis shows the scope boundaries and dependencies are correct -- the bug is purely in emission.

**Depends on:** None

---

## Gap 2: Hook Destructuring Codegen

**File:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
**Priority:** P0 -- wrong values rendered

**Current state:** When the compiler encounters hook destructuring patterns like `const [count, setCount] = useState(0)`, the generated code creates empty cache slots but does not actually call the hook and store its return value. The destructured variables end up undefined at runtime.

**What's needed:**

- The codegen must emit the actual hook call (`useState(0)`) and assign its return value
- The destructuring pattern must be preserved or lowered into individual assignments from the hook result
- The cache slot logic must wrap the hook call, not replace it
- Verify against upstream behavior: hooks are never memoized themselves, only their results flow into downstream scopes

**Evidence:** Rendered output shows wrong/missing values for any component using `useState`, `useReducer`, or similar hooks with destructured returns.

**Depends on:** None

---

## Gap 3: Variable Ordering / Use-Before-Declare

**File:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
**Priority:** P0 -- runtime errors

**Current state:** The dependency guard pattern `if ($[n] !== x)` sometimes references a variable `x` that has not been declared yet at that point in the emitted code. This happens when scope emission order does not match the declaration order of the variables involved.

**What's needed:**

- Either reorder scope emission so that scopes producing a variable are emitted before scopes that depend on it
- Or hoist all variable declarations to the top of the function (before any scope blocks), so that guards can reference them in any order
- The upstream compiler uses a topological sort on scope dependencies to determine emission order -- verify our implementation matches
- Add test coverage for cross-scope dependencies where variable A (produced by scope 1) is a dependency of scope 2

**Evidence:** Runtime `ReferenceError: x is not defined` in guard conditions for compiled output.

**Depends on:** Gap 1 (duplicate declarations must be fixed first, since hoisting more declarations amplifies the duplicate-declaration bug if Gap 1 is not resolved)

---

## Gap 4: Assignment vs Re-declaration for Pre-declared Variables

**File:** `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
**Priority:** P1 -- breaks compiled output

**Current state:** When a variable is pre-declared with `let x;` at the function level (via hoisting), the scope body still emits `const x = value` instead of `x = value`. This is a `SyntaxError` since `x` is already declared.

**What's needed:**

- When emitting an instruction inside a scope body, check if the target variable was pre-declared (exists in the hoisted declarations set)
- If yes, emit a plain assignment: `x = value`
- If no, emit a declaration: `const x = value`
- This is closely related to Gap 1 but distinct: Gap 1 is about the `declared` set not being checked; Gap 4 is about the emission form being wrong even when the check exists

**Evidence:** Same duplicate-declaration errors as Gap 1, but this specifically covers the assignment form inside scope bodies.

**Depends on:** Gap 1 (fixing the `declared` set tracking is prerequisite to correctly choosing assignment vs declaration)
