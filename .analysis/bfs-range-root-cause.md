# BFS Range Root Cause Analysis

**Date:** 2026-04-05
**Symptom:** `use_mutable_range=true` causes -57 regression (560->503), dominated by over-splitting (+1 category: 66->145)
**Status:** Root causes identified, fixes proposed

---

## Executive Summary

Three root causes were identified, in order of impact:

1. **[CRITICAL] Duplicate capture edges from CreateFunction** -- Our `CreateFunction` handling in `infer_mutation_aliasing_ranges.rs` creates capture graph edges that are DUPLICATES of separate `Capture` effects already emitted by `infer_mutation_aliasing_effects.rs`. The extra edges advance the global `index` counter, shifting ALL subsequent mutation and edge indices higher than upstream's. This causes the BFS `edge.index >= mutation_index` filter to exclude valid graph paths, producing narrower ranges.

2. **[MEDIUM] Priority-based Capture effect merging in Part 2** -- Our Capture branch in `annotate_place_effects` uses `and_modify` with priority-based merging, while upstream uses `Map.set()` (last-write-wins) identically for ALL five aliasing effects. This produces different operand effects in multi-effect scenarios, though it tends to make effects WIDER (not narrower), so it contributes to other divergences rather than over-splitting directly.

3. **[LOW] Stale operand mutable_range in start fixup** -- The `apply_operand_start_fixup` function reads `place.identifier.mutable_range.end` from operand copies that were NOT updated by the BFS writeback (Phase 3 only updates lvalue identifiers). In upstream, JavaScript reference semantics make the BFS-extended range visible to all copies. This affects `mutableRange.start` assignment but not `end`, and `infer_reactive_scope_variables` reads from the lvalue ranges map, so the impact on scope grouping is minimal.

---

## Root Cause 1: Duplicate CreateFunction Capture Edges (CRITICAL)

### The Problem

In `infer_mutation_aliasing_ranges.rs`, lines 405-418:

```rust
AliasingEffect::CreateFunction { into, captures, .. } => {
    graph.create(into.identifier.id);
    // ...
    for cap in captures {
        graph.capture(index, cap.identifier.id, into.identifier.id);
        index += 1;  // <-- advances global index
    }
}
```

In upstream `InferMutationAliasingRanges.ts`:

```typescript
} else if (effect.kind === 'CreateFunction') {
  state.create(effect.into, {kind: 'Function', function: ...});
  // NO capture edges, NO index advancement
}
```

### Why Duplicates Exist

The effects pass (`infer_mutation_aliasing_effects.rs`, lines 940-952) ALREADY generates separate `Capture` effects for each CreateFunction capture:

```rust
AliasingEffect::CreateFunction { captures, function: _, into } => {
    // ... state management ...
    effects.push(effect.clone());  // Push the CreateFunction itself
    for capture in captures {
        apply_effect(ctx, state,
            &AliasingEffect::Capture { from: capture.clone(), into: into.clone() },
            // ...
        );  // This may produce Capture, MaybeAlias, ImmutableCapture, or nothing
    }
}
```

So when `infer_mutation_aliasing_ranges` processes the instruction's effects, it sees:
1. `CreateFunction { captures: [a, b] }` -- our code creates 2 capture edges, advances index by 2
2. `Capture { from: a, into: fn }` -- creates another capture edge, advances index by 1
3. `Capture { from: b, into: fn }` -- creates another capture edge, advances index by 1

Upstream only sees edges from steps 2 and 3. Our global `index` is now 2 higher than upstream's for ALL subsequent effects in this instruction and beyond.

### Worse: Conditional Capture Effects

The `apply_effect` for Capture may NOT always produce a `Capture` effect:
- If source is frozen: converts to `ImmutableCapture` (skipped by range inference)
- If source is primitive/global: dropped entirely
- If context involved: converts to `MaybeAlias`

So our `CreateFunction` handling creates capture edges UNCONDITIONALLY, while the actual `Capture` effects may be downgraded or dropped. This means:
- We create edges between nodes that upstream wouldn't connect
- For frozen/global captures, we advance `index` for edges that shouldn't exist

### Impact Mechanism

The index offset cascades through the entire remainder of the function:
- Mutation effect at our index `N+K` corresponds to upstream's index `N`
- BFS filter `edge.index >= mutation_index` becomes `edge.index >= N+K`
- Edges with indices between `N` and `N+K-1` are incorrectly excluded
- These excluded edges may be the paths through which range extensions propagate
- Result: identifiers that upstream would extend don't get extended, producing narrower ranges

### Proposed Fix

Remove the capture loop from `CreateFunction` handling in `infer_mutation_aliasing_ranges.rs`:

```rust
AliasingEffect::CreateFunction { into, .. } => {
    graph.create(into.identifier.id);
    creation_map.entry(into.identifier.id).or_insert(instr.id);
    ranges.entry(into.identifier.id).or_insert(into.identifier.mutable_range);
    // Do NOT create capture edges here -- they come from separate Capture effects
}
```

**Expected impact:** Fixes the index offset for ALL fixtures that contain function expressions with captures. This is the single highest-impact fix.

---

## Root Cause 2: Capture Effect Priority Merging (MEDIUM)

### The Problem

In `infer_mutation_aliasing_ranges.rs`, lines 714-731 (our `annotate_place_effects`):

```rust
AliasingEffect::Capture { from, into } => {
    // ...
    operand_effects
        .entry(from.identifier.id)
        .and_modify(|e| {
            if effect_priority(source_effect) > effect_priority(*e) {
                *e = source_effect;
            }
        })
        .or_insert(source_effect);
}
```

In upstream (Part 2), ALL five aliasing effects use `Map.set()`:

```typescript
case 'Assign':
case 'Alias':
case 'Capture':
case 'CreateFrom':
case 'MaybeAlias': {
  // ...
  operandEffects.set(effect.from.identifier.id, isMutatedOrReassigned ? Effect.Capture : Effect.Read);
  operandEffects.set(effect.into.identifier.id, Effect.Store);
  break;
}
```

### Impact

Our priority merging means `Capture` (higher priority) wins over `Read` (lower priority) when the same identifier appears as `from` in multiple effects. Upstream's last-write-wins means the LAST effect determines the value.

This makes our effects MORE aggressive (Capture > Read), which creates WIDER effective ranges, not narrower. So this divergence contributes to over-MERGING (too few scopes), not over-splitting.

### Proposed Fix

Change the `Capture` branch to use `insert()` (last-write-wins), matching upstream:

```rust
AliasingEffect::Capture { from, into } => {
    // ...
    operand_effects.insert(from.identifier.id, source_effect);
}
```

Or better: merge the `Capture` branch into the same match arm as `Assign|Alias|CreateFrom|MaybeAlias`.

---

## Root Cause 3: Stale Operand Range in Start Fixup (LOW)

### The Problem

Phase 3 writes BFS-computed ranges back to `instr.lvalue.identifier.mutable_range` only. Operand identifiers inside instruction values are NOT updated.

In `apply_operand_start_fixup` (lines 955-962):

```rust
let fixup = |place: &mut Place| {
    if place.identifier.mutable_range.end > instr_id   // <-- reads STALE operand copy
        && place.identifier.mutable_range.start == InstructionId(0)
    {
        place.identifier.mutable_range.start = instr_id;
    }
};
```

In upstream, `operand.identifier.mutableRange.end` sees the BFS-extended value because JavaScript's reference semantics share the Identifier object.

### Impact

The start fixup may not fire for operands whose BFS-extended range wasn't written back to the operand copy. However, `infer_reactive_scope_variables` reads ranges from the `ranges` map (populated from lvalue ranges), not from operand copies. The impact on scope grouping is therefore minimal.

The more significant impact is on the `Effect` values assigned to operand `Place` objects, which downstream passes (like `propagate_scope_dependencies_hir`) may read. But this is a secondary concern compared to Root Cause 1.

### Proposed Fix

After the BFS writeback (Phase 3), update ALL operand identifiers' `mutable_range` from the `ranges` map. Alternatively, change `apply_operand_start_fixup` to look up from the `ranges` map:

```rust
let fixup = |place: &mut Place| {
    let range_end = ranges.get(&place.identifier.id)
        .map(|r| r.end)
        .unwrap_or(place.identifier.mutable_range.end);
    if range_end > instr_id && place.identifier.mutable_range.start == InstructionId(0) {
        place.identifier.mutable_range.start = instr_id;
    }
};
```

---

## Other Verified Non-Issues

### BFS Traversal Order (pop_back vs pop_front)

**Status: ALREADY FIXED.** Our code now uses `bufs.queue.pop_back()` (DFS/stack), matching upstream's `queue.pop()`.

### Forward Edge break vs continue

**Status: BENIGN.** Our `continue` is functionally equivalent to `break` because forward edges are monotonically ordered. `continue` does unnecessary work but produces the same results.

### Backward Edge Map vs Vec

**Status: BENIGN.** Upstream deduplicates backward edges by source identifier (keeping the first/lowest index). Our Vec keeps all edges. Since the BFS `seen` map deduplicates by identifier, and we also have the earliest edge, traversal results are equivalent.

### ensure_node vs null-check Skip

**Status: POTENTIALLY CREATES EXTRA EDGES.** Upstream's `assign/capture/maybeAlias` silently skip when either node doesn't exist. Our `ensure_node` creates nodes on demand, potentially creating edges between nodes that upstream would ignore. However, this makes ranges WIDER (not narrower), so it's not a cause of over-splitting.

### isMutatedOrReassigned Using Ranges Map

**Status: ALREADY FIXED.** Our `annotate_place_effects` correctly looks up from the `ranges` map (line 702), not from the stale operand `mutable_range` copy.

---

## Fixtures Traced

### 1. `alias-capture-in-method-receiver-and-mutate.js`

```javascript
let a = makeObject_Primitives();
let x = [];
x.push(a);
mutate(x);
return [x, a];
```

**Expected:** `_c(1)` -- all in one scope (sentinel)
**Our output (use_mutable_range=false):** `_c(1)` -- MATCHES

**Analysis:** This fixture has no function expressions, so Root Cause 1 (CreateFunction) doesn't apply. The `effective_range` workaround in `use_mutable_range=false` mode extends `a`'s range to overlap with `x`, creating one scope. With `use_mutable_range=true`, the BFS should extend `a`'s range through the Capture edge (`x.push(a)` captures `a` into `x`) and then through `mutate(x)` (MutateTransitive). If `a`'s range doesn't extend, it's because the BFS didn't propagate through the capture→mutation chain.

### 2. `alias-nested-member-path-mutate.js`

```javascript
let z = [];
let y = {};
y.z = z;
let x = {};
x.y = y;
mutate(x.y.z);
return x;
```

**Expected:** `_c(1)` -- all in one scope
**Our output (use_mutable_range=false):** `_c(1)` -- MATCHES

**Analysis:** Chain of property stores creates Alias edges: `z → y.z → x.y.z`. `mutate(x.y.z)` should propagate backward through these aliases to extend `z`, `y`, and `x`'s ranges. No function expressions, so Root Cause 1 doesn't apply directly.

### 3. `array-at-mutate-after-capture.js`

```javascript
let x = [42, {}];
const idx = foo(props.b);
let y = x.at(idx);
mutate(y);
return x;
```

**Expected:** `_c(2)` -- props.b dep + scope
**Our output (use_mutable_range=false):** `_c(2)` -- MATCHES

**Analysis:** `x.at(idx)` creates a MaybeAlias relationship between `x` and `y`. `mutate(y)` should propagate backward through MaybeAlias to extend `x`'s range. No function expressions.

### Common Pattern

The three traced fixtures all pass with `use_mutable_range=false` (effective_range workaround). To verify Root Cause 1's impact, fixtures with function expressions and captures are needed. The over-splitting regression likely affects fixtures like:
- Callback functions with captured mutable variables
- Array.map/filter with captured state
- Event handlers with closure captures

These patterns are common and would have `CreateFunction { captures: [...] }` effects, triggering the duplicate edge + index offset issue.

---

## Recommended Fix Order

1. **Remove CreateFunction capture edges** (Root Cause 1) -- highest impact, simple deletion
2. **Test with `use_mutable_range=true`** -- measure regression delta
3. **Fix Capture priority merging** (Root Cause 2) -- simple change, may fix additional fixtures
4. **Fix operand start fixup** (Root Cause 3) -- if regression persists after 1+2

---

## Validation Plan

After each fix:
1. `cargo test --release upstream_conformance -- --nocapture 2>&1 | tail -30`
2. Compare baseline (560/1717 with `use_mutable_range=false`)
3. Test `use_mutable_range=true` to measure regression delta
4. If regression < 10, the `effective_range` workaround can potentially be removed
5. Target: `use_mutable_range=true` matches or exceeds `use_mutable_range=false` results
