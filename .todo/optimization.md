# Optimization Passes

> Early optimization passes that simplify the HIR before analysis.
> Upstream: `src/Optimization/*.ts`
> Rust modules: `crates/oxc_react_compiler/src/optimization/*.rs`

---

### Gap 1: ConstantPropagation

**Upstream:** `src/Optimization/ConstantPropagation.ts`
**Pipeline position:** Pass #10, Phase 3
**Current state:** `optimization/constant_propagation.rs` is a stub.
**What's needed:**

Propagate known constant values through the HIR:

- Track which SSA identifiers hold known constant values (primitives)
- When an instruction reads a constant identifier, substitute the constant directly
- Fold constant binary/unary expressions: `1 + 2` -> `Primitive(3)`
- Fold constant conditionals: `if (true) { A } else { B }` -> just A
- Propagate through `LoadLocal` / `StoreLocal` chains
- Do NOT propagate across function boundaries (closures may observe mutations)

Standard constant propagation on SSA form. Single pass over all blocks in order.

**Depends on:** SSA (pass #8-9)

---

### Gap 2: InlineIIFE

**Upstream:** `src/Optimization/InlineImmediatelyInvokedFunctionExpressions.ts`
**Pipeline position:** Pass #6, Phase 1 (pre-SSA)
**Current state:** `optimization/inline_iife.rs` is a stub.
**What's needed:**

Inline immediately-invoked function expressions into the calling scope:

- Detect pattern: `(function() { ... })()` or `(() => { ... })()`
- Only inline if:
  - No arguments (or simple argument mapping)
  - Not recursive (doesn't reference itself)
  - No `this` binding issues
- Replace the call with the inlined function body
- Map parameters to arguments
- Handle return value -> becomes the result of the inlined block

**Depends on:** HIR types (complete)

---

### Gap 3: MergeConsecutiveBlocks

**Upstream:** `src/Optimization/MergeConsecutiveBlocks.ts` (inferred from pipeline)
**Pipeline position:** Pass #7, Phase 1
**Current state:** No file exists yet — need to create `optimization/merge_consecutive_blocks.rs`.
**What's needed:**

Merge basic blocks that have a single predecessor/successor relationship:

- If block A's terminal is `Goto { block: B }` and B has only A as predecessor
- Merge B's instructions into A, replace A's terminal with B's terminal
- Update all references from B to A
- Remove B from the block map
- Repeat until no more merges possible

This simplifies the CFG after control flow lowering, which often creates unnecessary intermediate blocks.

**Depends on:** HIR types (complete)

---

### Gap 4: DeadCodeElimination

**Upstream:** `src/Optimization/DeadCodeElimination.ts`
**Pipeline position:** Pass #18, Phase 5
**Current state:** `optimization/dead_code_elimination.rs` is a stub.
**What's needed:**

Remove instructions whose results are never used:

- Build use-def chains from SSA form
- Mark instructions whose lvalues are never referenced by other instructions or terminals
- Remove marked instructions (unless they have side effects)
- Preserve instructions with observable effects (calls, stores, mutations)
- May need to iterate if removing an instruction makes its operands unused

Note: This runs after mutation analysis (pass #18), so it can use effect information to determine which instructions are side-effect-free.

**Depends on:** SSA, InferMutationAliasingEffects (for effect-aware DCE)

---

### Gap 5: PruneMaybeThrows

**Upstream:** `src/HIR/PruneMaybeThrows.ts` (inferred from pipeline)
**Pipeline position:** Pass #2 and Pass #19, Phase 1 and Phase 5
**Current state:** No file exists yet.
**What's needed:**

Remove `MaybeThrow` terminals that are not inside a try/catch:

- Walk the CFG and track whether we are inside a try block
- `MaybeThrow` terminals outside of try blocks serve no purpose (the exception will propagate normally)
- Replace `MaybeThrow { continuation, handler }` with `Goto { block: continuation }` when not in a try
- Remove now-unreachable handler blocks
- Runs twice: once in Phase 1 (cleanup after lowering) and once in Phase 5 (after optimization may have removed try blocks)

**Depends on:** HIR types (complete)

---

### Gap 6: OptimizePropsMethodCalls

**Upstream:** `src/Optimization/OptimizePropsMethodCalls.ts`
**Pipeline position:** Pass #14, Phase 5
**Current state:** No file exists yet — need to create `optimization/optimize_props_method_calls.rs`.
**What's needed:**

Optimize method calls on props to avoid unnecessary memoization:

- Detect `props.onClick()` patterns
- Convert `MethodCall { receiver: props, property: "onClick", args }` into `PropertyLoad` + `CallExpression`
- This allows the property load and the call to be in different reactive scopes, improving granularity

**Depends on:** SSA, InferTypes (needs type info to identify props)
