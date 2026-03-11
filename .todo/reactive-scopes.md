# Reactive Scopes

> Scope inference, alignment, merging, dependency propagation, ReactiveFunction construction, and RF optimization passes.
> Upstream: `src/ReactiveScopes/*.ts`, `src/Inference/InferReactiveScopeVariables.ts`
> Rust modules: `crates/oxc_react_compiler/src/reactive_scopes/*.rs`

---

### Gap 1: InferReactiveScopeVariables

**Upstream:** `src/Inference/InferReactiveScopeVariables.ts`
**Pipeline position:** Pass #33, Phase 8
**Current state:** `reactive_scopes/infer_reactive_scope_variables.rs` is a stub.
**What's needed:**

Uses `DisjointSet<IdentifierId>` to group identifiers into reactive scopes:

1. For each instruction:
   - If the lvalue has `MutableRange > 1` (is mutated after creation) OR the instruction allocates a new value:
     - Union the lvalue with all mutable operands
     - If any operand is reactive, the whole set becomes reactive
2. For phi nodes whose values are mutated after creation:
   - Union all phi operands together
3. Each disjoint set becomes a `ReactiveScope`:
   - `id`: fresh `ScopeId`
   - `range`: merged `MutableRange` (min start, max end across all set members)
   - `declarations`: all identifiers declared within the scope's range
4. Assign `scope: Some(Box::new(reactive_scope))` on each `Identifier` in the set

This is the core memoization decision: identifiers in the same scope are memoized together.

**Depends on:** DisjointSet (utils.md), InferReactivePlaces (inference.md Gap 6), InferMutationAliasingRanges (inference.md Gap 5)

---

### Gap 2: MemoizeFbtAndMacroOperandsInSameScope

**Upstream:** `src/ReactiveScopes/MemoizeFbtAndMacroOperandsInSameScope.ts` (inferred)
**Pipeline position:** Pass #34, Phase 8
**Current state:** No file exists yet.
**What's needed:**

Ensure that fbt (Facebook translation framework) and custom macro operands are grouped into the same reactive scope as their consumer:

- Detect fbt/macro call patterns
- Union the operand identifiers with the call result in the DisjointSet
- This prevents splitting a translation across multiple cache slots

Meta-specific but part of the upstream pipeline. Can be stubbed initially.

**Depends on:** Gap 1

---

### Gap 3: AlignMethodCallScopes

**Upstream:** `src/ReactiveScopes/AlignMethodCallScopes.ts`
**Pipeline position:** Pass #38, Phase 8
**Current state:** No file exists yet (align_scopes.rs exists but may be for Gap 6).
**What's needed:**

Ensure method calls are in the same scope as their receiver:

- Pattern: `obj.method()` where `obj` and the call result are in different scopes
- If `obj` is mutable, the method call must be in the same scope as `obj`
- Merge the scopes so the receiver and call are memoized together

**Depends on:** Gap 1

---

### Gap 4: AlignObjectMethodScopes

**Upstream:** `src/ReactiveScopes/AlignObjectMethodScopes.ts`
**Pipeline position:** Pass #39, Phase 8
**Current state:** No file exists yet.
**What's needed:**

Similar to Gap 3 but for object literal methods:

- Pattern: `{ method() { ... } }` where the object and method are in different scopes
- Ensure the method body's scope aligns with the object's scope

**Depends on:** Gap 1

---

### Gap 5: PruneUnusedLabelsHIR

**Upstream:** `src/ReactiveScopes/PruneUnusedLabelsHIR.ts` (inferred)
**Pipeline position:** Pass #40, Phase 8
**Current state:** No file exists yet.
**What's needed:**

Remove `Label` terminals whose labels are never referenced by `break`/`continue`:

- Walk all terminals looking for break/continue targets
- Collect used label IDs
- Replace `Label { block, fallthrough, label }` with `Goto { block }` if label is unused
- Simplifies the CFG before scope alignment

**Depends on:** Gap 1

---

### Gap 6: AlignReactiveScopesToBlockScopesHIR

**Upstream:** `src/ReactiveScopes/AlignReactiveScopesToBlockScopesHIR.ts`
**Pipeline position:** Pass #41, Phase 8
**Current state:** `reactive_scopes/align_scopes.rs` exists as a stub.
**What's needed:**

Align reactive scope boundaries to JavaScript block scope boundaries:

- A reactive scope must not start or end in the middle of a `let`/`const` block scope
- If a scope's range partially overlaps a block scope, extend it to cover the entire block
- This ensures the generated memoization blocks have valid JavaScript scoping
- Single CFG traversal, O(n)

**Depends on:** Gap 1

---

### Gap 7: MergeOverlappingReactiveScopesHIR

**Upstream:** `src/ReactiveScopes/MergeOverlappingReactiveScopesHIR.ts`
**Pipeline position:** Pass #42, Phase 8
**Current state:** `reactive_scopes/merge_scopes.rs` exists as a stub.
**What's needed:**

Merge reactive scopes whose ranges overlap:

- If scope A's range overlaps with scope B's range, merge them into one scope
- The merged scope has `range = [min(A.start, B.start), max(A.end, B.end)]`
- Merge declarations, reassignments, and dependencies
- Record merged scope IDs in `ReactiveScope.merged`
- Iterate until no more overlapping scopes exist (merging can create new overlaps)

**Depends on:** Gap 6 (alignment may create overlaps)

---

### Gap 8: BuildReactiveScopeTerminalsHIR

**Upstream:** `src/ReactiveScopes/BuildReactiveScopeTerminalsHIR.ts`
**Pipeline position:** Pass #43, Phase 8
**Current state:** No file exists yet.
**What's needed:**

Insert `Scope` terminals into the CFG to delineate reactive scope boundaries:

- For each reactive scope, find the entry and exit points in the CFG
- Insert `Scope { scope_id, block, fallthrough }` terminals at scope entry points
- The `block` contains the scope's instructions
- The `fallthrough` continues after the scope
- This transforms the CFG from scope-annotated-identifiers to scope-structured blocks

**Depends on:** Gap 7

---

### Gap 9: FlattenReactiveLoopsHIR

**Upstream:** `src/ReactiveScopes/FlattenReactiveLoopsHIR.ts`
**Pipeline position:** Pass #44, Phase 8
**Current state:** No file exists yet.
**What's needed:**

Flatten reactive scopes that span loop boundaries:

- If a reactive scope's range covers the entire body of a loop, flatten it out
- Move the scope boundary outside the loop
- This prevents re-memoizing on every loop iteration

**Depends on:** Gap 8

---

### Gap 10: FlattenScopesWithHooksOrUseHIR

**Upstream:** `src/ReactiveScopes/FlattenScopesWithHooksOrUseHIR.ts`
**Pipeline position:** Pass #45, Phase 8
**Current state:** No file exists yet.
**What's needed:**

Flatten reactive scopes that contain hook calls or `use()`:

- Hooks must be called unconditionally at the top level
- If a reactive scope contains a hook call, the scope cannot be conditionally skipped
- Flatten such scopes: remove the `Scope` terminal, keep the instructions inline
- This ensures hooks are always called (Rules of Hooks)

**Depends on:** Gap 8

---

### Gap 11: PropagateScopeDependenciesHIR

**Upstream:** `src/ReactiveScopes/PropagateScopeDependenciesHIR.ts`
**Pipeline position:** Pass #46, Phase 8
**Current state:** `reactive_scopes/propagate_dependencies.rs` exists as a stub.
**What's needed:**

Compute the dependency set for each reactive scope:

- A scope's dependencies are the reactive values read inside the scope that are defined outside it
- For each scope:
  1. Walk all instructions inside the scope
  2. For each read operand, check if it's defined outside the scope
  3. If yes, add to the scope's `dependencies` set
  4. Track dependency paths: `props.a.b` becomes `ReactiveScopeDependency { identifier: props, path: [a, b] }`
- Dependency paths are critical for codegen: they become the cache invalidation checks
- O(n * s) where n = instructions and s = scopes

**Depends on:** Gap 8 (scope terminals must be built first)

---

### Gap 12: BuildReactiveFunction

**Upstream:** `src/ReactiveScopes/BuildReactiveFunction.ts`
**Pipeline position:** Pass #47, Phase 9
**Current state:** `reactive_scopes/build_reactive_function.rs` exists as a stub.
**What's needed:**

Convert the CFG-shaped HIR into a tree-shaped ReactiveFunction:

- The HIR is a flat CFG with `Scope` terminals marking reactive scope boundaries
- The ReactiveFunction is a nested tree:
  - `ReactiveBlock` contains a list of `ReactiveInstruction`s
  - `ReactiveInstruction` can be a regular instruction, a control flow terminal (with nested blocks), or a `ReactiveScopeBlock`
- Algorithm:
  1. Start from the entry block
  2. Emit instructions linearly until hitting a terminal
  3. For `Scope` terminals: create `ReactiveScopeBlock`, recursively process the scope body
  4. For `If`/`Switch`/loops: create nested `ReactiveTerminal` with recursive blocks
  5. For `Goto`/`Return`: emit directly
- The tree structure mirrors the final JavaScript output: each `ReactiveScopeBlock` becomes a `if ($[n] !== dep) { ... }` block

**Depends on:** Gap 11 (all HIR scope passes must be complete)

---

### Gap 13: PruneUnusedLabels (RF)

**Upstream:** `src/ReactiveScopes/PruneUnusedLabels.ts` (ReactiveFunction version)
**Pipeline position:** Pass #48, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Same as Gap 5 but operating on the ReactiveFunction tree instead of the CFG:
- Remove label terminals whose labels are never targeted by break/continue
- Walk the ReactiveFunction tree, collect used labels, prune unused ones

**Depends on:** Gap 12

---

### Gap 14: PruneNonEscapingScopes

**Upstream:** `src/ReactiveScopes/PruneNonEscapingScopes.ts`
**Pipeline position:** Pass #49, Phase 10
**Current state:** `reactive_scopes/prune_scopes.rs` exists as a stub.
**What's needed:**

Remove reactive scopes whose declarations are never used outside the scope:

- If all identifiers declared by a scope are only used within the scope itself, the scope is unnecessary
- Convert `ReactiveScopeBlock` to plain instructions (remove the scope wrapper)
- This reduces the number of cache slots needed

**Depends on:** Gap 12

---

### Gap 15: PruneNonReactiveDependencies

**Upstream:** `src/ReactiveScopes/PruneNonReactiveDependencies.ts`
**Pipeline position:** Pass #50, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Remove non-reactive dependencies from scope dependency sets:

- If a dependency is not reactive (its value never changes between renders), it doesn't need to be checked
- Remove non-reactive entries from `ReactiveScope.dependencies`
- A constant dependency means the scope's output is also constant for that input

**Depends on:** Gap 12

---

### Gap 16: PruneUnusedScopes

**Upstream:** `src/ReactiveScopes/PruneUnusedScopes.ts`
**Pipeline position:** Pass #51, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Remove reactive scopes that have no declarations and no output values:

- After other pruning passes, some scopes may become empty
- Remove these empty `ReactiveScopeBlock` nodes from the tree
- Flatten their instructions into the parent block

**Depends on:** Gap 14, Gap 15

---

### Gap 17: MergeReactiveScopesThatInvalidateTogether

**Upstream:** `src/ReactiveScopes/MergeReactiveScopesThatInvalidateTogether.ts`
**Pipeline position:** Pass #52, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Merge adjacent scopes that have the same dependency set:

- If scope A and scope B are adjacent and `A.dependencies == B.dependencies`, merge them
- This reduces the number of cache invalidation checks (one check instead of two)
- Must check that merging doesn't violate scope ordering constraints

**Depends on:** Gap 16

---

### Gap 18: PruneAlwaysInvalidatingScopes

**Upstream:** `src/ReactiveScopes/PruneAlwaysInvalidatingScopes.ts`
**Pipeline position:** Pass #53, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Remove scopes that would always be invalidated (never hit cache):

- If a scope depends on a value that changes every render (e.g., `new Date()`), the scope is useless
- Detect patterns like dependencies on impure function results
- Convert to plain instructions (remove memoization)

**Depends on:** Gap 17

---

### Gap 19: PropagateEarlyReturns

**Upstream:** `src/ReactiveScopes/PropagateEarlyReturns.ts`
**Pipeline position:** Pass #54, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Handle early returns that occur inside reactive scopes:

- If a scope contains a `return` terminal, the scope needs special handling
- Set `ReactiveScope.early_return_value` to track the return path
- In codegen, generate code that checks the cache and returns early if needed

**Depends on:** Gap 12

---

### Gap 20: PruneUnusedLvalues

**Upstream:** `src/ReactiveScopes/PruneUnusedLvalues.ts`
**Pipeline position:** Pass #55, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Remove lvalue declarations for values that are never read:

- Walk the ReactiveFunction tree
- Build use counts for each identifier
- If an identifier is only written (declared) but never read, remove the declaration
- Keep the RHS if it has side effects; just remove the assignment

**Depends on:** Gap 12

---

### Gap 21: PromoteUsedTemporaries

**Upstream:** `src/ReactiveScopes/PromoteUsedTemporaries.ts`
**Pipeline position:** Pass #56, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Promote temporary variables to named declarations when they are used as scope outputs:

- Temporaries (unnamed identifiers) that are output from a scope need names for codegen
- Generate descriptive names based on the expression that produced them
- E.g., a temporary holding `props.count + 1` might become `t0` or `count_plus_1`

**Depends on:** Gap 12

---

### Gap 22: ExtractScopeDeclarationsFromDestructuring

**Upstream:** `src/ReactiveScopes/ExtractScopeDeclarationsFromDestructuring.ts`
**Pipeline position:** Pass #57, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Extract individual declarations from destructuring patterns at scope boundaries:

- If a destructuring like `const { a, b } = obj` spans a scope boundary, split it
- The object creation stays inside the scope, individual bindings are extracted
- This allows finer-grained memoization of individual destructured values

**Depends on:** Gap 12

---

### Gap 23: StabilizeBlockIds

**Upstream:** `src/ReactiveScopes/StabilizeBlockIds.ts`
**Pipeline position:** Pass #58, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Renumber block IDs to be sequential and deterministic:

- After all transformations, block IDs may be sparse or out of order
- Renumber to sequential IDs (0, 1, 2, ...) based on traversal order
- This ensures deterministic output for testing (same input always produces same block numbering)

**Depends on:** All previous RF passes

---

### Gap 24: RenameVariables

**Upstream:** `src/ReactiveScopes/RenameVariables.ts`
**Pipeline position:** Pass #59, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Rename SSA variables back to user-friendly names for codegen:

- SSA may have created many versions of the same variable (e.g., `x_0`, `x_1`, `x_2`)
- Rename back to the original names where possible
- Generate unique names for temporaries
- Avoid name collisions
- Handle scope-based renaming (variables in different scopes can have the same name)

**Depends on:** Gap 23

---

### Gap 25: PruneHoistedContexts

**Upstream:** `src/ReactiveScopes/PruneHoistedContexts.ts`
**Pipeline position:** Pass #60, Phase 10
**Current state:** No file exists yet.
**What's needed:**

Remove hoisted context variable declarations that are no longer needed:

- After optimization passes, some context variables may have been inlined or eliminated
- Remove their declarations from the function's context list
- Clean up `DeclareContext` / `LoadContext` instructions for removed contexts

**Depends on:** Gap 24
