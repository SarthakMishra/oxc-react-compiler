# Inference Passes

> Type inference, mutation analysis, aliasing effects, and reactivity inference.
> These are the core algorithmic engine of the compiler.
> Upstream: `src/Inference/*.ts`
> Rust modules: `crates/oxc_react_compiler/src/inference/*.rs`

---

### Gap 1: InferTypes

**Upstream:** `src/Inference/InferTypes.ts`
**Pipeline position:** Pass #11, Phase 3
**Current state:** `inference/infer_types.rs` is a stub.
**What's needed:**

Constraint-based type inference that annotates every `Identifier.type_` field:

- Walk all instructions and generate type constraints based on instruction kind
- Propagate types through data flow:
  - `Primitive(value)` -> infer concrete primitive type
  - `CallExpression(callee, args)` -> look up callee's shape, use return type
  - `PropertyLoad(object, property)` -> look up object shape, get property type
  - `ObjectExpression` -> create object type
  - `ArrayExpression` -> create array type
  - `JsxExpression` -> JSX element type
  - `FunctionExpression` -> function type
  - `BinaryExpression` -> infer result type from operand types and operator
- Use `ShapeRegistry` to resolve built-in types and methods
- Propagate through phi nodes (join types from branches)
- Handle `LoadGlobal` -> look up global shape
- Type narrowing through conditional branches (optional, upstream does limited narrowing)
- Result: every `Identifier` has a populated `Type` field for use by mutation analysis

**Depends on:** SSA (pass #8-9), ObjectShape/ShapeRegistry/Globals (environment.md)

---

### Gap 2: AliasingEffect Enum and Composition Rules

**Upstream:** `src/Inference/AliasingEffects.ts`
**Current state:** `inference/aliasing_effects.rs` is a stub.
**What's needed:**

The `AliasingEffect` enum (17 variants) and the functions that compute effects for each instruction kind:

- `AliasingEffect` enum from REQUIREMENTS.md Section 8:
  - Creation: `Create`, `CreateFrom`, `CreateFunction`, `Apply`
  - Data flow: `Assign`, `Alias`, `MaybeAlias`, `Capture`, `ImmutableCapture`
  - Mutation: `Mutate`, `MutateConditionally`, `MutateTransitive`, `MutateTransitiveConditionally`, `Freeze`
  - Errors: `MutateFrozen`, `MutateGlobal`, `Impure`, `Render`
- `compute_effects_for_instruction(instruction: &Instruction, env: &Environment) -> Vec<AliasingEffect>`
  - For each `InstructionValue` variant, produce the appropriate effects
  - Uses `FunctionSignature` from shape system to determine call effects
  - E.g., `CallExpression` with a known pure function -> `Create` for return, `Read` for args
  - E.g., `PropertyStore` -> `Mutate` for object, `Capture` of value into object
- Transitivity rules (from REQUIREMENTS.md Section 8):
  - `Assign`/`Alias`/`CreateFrom`: direct edge, local mutation flows through
  - `Capture`: indirect edge, local mutation does NOT flow, transitive DOES
  - `MaybeAlias`: downgrades mutation to conditional
  - `Freeze`: freezes the reference

**Depends on:** HIR types (Gap 5: Effect, ValueKind), ObjectShape/ShapeRegistry

---

### Gap 3: InferMutationAliasingEffects

**Upstream:** `src/Inference/InferMutationAliasingEffects.ts`
**Pipeline position:** Pass #16, Phase 5
**Current state:** `inference/infer_mutation_aliasing_effects.rs` is a stub.
**What's needed:**

The most computationally intensive pass. Abstract interpretation over the HIR:

1. **Build abstract heap model**:
   - Each `IdentifierId` maps to an abstract value with a `ValueKind`
   - Pointer graph: tracks aliasing relationships between values
   - Effects create edges in the pointer graph

2. **For each instruction** (in CFG order):
   - Compute candidate effects using `compute_effects_for_instruction()`
   - Apply each effect to the abstract heap:
     - `Create(into, kind)` -> create new abstract value
     - `Alias(from, into)` -> into points to same value as from
     - `Capture(from, into)` -> into captures from (indirect reference)
     - `Mutate(value)` -> mark value and all transitively aliased values as mutated
     - `Freeze(value)` -> mark value as frozen
     - `MutateFrozen` -> emit error diagnostic
   - Record the resolved effects on `instruction.effects`

3. **Fixpoint iteration** for loops:
   - Loop bodies may need multiple iterations until the abstract state stabilizes
   - Typically converges in 2-5 iterations
   - Complexity: O(n * k) where k is iterations to convergence

4. **Effect on each `Place`**:
   - After computing effects, annotate each `Place.effect` field in the instruction

Key Rust considerations:
- The abstract heap can use `FxHashMap<IdentifierId, AbstractValue>`
- Pointer graph edges via `FxHashMap<IdentifierId, FxHashSet<IdentifierId>>`
- Need careful lifetime management for iterating while mutating the heap

**Depends on:** Gap 2 (AliasingEffect), InferTypes (#11), AnalyseFunctions (#15)

---

### Gap 4: AnalyseFunctions

**Upstream:** `src/Inference/AnalyseFunctions.ts` (inferred from pipeline)
**Pipeline position:** Pass #15, Phase 5
**Current state:** No file exists yet.
**What's needed:**

Recursively analyze nested function expressions before the parent function's mutation analysis:

- For each `FunctionExpression` / `ObjectMethod` instruction in the HIR:
  - Run the analysis pipeline (passes #1-16) on the nested `HIRFunction`
  - Record the function's effect signature (how it affects its captures and parameters)
- The parent function's `InferMutationAliasingEffects` can then use the nested function's signature
- This enables precise effect analysis: if a callback only reads its captures, the parent doesn't need to treat the capture as mutated

Recursive analysis order:
- Analyze innermost functions first (bottom-up)
- Each nested function is analyzed independently
- Results are stored as `FunctionSignature` on the function instruction

**Depends on:** All passes up to #15 must be implemented for recursive analysis

---

### Gap 5: InferMutationAliasingRanges

**Upstream:** `src/Inference/InferMutationAliasingRanges.ts`
**Pipeline position:** Pass #20, Phase 5
**Current state:** `inference/infer_mutation_aliasing_ranges.rs` is a stub.
**What's needed:**

Compute `MutableRange` for each identifier based on the effects from pass #16:

- For each `Identifier`:
  - `start`: the `InstructionId` where the value is created (first `Create` effect)
  - `end`: the `InstructionId` of the last instruction that mutates this value (transitively through aliases)
- Algorithm:
  1. Walk all instructions, collect effect-annotated places
  2. For each `Create` effect, set `start = instruction.id`
  3. For each `Mutate`/`MutateTransitive` effect, extend `end` to `max(end, instruction.id)`
  4. Follow alias chains: if A aliases B and B is mutated at instruction N, then A's range extends to N
- Values with `MutableRange.start == MutableRange.end` are never mutated (constants)
- Values with ranges extending beyond their creation block may span multiple scopes

The mutable ranges directly feed into `InferReactiveScopeVariables` (pass #33) for scope grouping.

**Depends on:** Gap 3 (InferMutationAliasingEffects)

---

### Gap 6: InferReactivePlaces

**Upstream:** `src/Inference/InferReactivePlaces.ts`
**Pipeline position:** Pass #29, Phase 7
**Current state:** `inference/infer_reactive_places.rs` is a stub.
**What's needed:**

Mark which places are "reactive" (their value may change between renders):

- **Reactive sources**: function parameters (props), hook return values, context values
- **Reactivity propagation**: any value computed from reactive values is also reactive
  - `t0 = props.a` -> t0 is reactive
  - `t1 = t0 + 1` -> t1 is reactive (depends on reactive t0)
  - `t2 = "hello"` -> t2 is NOT reactive (constant)
- **Fixpoint iteration** with post-dominator analysis:
  - Reactivity flows forward through data dependencies
  - At join points (phi nodes), if any operand is reactive, the phi result is reactive
  - Iterate until no more places become reactive
- Set `Place.reactive = true` for all reactive places
- Values that are NOT reactive do not need memoization (they are the same every render)

This is critical for scope inference: only reactive values trigger memoization scopes.

**Depends on:** InferMutationAliasingRanges (pass #20), validation passes (#21-28)

---

### Gap 7: RewriteInstructionKindsBasedOnReassignment

**Upstream:** `src/Inference/RewriteInstructionKindsBasedOnReassignment.ts` (inferred from pipeline)
**Pipeline position:** Pass #31, Phase 7
**Current state:** No file exists yet.
**What's needed:**

After reactivity inference, rewrite `InstructionKind` on `DeclareLocal`/`StoreLocal` instructions:

- If a `const` declaration is reassigned (via phi nodes from SSA), change to `let`
- If a `let` declaration is never reassigned, it can stay as `let` (or be promoted to `const` in codegen)
- This ensures the generated code uses the correct declaration keywords

**Depends on:** Gap 6 (InferReactivePlaces)
