# InferMutationAliasingRanges: Upstream vs Rust Comparison

**Date:** 2026-04-05
**Upstream:** `facebook/react/compiler/packages/babel-plugin-react-compiler/src/Inference/InferMutationAliasingRanges.ts` (843 lines)
**Ours:** `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_ranges.rs` (~1200 lines)

## Executive Summary

Our implementation has **7 divergences** from upstream. Of these, **3 are likely causing narrower mutable ranges** (the root cause of the `use_mutable_range=true` -55 regression). The remaining 4 are either benign or only affect validation/error reporting.

---

## Divergence 1: BFS (stack/DFS) vs BFS (queue) traversal order — CRITICAL

### Upstream (line 722-723)
```typescript
while (queue.length !== 0) {
  const {place: current, transitive, direction, kind} = queue.pop()!;
```
**Upstream uses `queue.pop()` — this is a STACK (LIFO/DFS), not a queue.**

### Ours (line 228)
```rust
while let Some(item) = bufs.queue.pop_front() {
```
**We use `pop_front()` — this is a true BFS (FIFO).**

### Impact: LIKELY CAUSES NARROWER RANGES

The traversal order matters because of the `seen` map deduplication with `MutationKind` ordering. In DFS, deeper nodes (farther from mutation source) are visited first, which can result in them being marked with higher-priority `MutationKind::Definite` before being re-encountered via a `MaybeAlias` edge that would downgrade to `Conditional`. In BFS, the breadth-first order means we might visit nodes closer to the source first via MaybeAlias edges, marking them as `Conditional`, and then when the `Definite` path arrives, it can still upgrade.

Actually, the more critical difference: DFS follows edges deeply before backtracking. When a mutation propagates backward through `createdFrom` (which sets `transitive: true`), the DFS immediately follows all of that node's edges deeply. With BFS, the backwards `createdFrom` items sit in the queue while forward edges from the original node are processed first. This changes which nodes get "seen" first and with what `MutationKind`, which directly affects range extension.

**Severity: HIGH.** This is the single most likely cause of narrower ranges. The traversal order determines which path "wins" for each node's MutationKind in the dedup check.

### Fix: Change `pop_front()` to `pop_back()` to match upstream's stack-based traversal.

---

## Divergence 2: Forward edge break vs continue — MODERATE

### Upstream (line 761-772)
```typescript
for (const edge of node.edges) {
  if (edge.index >= index) {
    break;  // <-- BREAK, not continue
  }
  queue.push({...});
}
```

### Ours (line 247-262)
```rust
for edge in &node.edges {
    if edge.index >= mutation_index {
        continue; // <-- CONTINUE, not break
    }
    // ...push to queue
}
```

### Impact: LIKELY CAUSES NARROWER RANGES (in edge cases)

Upstream uses `break` because edges are appended in monotonically increasing index order. Once an edge's index exceeds the mutation index, all subsequent edges will too, so `break` is an optimization. But more importantly, upstream's `break` means it stops iterating **immediately**, while our `continue` keeps iterating through all edges.

Wait — actually `continue` processes *more* edges than `break` would if the edges are in order. If edges are strictly monotonically increasing (which they should be since index is incremented after each), then `break` and `continue` produce the same result because no later edge could have `index < mutation_index` once one has `index >= mutation_index`.

**However**, if edges are NOT strictly monotonically increasing (e.g., due to phi operand wiring with stored indices, or deferred phi processing), then `continue` could process edges that `break` would skip. This would actually make our ranges WIDER, not narrower.

**Severity: LOW.** If indices are monotonically ordered, this is a no-op difference. If not, our `continue` is more permissive.

### Fix: Change to `break` for upstream fidelity, but this likely doesn't cause the regression.

---

## Divergence 3: Backward edges use `Map<Identifier, number>` (dedup) vs `Vec<BackEdge>` (duplicates) — CRITICAL

### Upstream (lines 580-597)
```typescript
type Node = {
  createdFrom: Map<Identifier, number>;
  captures: Map<Identifier, number>;
  aliases: Map<Identifier, number>;
  maybeAliases: Map<Identifier, number>;
  // ...
};
```
Backward edges are stored in **Maps keyed by Identifier** — meaning **only one backward edge per source identifier per category**. The `set` operations use `has()` checks:
```typescript
// line 626
if (!toNode.createdFrom.has(from.identifier)) {
  toNode.createdFrom.set(from.identifier, index);
}
```
This means **only the first edge from a given identifier is recorded** as a backward edge.

### Ours (lines 57-69)
```rust
struct Node {
    aliases: Vec<BackEdge>,
    created_from: Vec<BackEdge>,
    captures: Vec<BackEdge>,
    maybe_aliases: Vec<BackEdge>,
}
```
We use **Vecs** — every `add_edge` call pushes a new `(IdentifierId, u32)` tuple. **Duplicate backward edges from the same identifier are preserved.**

### Impact: COULD CAUSE WIDER OR NARROWER RANGES

This is a subtle difference. Upstream deduplicates backward edges by source identifier, keeping only the **first** (lowest index) edge. Our code keeps all backward edges, meaning during BFS backward traversal, we may follow the same source identifier multiple times with different indices.

During mutation BFS, the `when >= mutation_index` filter on backward edges means:
- Upstream: checks only the first edge's index. If the first edge was created before the mutation, it's followed. Later edges from the same source are ignored entirely.
- Ours: checks each edge's index independently. If the first edge was created after the mutation (filtered out), a later edge from the same source with a different index might still be followed, OR we might re-traverse the same source multiple times.

Since the `seen` map deduplicates by identifier, re-traversal from the same source is mostly harmless (it's skipped). But the index filtering difference could cause us to follow backward edges that upstream wouldn't, or miss edges upstream would follow.

**Most likely effect: Our duplicate backward edges are mostly benign due to `seen` dedup, but the index semantics differ.** When upstream stores only the first backward edge's index, it uses the earliest index for filtering. We might use a later index that gets filtered out by `when >= mutation_index`.

Wait — upstream uses `!has()` before `set()`, which means it keeps the **first** (earliest) index. This means upstream backward edges have the LOWEST possible index, making them MORE likely to pass the `when < mutation_index` filter. Our Vec could have later (higher) indices from duplicate edges, but since we also have the first one, and BFS processes all of them, we should also find the earliest one.

**Severity: LOW-MEDIUM.** The duplicate storage shouldn't cause narrower ranges since we preserve the earliest edge too. But it adds noise to traversal.

---

## Divergence 4: CreateFunction capture edges — no `index++` consumed for `Create` — MATCHES UPSTREAM

### Upstream (line 142-146)
```typescript
} else if (effect.kind === 'CreateFunction') {
  state.create(effect.into, {...});
}
```
**Note: upstream's CreateFunction does NOT process captures at graph-build time.** The captures are part of the function's internal effects, not the outer graph. The `Create` just creates a node.

### Ours (lines 403-417)
```rust
AliasingEffect::CreateFunction { into, captures, .. } => {
    graph.create(into.identifier.id);
    // ...
    for cap in captures {
        graph.capture(index, cap.identifier.id, into.identifier.id);
        index += 1;
    }
}
```
**We process capture edges from CreateFunction captures into the graph, consuming index slots.**

### Impact: CAUSES WIDER RANGES (not narrower)

Our code adds extra capture edges that upstream doesn't. This would make ranges WIDER, not narrower. The `captures` field in upstream's `CreateFunction` is used differently — it tracks captured values for the function expression but doesn't add them as graph edges at this point.

Actually wait — let me re-check. Upstream's `AliasingEffect` for `CreateFunction` has a `captures` field but the graph-building code in `inferMutationAliasingRanges` doesn't use it. The captures are handled by separate `Capture` effects that are emitted by `InferMutationAliasingEffects`. So our extra capture edges here may be DUPLICATING edges that already exist as separate `Capture` effects.

**Severity: LOW.** This makes ranges wider (if anything), not narrower. But the index advancement means all subsequent mutation/edge indices are shifted, which could affect filtering.

### Fix: Remove the capture loop from CreateFunction handling, matching upstream. The captures should already be represented as separate Capture effects.

---

## Divergence 5: Assign effect — missing `create` for uninitialized target — POTENTIALLY CRITICAL

### Upstream (lines 149-162)
```typescript
} else if (effect.kind === 'Assign') {
  if (!state.nodes.has(effect.into.identifier)) {
    state.create(effect.into, {kind: 'Object'});
  }
  state.assign(index++, effect.from, effect.into);
}
```
**Upstream creates a node for the `into` target if it doesn't already exist.**

### Ours (lines 429-438)
```rust
AliasingEffect::Assign { from, into } => {
    // ... ranges entries ...
    graph.assign(index, from.identifier.id, into.identifier.id);
    index += 1;
}
```
**We do NOT create a node for `into` if it doesn't exist.** However, our `ensure_node` in `add_edge` creates it implicitly. So this is functionally equivalent — the node gets created with default (empty) state.

But there's a subtle difference: upstream's `create` sets the node's `value` to `{kind: 'Object'}`, while our `ensure_node` creates a node with empty Vecs and no value tracking at all. Since we don't track `value.kind` (Object/Phi/Function), this shouldn't matter for range computation.

**Severity: LOW.** Functionally equivalent for range purposes.

---

## Divergence 6: Missing `node.local` / `node.transitive` tracking — CRITICAL

### Upstream (lines 733-755)
```typescript
node.mutationReason ??= reason;
node.lastMutated = Math.max(node.lastMutated, index);
if (end != null) {
  node.id.mutableRange.end = makeInstructionId(
    Math.max(node.id.mutableRange.end, end),
  );
}
if (node.value.kind === 'Function' && node.transitive == null && node.local == null) {
  appendFunctionErrors(env, node.value.function);
}
if (transitive) {
  if (node.transitive == null || node.transitive.kind < kind) {
    node.transitive = {kind, loc};
  }
} else {
  if (node.local == null || node.local.kind < kind) {
    node.local = {kind, loc};
  }
}
```

Upstream tracks **per-node mutation metadata**:
1. `node.mutationReason` — first mutation reason
2. `node.lastMutated` — highest mutation index that touched this node
3. `node.transitive` — strongest transitive mutation kind
4. `node.local` — strongest local mutation kind
5. Range extension via `Math.max(node.id.mutableRange.end, end)`

### Ours (lines 237-242)
```rust
if let Some(range) = ranges.get_mut(&item.place)
    && end_instr > range.end
{
    range.end = end_instr;
}
```

We only track range extension. We do NOT track:
- `node.local` / `node.transitive` mutation kinds
- `node.lastMutated`
- `node.mutationReason`

### Impact: Does NOT cause narrower ranges directly

The `local`/`transitive`/`lastMutated` fields are used by upstream for:
1. **Part 3** (function effects) — computing externally-visible effects for function expressions. We don't implement Part 3.
2. **Render validation** — the `render()` method checks `node.transitive == null && node.local == null`. We don't implement render validation.

The range extension logic is equivalent: both use `Math.max(current_end, new_end)`.

**Severity: LOW for range computation.** HIGH for function expression effects (Part 3), but that's a separate pass concern.

---

## Divergence 7: Range writeback uses separate creation_map — POTENTIALLY CRITICAL

### Upstream
Upstream writes ranges **directly on `node.id.mutableRange`** during BFS traversal (line 736-738):
```typescript
node.id.mutableRange.end = makeInstructionId(
  Math.max(node.id.mutableRange.end, end),
);
```
Since `node.id` is the actual `Identifier` object (by reference in JavaScript), this mutates the range **in place on the identifier**. The range is then directly available when Part 2 reads `operand.identifier.mutableRange`.

### Ours
We use a separate `ranges: FxHashMap<IdentifierId, MutableRange>` during BFS, then write back in Phase 3 (lines 599-619):
```rust
for instr in &mut block.instructions {
    let id = instr.lvalue.identifier.id;
    let start = creation_map.get(&id).copied().unwrap_or(instr.id);
    let mut end = InstructionId(start.0 + 1);
    if let Some(&range) = ranges.get(&id) && range.end > end {
        end = range.end;
    }
    instr.lvalue.identifier.mutable_range = MutableRange { start, end };
}
```

### Impact: LIKELY CAUSES NARROWER RANGES — THIS IS THE KEY DIVERGENCE

There are several problems:

#### Problem 7a: We only write to lvalue identifiers, not operand identifiers

Upstream mutates the `Identifier` object, which is shared by reference. When a value is created at instruction 5 (lvalue) and used as an operand at instruction 10, both the lvalue and operand reference the SAME `Identifier` object. So when BFS extends the range at the lvalue's identifier, the operand automatically sees the updated range.

In Rust, `Identifier` is a value type. The lvalue's `Identifier` and the operand's `Identifier` are separate copies with the same `id`. **Our Phase 3 writeback only updates lvalue identifiers** (line 605-618). Operand identifiers in instruction values are NOT updated.

This means when Part 2 (`annotate_place_effects`) reads `operand.identifier.mutable_range` to determine effects (e.g., `isMutatedOrReassigned = into.identifier.mutable_range.end > instr.id`), **the operand still has its original range, not the BFS-extended range.**

This is a **fundamental architectural mismatch** caused by Rust's value semantics vs JavaScript's reference semantics.

#### Problem 7b: The `creation_map` start computation

Our start computation uses `creation_map.get(&id).copied().unwrap_or(instr.id)`, falling back to `instr.id`. Upstream doesn't explicitly compute `start` during writeback — it's already set on the identifier. Our approach should be equivalent since we initialize ranges from `identifier.mutable_range` which should have the correct start.

But wait — we override the entire range with `MutableRange { start, end }` where `start` comes from `creation_map`. If the BFS had extended the range's start (which it doesn't — BFS only extends `end`), this would be fine. But there's a subtle issue: the `start` value might differ from what upstream computes because upstream's Part 2 fixup logic reads the already-mutated ranges.

#### Problem 7c: Phi range writeback

We do write phi ranges back (lines 600-604), but phi place identifiers that are also referenced as operands in later instructions still have stale ranges in those operand copies.

**Severity: HIGH.** Problem 7a is almost certainly the primary cause of narrower ranges.

**Specific mechanism:** In `annotate_place_effects` (our Phase 4), we read `into.identifier.mutable_range.end` from `AliasingEffect` structs stored in `instr.effects`. These `Place`/`Identifier` objects are copies created during `infer_mutation_aliasing_effects` (the prior pass). Our BFS (Phase 2) updates `ranges: FxHashMap<IdentifierId, MutableRange>`, and Phase 3 writes those ranges back ONLY to `instr.lvalue.identifier.mutable_range`. The `Identifier` copies inside `instr.effects[].{from,into}` are never updated.

In upstream JavaScript, `Place.identifier` is a reference to a shared `Identifier` object. BFS directly mutates `node.id.mutableRange` (line 736-738), and since `effect.into.identifier` points to the SAME object, Part 2 automatically sees the updated range.

The consequence: `isMutatedOrReassigned` (line 698) reads stale (narrow) ranges from effect targets, returning `false` when upstream would return `true`. This causes source operands to get `Effect::Read` instead of `Effect::Capture`, which propagates to incorrect scope grouping downstream.

### Fix: After BFS writeback (Phase 3), add a pass that updates ALL `Identifier` copies inside `instr.effects` from the `ranges` map. Alternatively, change `annotate_place_effects` to look up ranges from the map rather than reading from the `Identifier` copies. The simplest approach is to pass the computed `ranges` map into `annotate_place_effects` and use it for the `isMutatedOrReassigned` check.

---

## Divergence 8: Part 2 operand effect ordering — MODERATE

### Upstream (lines 340-438)
```typescript
for (const instr of block.instructions) {
  // 1. Set lvalue effects
  for (const lvalue of eachInstructionLValue(instr)) {
    lvalue.effect = Effect.ConditionallyMutate;
    // fixup start/end
  }
  // 2. Set ALL operands to Read FIRST
  for (const operand of eachInstructionValueOperand(instr.value)) {
    operand.effect = Effect.Read;
  }
  // 3. Then build operandEffects map and override
  if (instr.effects == null) continue;
  const operandEffects = new Map<IdentifierId, Effect>();
  // ... build map ...
  // 4. Apply to lvalues
  for (const lvalue of eachInstructionLValue(instr)) {
    const effect = operandEffects.get(lvalue.identifier.id) ?? Effect.ConditionallyMutate;
    lvalue.effect = effect;
  }
  // 5. Apply fixup + effects to operands
  for (const operand of eachInstructionValueOperand(instr.value)) {
    // fixup start
    const effect = operandEffects.get(operand.identifier.id) ?? Effect.Read;
    operand.effect = effect;
  }
}
```

Key: upstream sets ALL operands to `Read` BEFORE building the effect map, then overrides.

### Ours (lines 673-778)
We:
1. Set lvalue effect to `ConditionallyMutate`
2. Build `operand_effects` map
3. Apply lvalue effect from map
4. Apply operand start fixup
5. Apply operand effects (defaulting to Read)

We never explicitly set all operands to `Read` first — we rely on the `apply_operand_effects` function's default branch.

### Impact: PROBABLY BENIGN

The ordering difference doesn't matter because our `apply_operand_effects` function defaults unknown effects to `Read`. The result should be equivalent.

**Severity: LOW.**

---

## Divergence 9: Upstream operandEffects map does NOT use priority-based merging — MODERATE

### Upstream (lines 358-374)
```typescript
const operandEffects = new Map<IdentifierId, Effect>();
for (const effect of instr.effects) {
  switch (effect.kind) {
    case 'Assign':
    case 'Alias':
    case 'Capture':
    case 'CreateFrom':
    case 'MaybeAlias': {
      const isMutatedOrReassigned = effect.into.identifier.mutableRange.end > instr.id;
      if (isMutatedOrReassigned) {
        operandEffects.set(effect.from.identifier.id, Effect.Capture);
        operandEffects.set(effect.into.identifier.id, Effect.Store);
      } else {
        operandEffects.set(effect.from.identifier.id, Effect.Read);
        operandEffects.set(effect.into.identifier.id, Effect.Store);
      }
      break;
    }
```

**Upstream uses plain `Map.set()` — last write wins, no priority merging.**

### Ours (lines 708-715)
```rust
operand_effects
    .entry(from.identifier.id)
    .and_modify(|e| {
        if effect_priority(source_effect) > effect_priority(*e) {
            *e = source_effect;
        }
    })
    .or_insert(source_effect);
```

**We use priority-based merging — higher priority effect wins.**

### Impact: COULD CAUSE DIFFERENT EFFECTS

If an identifier appears as `from` in multiple effects (e.g., once as `Alias` where target is mutated, and once as `Capture` where target is not mutated), upstream would use the LAST effect's value while we use the HIGHEST priority effect's value.

Example:
- Effect 1: `Alias from=x into=y` where `y` is mutated → `x` gets `Capture`
- Effect 2: `Capture from=x into=z` where `z` is NOT mutated → `x` gets `Read`

Upstream: `x` gets `Read` (last write wins)
Ours: `x` gets `Capture` (higher priority wins)

This could cause our effects to be MORE aggressive (wider), not narrower.

**Severity: MEDIUM.** The priority merging could cause different effects, but likely makes them wider, not narrower.

### Fix: Use plain `insert()` (last-write-wins) to match upstream.

---

## Divergence 10: Return terminal effect — `isFunctionExpression` not available

### Upstream (lines 473-476)
```typescript
if (block.terminal.kind === 'return') {
  block.terminal.value.effect = isFunctionExpression
    ? Effect.Read
    : Effect.Freeze;
}
```

### Ours (lines 782-789)
```rust
Terminal::Return { value, .. } => {
    if value.effect == Effect::Unknown {
        value.effect = Effect::Read;
    }
}
```

**We always use `Read`, never `Freeze`.** We don't have the `isFunctionExpression` flag.

### Impact: COULD CAUSE NARROWER RANGES (indirectly)

For non-function-expression functions, upstream freezes the return value. `Freeze` is a stronger effect than `Read`. This could affect downstream passes that check `place.effect`.

**Severity: MEDIUM.** This doesn't affect mutable ranges directly, but affects the `Effect` annotation on return values.

### Fix: Pass `is_function_expression` flag through to this function.

---

## Summary: Root Causes of Narrower Ranges (Priority Order)

### 1. [CRITICAL] Divergence 7a: Operand identifiers not updated after BFS
- **Cause:** Rust value semantics mean operand identifier copies don't get BFS-extended ranges
- **Effect:** `annotate_place_effects` reads stale (narrow) ranges from operands, producing `Read` instead of `Capture`
- **Fix:** After BFS, do a full pass updating all operand identifiers from the `ranges` map
- **Expected impact:** This alone likely fixes the majority of the -55 regression

### 2. [HIGH] Divergence 1: DFS vs BFS traversal order
- **Cause:** Upstream uses `queue.pop()` (stack/DFS), we use `pop_front()` (queue/BFS)
- **Effect:** Different traversal order changes which `MutationKind` wins for deduplicated nodes
- **Fix:** Change `pop_front()` to `pop_back()`
- **Expected impact:** Moderate — affects edge cases where MaybeAlias and Definite paths compete

### 3. [MEDIUM] Divergence 4: Extra CreateFunction capture edges + index shift
- **Cause:** We add capture edges for CreateFunction captures that upstream doesn't
- **Effect:** All subsequent indices are shifted by the number of captures, which could change mutation_index filtering
- **Fix:** Remove capture loop from CreateFunction handling
- **Expected impact:** Low-moderate — index shift affects which edges pass the `when >= mutation_index` filter

### 4. [LOW-MEDIUM] Divergence 9: Priority-based effect merging vs last-write-wins
- **Cause:** Our priority merging differs from upstream's simple `Map.set()`
- **Effect:** Could produce different operand effects in multi-effect scenarios
- **Fix:** Use `insert()` instead of priority-based merging
- **Expected impact:** Low — mostly makes effects wider, not narrower

---

## Recommended Fix Order

1. **Fix Divergence 7a first** — add operand range writeback pass. This is the highest-confidence fix.
2. **Fix Divergence 1** — change `pop_front()` to `pop_back()`. Simple one-line change.
3. **Fix Divergence 4** — remove CreateFunction capture edges. Simple deletion.
4. **Fix Divergence 9** — change to last-write-wins. Simple change.
5. **Test after each fix** to measure incremental impact.

## Validation Plan

After each fix:
1. Run `cargo test --release upstream_conformance -- --nocapture 2>&1 | tail -30`
2. Compare with baseline (559/1717)
3. Check that `use_mutable_range=true` no longer causes -55 regression
4. If regression is eliminated, the `effective_range` workaround (419 fixture over-merge) can potentially be removed
