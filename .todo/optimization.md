# Optimization Pass Gaps

> These passes improve memoization quality and code size but are not required for
> correctness. The compiler will produce valid output without them -- just with
> sub-optimal memoization granularity.

---

## Gap 1: inline_iife

**Upstream:** `packages/babel-plugin-react-compiler/src/Optimization/InlineIIFE.ts` (~200 lines)
**Current state:** `optimization/inline_iife.rs` is a no-op stub (30 lines).
**What's needed:**
- Detect `CallExpression` where callee is a `FunctionExpression` with no captures
- Inline the function body: create new blocks, remap instruction/block IDs
- Map parameters to arguments
- Replace the call result with the function's return value
- Handle single-expression arrow functions (common pattern)
- Handle functions with multiple return points (need phi at join)
- Skip functions that capture mutable state
**Depends on:** None (runs at pass 6, before SSA)

---

## Gap 2: optimize_props_method_calls

**Upstream:** `packages/babel-plugin-react-compiler/src/Optimization/OptimizePropsMethodCalls.ts`
(~100 lines)
**Current state:** `optimization/optimize_props_method_calls.rs` is a no-op stub (41 lines)
with a pseudocode sketch in comments.
**What's needed:**
- Identify places typed as component props (requires type information from `infer_types`)
- For `MethodCall { receiver: props, property, args }`, split into:
  1. `PropertyLoad { object: props, property }` -> new temp
  2. `CallExpression { callee: new_temp, args }`
- Allocate new instruction IDs and identifier IDs via an IdGenerator
- Insert the PropertyLoad instruction before the transformed call
**Depends on:** `infer_types` (needs to know which identifiers are props)

---

## Gap 3: outline_jsx

**Upstream:** `packages/babel-plugin-react-compiler/src/Optimization/OutlineJSX.ts` (~250 lines)
**Current state:** `optimization/outline_jsx.rs` is a no-op stub (11 lines).
**What's needed:**
- Walk instructions and find `JsxExpression` nodes used as inline arguments
- Extract them into separate instructions with their own temporaries
- This makes JSX elements individually memoizable (each gets its own scope)
- Conditional on `config.enable_jsx_outlining`
**Depends on:** None (optional pass, runs at pass 35)

---

## Gap 4: outline_functions

**Upstream:** `packages/babel-plugin-react-compiler/src/Optimization/OutlineFunctions.ts` (~200 lines)
**Current state:** `optimization/outline_functions.rs` is a no-op stub (9 lines).
**What's needed:**
- Identify `FunctionExpression` instructions that don't capture mutable state
- Hoist them to module level (outside the component function)
- Replace the original instruction with a reference to the hoisted function
- This reduces closure overhead and allows the function to be shared across renders
- Conditional on `config.enable_function_outlining`
**Depends on:** `infer_mutation_aliasing_effects` (needs to know which captures are mutable)

---

## Gap 5: optimize_for_ssr

**Upstream:** `packages/babel-plugin-react-compiler/src/Optimization/OptimizeForSSR.ts` (~100 lines)
**Current state:** `optimization/optimize_for_ssr.rs` is a no-op stub (13 lines).
**What's needed:**
- In SSR mode, memoization is unnecessary (each render is fresh)
- Remove reactive scope tracking
- Simplify effect hooks (effects don't run on server)
- Keep basic structure for correctness
- Conditional on `config.enable_ssr`
**Depends on:** None (runs at pass 17, between aliasing effects and DCE)
