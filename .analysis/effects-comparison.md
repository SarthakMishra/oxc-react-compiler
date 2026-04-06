# Effects Comparison: InferMutationAliasingEffects

**Date:** 2026-04-05
**Goal:** Identify why our effects produce narrower BFS ranges than upstream
**Files compared:**
- Ours: `crates/oxc_react_compiler/src/inference/aliasing_effects.rs` (candidate effects)
- Ours: `crates/oxc_react_compiler/src/inference/infer_mutation_aliasing_effects.rs` (apply/resolve)
- Upstream: `compiler/packages/babel-plugin-react-compiler/src/Inference/InferMutationAliasingEffects.ts`

---

## Executive Summary

Six divergences were found between our effect generation and upstream. Three are **critical** (directly cause narrower BFS ranges and over-splitting), two are **medium** (may cause subtle differences), and one is **low** (cosmetic difference with minimal impact).

| # | Severity | Divergence | Impact |
|---|----------|-----------|--------|
| D1 | CRITICAL | CallExpression `mutates_function` is `false`, upstream is `true` | Callee excluded from ALL conservative effects |
| D2 | CRITICAL | Conservative fallback excludes `function` entirely when `!mutates_function` | Missing MaybeAlias and cross-arg Capture for function |
| D3 | CRITICAL | PropertyStore uses `MutateTransitive` instead of `Mutate`, missing lvalue Create | Different mutation depth + missing Primitive lvalue |
| D4 | MEDIUM | PropertyStore/Await effect ordering differs from upstream | BFS edge index interaction |
| D5 | MEDIUM | GetIterator missing `MutateTransitiveConditionally` for non-array types | Missing mutation on iterated values |
| D6 | LOW | StoreContext always uses Assign, not conditional Create/Mutate | Different abstract state for reassigned context vars |

---

## D1 [CRITICAL]: CallExpression `mutates_function` is `false`

### What differs

**Our code** (`aliasing_effects.rs:100`):
```rust
InstructionValue::CallExpression { callee, args, .. } => {
    effects.push(AliasingEffect::Apply {
        receiver: callee.clone(),
        function: callee.clone(),
        mutates_function: false,  // <-- WRONG
        ...
    });
}
```

**Upstream** (`InferMutationAliasingEffects.ts`):
```typescript
} else if (value.kind === 'CallExpression') {
  callee = value.callee;
  receiver = value.callee;
  mutatesCallee = true;  // <-- CORRECT
}
```

### Why it causes narrower ranges

For a call like `foo(x)`, upstream sets `mutatesFunction: true`, which means:
- The callee `foo` gets `MutateTransitiveConditionally` applied
- The callee gets `MaybeAlias` to the return value
- The callee participates in cross-arg `Capture` with all other operands

Our code sets `mutates_function: false`, AND due to D2 below, the function is completely excluded from the operand list. This means fewer graph edges, fewer aliasing relationships, and narrower BFS ranges.

### Proposed fix

Change `mutates_function: false` to `mutates_function: true` for `CallExpression`.

### Expected impact

HIGH -- this affects every `CallExpression` in the program. For fixtures like `capturing-func-mutate-nested.js` where a locally-defined function captures mutable state and is then called, the callee's mutable range needs to extend to cover the call site.

---

## D2 [CRITICAL]: Conservative fallback excludes `function` entirely

### What differs

**Our code** (`infer_mutation_aliasing_effects.rs:1280-1287`):
```rust
let mut operands: Vec<Place> = Vec::new();
if mutates_function {
    operands.push(function.clone());
}
operands.push(receiver.clone());
for arg in args {
    operands.push(arg.clone());
}
```

**Upstream** (Apply conservative fallback):
```typescript
for (const arg of [effect.receiver, effect.function, ...effect.args]) {
    // ...
    if (operand !== effect.function || effect.mutatesFunction) {
        applyEffect(/* MutateTransitiveConditionally */);
    }
    applyEffect(/* MaybeAlias to return */);
    // cross-arg Capture...
}
```

### The key difference

Upstream ALWAYS includes `function` in the iteration list. It only SKIPS `MutateTransitiveConditionally` for the function when `!mutatesFunction`. The function still gets:
- `MaybeAlias { from: function, into: return_value }` -- aliasing the callee to the return
- `Capture { from: function, into: other_args }` -- cross-arg aliasing

Our code completely removes `function` from the operand list when `!mutates_function`, so it gets NONE of these effects.

### Why it causes narrower ranges

For MethodCall where `receiver != function`, the function (property accessor) should still participate in aliasing. For example, `obj.method(x)` -- the method itself could alias to the return value.

Even for CallExpression where `receiver == function`, the deduplication in upstream via `other === arg` identity checks naturally avoids duplicate work. Our code achieves the wrong result by exclusion.

### Proposed fix

Always include function in the operand list. Only conditionally skip `MutateTransitiveConditionally`:

```rust
let mut operands: Vec<Place> = Vec::new();
operands.push(receiver.clone());
// Always include function (even if same as receiver, upstream uses identity checks)
operands.push(function.clone());
for arg in args {
    operands.push(arg.clone());
}

// MutateTransitiveConditionally each operand (skip function if !mutates_function)
for operand in &operands {
    if operand.identifier.id == function.identifier.id && !mutates_function {
        continue;
    }
    apply_effect(/* MutateTransitiveConditionally */);
}

// MaybeAlias and cross-arg Capture on ALL operands (including function)
```

Note: since we use value equality (IdentifierId) rather than reference identity, we need to be careful about deduplication when receiver == function (CallExpression case). Upstream relies on JavaScript reference identity (`other === arg`) which naturally deduplicates when both are the same object reference.

---

## D3 [CRITICAL]: PropertyStore uses wrong mutation variant

### What differs

**Our code** (`aliasing_effects.rs:144-152`):
```rust
InstructionValue::PropertyStore { object, value, .. } => {
    effects.push(AliasingEffect::Capture { from: value, into: object });
    effects.push(AliasingEffect::MutateTransitive { value: object });
}
```

**Upstream**:
```typescript
case 'PropertyStore': {
    effects.push({kind: 'Mutate', value: value.object, reason: mutationReason});
    effects.push({kind: 'Capture', from: value.value, into: value.object});
    effects.push({kind: 'Create', into: lvalue, value: ValueKind.Primitive, reason: ValueReason.Other});
}
```

### Three sub-divergences

1. **Mutation variant**: Upstream uses `Mutate` (shallow), we use `MutateTransitive` (deep transitive). `MutateTransitive` propagates through all aliases, while `Mutate` only affects the direct value. When the abstract state shows the object is frozen, `MutateTransitive` produces a `MutateFrozen` error (correct behavior), but `Mutate` also does -- the difference is subtle in the BFS graph.

2. **Effect ordering**: Upstream has `Mutate, Capture, Create`. We have `Capture, MutateTransitive`. The BFS `edge.index >= mutation_index` filter means Capture edges created BEFORE the mutation get included in the BFS traversal, while those AFTER get excluded. Our ordering (Capture before Mutate) intentionally differs from upstream to include the capture edge, but this may produce different results when combined with the wrong mutation variant.

3. **Missing lvalue Create**: Upstream creates the instruction lvalue as `Primitive`. Our code doesn't create the lvalue for PropertyStore at all. The fallback in `apply_signature` (line 837) creates a default `Primitive` lvalue, but this happens AFTER all effects are applied, which may affect the abstract state during effect application.

### Why it causes narrower ranges

The combination of wrong mutation variant + different ordering + missing lvalue Create could cause the BFS to compute different edge sets and mutation indices, resulting in narrower or different ranges.

### Proposed fix

1. Change `MutateTransitive` to `Mutate` for PropertyStore
2. Add `Create { into: lvalue, value: Primitive }` effect
3. Change ordering to: Mutate, Capture, Create (matching upstream)
4. Same changes for ComputedStore

### Expected impact

MEDIUM-HIGH -- PropertyStore is common in real code (`x.y = z`, `obj.prop = val`). The `property-assignment.js` fixture directly tests this pattern.

---

## D4 [MEDIUM]: Effect ordering differences

### Await

**Our code** (`aliasing_effects.rs:221-231`):
```
Create, Capture, MutateTransitiveConditionally
```

**Upstream**:
```
Create, MutateTransitiveConditionally, Capture
```

The BFS uses edge indices to determine which edges to follow during mutation propagation. Capture edges with index >= mutation_index are skipped. Our ordering puts Capture BEFORE MutateTransitiveConditionally (lower index), meaning the Capture edge IS followed during mutation propagation. Upstream puts Capture AFTER, so it's skipped.

This is the OPPOSITE of what we did for PropertyStore (where we intentionally put Capture first). The inconsistency suggests we should match upstream ordering everywhere.

### Proposed fix

For Await, reorder to: Create, MutateTransitiveConditionally, Capture.

---

## D5 [MEDIUM]: GetIterator missing MutateTransitiveConditionally

### What differs

**Our code** (`aliasing_effects.rs:280-291`):
```rust
InstructionValue::GetIterator { collection } => {
    effects.push(AliasingEffect::Create { into: lvalue, ... });
    effects.push(AliasingEffect::Alias { from: collection, into: lvalue });
}
```

**Upstream**:
```typescript
case 'GetIterator': {
    effects.push({kind: 'Create', into: lvalue, ...});
    if (isArrayType(collection) || isMapType(collection) || isSetType(collection)) {
        effects.push({kind: 'Capture', from: collection, into: lvalue});
    } else {
        effects.push({kind: 'Alias', from: collection, into: lvalue});
        effects.push({kind: 'MutateTransitiveConditionally', value: collection});
    }
}
```

### Two sub-divergences

1. **Missing type check**: Upstream differentiates Array/Map/Set (use `Capture`) from other iterables (use `Alias` + `MutateTransitiveConditionally`). We always use `Alias` without the mutation.

2. **Missing `MutateTransitiveConditionally`**: For non-array iterables, calling `Symbol.iterator` may mutate the collection. We omit this mutation effect.

### Why it causes narrower ranges

For-of loops over non-array iterables would miss the mutation edge on the collection, resulting in the collection's mutable range not extending through the loop body.

### Proposed fix

Since we don't have type information at this point, default to the conservative path (Alias + MutateTransitiveConditionally) which is what upstream does for unknown types.

---

## D6 [LOW]: StoreContext always uses Assign

### What differs

**Our code** (`aliasing_effects.rs:54-59`):
```rust
InstructionValue::StoreContext { lvalue: store_lvalue, value } => {
    effects.push(AliasingEffect::Assign { from: value, into: store_lvalue });
    effects.push(AliasingEffect::Capture { from: value, into: store_lvalue });
    effects.push(AliasingEffect::Assign { from: value, into: lvalue });
}
```

**Upstream**:
```typescript
case 'StoreContext': {
    if (value.lvalue.kind === InstructionKind.Reassign || 
        context.hoistedContextDeclarations.has(value.lvalue.place.identifier.declarationId)) {
        effects.push({kind: 'Mutate', value: value.lvalue.place});
    } else {
        effects.push({kind: 'Create', into: value.lvalue.place, value: ValueKind.Mutable, ...});
    }
    effects.push({kind: 'Capture', from: value.value, into: value.lvalue.place});
    effects.push({kind: 'Assign', from: value.value, into: lvalue});
}
```

### Key difference

Upstream differentiates between:
- **Reassign/hoisted**: `Mutate` on the context place (it already exists, we're changing it)
- **First assignment**: `Create(Mutable)` on the context place (initializing it)

Our code always uses `Assign`, which has different abstract state semantics:
- `Create` allocates a new abstract value
- `Assign` makes the destination point to the source's values
- `Mutate` checks frozen/global status and may produce error effects

### Why it causes narrower ranges

For reassigned context variables (e.g., `let` bindings in closures), upstream's `Mutate` would extend the mutable range of the context variable, while our `Assign` doesn't produce any mutation effect.

### Proposed fix

Track whether StoreContext is a reassignment (check `InstructionKind::Reassign` on the lvalue) and emit `Mutate` instead of `Assign` in that case.

---

## Fixture Trace: `capturing-func-mutate-nested.js`

```javascript
function component(a) {
  let y = {b: {a}};        // y is Mutable, inner {a} captures param a
  let x = function () {
    y.b.a = 2;              // Captures y, mutates y.b.a
  };
  x();                      // Calls x -- should extend y's mutable range
  return y;
}
```

**Expected**: Everything in ONE scope (ours=5, expected=2 means we over-split by 3).

### What upstream does

1. `y = {b: {a}}` -- Create(Mutable) for y, Capture(a -> y)
2. `x = function() {...}` -- CreateFunction with captures=[y], x is Mutable (captures mutable y)
3. `x()` -- Apply with `mutatesFunction: true` (CallExpression), conservative fallback:
   - `MutateTransitiveConditionally(x)` -- x is Mutable, so this IS a mutation
   - `MaybeAlias(x -> return)` -- aliasing x to return value
   - Since x captures y, the transitive mutation on x propagates through the capture edge to y
   - **Key**: because `mutatesFunction: true`, the callee x gets MutateTransitiveConditionally

### What our code does

1. `y = {b: {a}}` -- Same as upstream
2. `x = function() {...}` -- CreateFunction with ALL context vars (not filtered by Effect.Capture), x is Mutable
3. `x()` -- Apply with `mutates_function: false` (WRONG):
   - function x is NOT in operand list at all (D2)
   - Only receiver (= x) is in operand list
   - `MutateTransitiveConditionally(x)` -- x is Mutable, so this IS a mutation
   - Wait -- receiver IS x, so it does get mutated... but the MaybeAlias and cross-arg effects differ

Actually, for CallExpression, receiver == function == callee. So even with our bug, receiver IS added to the operand list. The key issue is that D1+D2 together mean that for `mutates_function: false`:
- Upstream iterates [receiver, function, ...args] = [x, x, ...args], with TWO entries for x
- Ours iterates [receiver, ...args] = [x, ...args], with ONE entry for x

The duplicate doesn't matter for MutateTransitiveConditionally/MaybeAlias (same operand = same effect). BUT for cross-arg Capture, upstream's loop would try to Capture(x -> x) and skip via `other === arg` identity check. Effectively, there's no difference for CallExpression with no args.

So for `x()` with no args, the D1+D2 divergence has NO practical effect. The over-splitting must come from somewhere else.

**Revised theory**: The over-splitting in `capturing-func-mutate-nested.js` is likely due to:
- D3 (PropertyStore `y.b.a = 2` inside the function uses MutateTransitive instead of Mutate)
- The function's aliasing effects being computed differently
- Or the function expression resolution (Step 1 in apply_call_effect) producing different effects

The function `x` has a known `aliasingEffects` (from its inner body analysis). Upstream resolves this via `buildSignatureFromFunctionExpression` and `computeEffectsForSignature`. Our `try_resolve_function_expression` may not find the aliasing effects if the inner function wasn't analyzed yet.

---

## Fixture Trace: `property-assignment.js`

```javascript
function Component(props) {
  const x = {};          // Create(Mutable)
  const y = [];          // Create(Mutable)
  x.y = y;               // PropertyStore: Mutate(x), Capture(y -> x)
  const child = <Component data={y} />;  // JSX: Freeze(y), Capture(y -> child)
  x.y.push(props.p0);   // MethodCall on x.y with arg props.p0
  return <Component data={x}>{child}</Component>;
}
```

**Expected**: ONE scope for everything except the return JSX.

### D3 impact on PropertyStore `x.y = y`

Upstream: `Mutate(x)` -- shallow mutation, x stays Mutable
Ours: `MutateTransitive(x)` -- deep transitive mutation

After `x.y = y`, both y and x are aliased. When `x.y.push(...)` is called later, the mutation should propagate to both x and y.

The key issue here is that after JSX freezes y (`Freeze(y, JsxCaptured)`), the `x.y.push(props.p0)` call mutates the now-frozen y. Upstream uses `Mutate` (shallow) for PropertyStore which doesn't propagate through aliases the way `MutateTransitive` does, potentially causing different abstract state at the `push` call.

---

## Priority-Ordered Fix Plan

1. **D1**: Change `mutates_function: false` to `true` for `CallExpression` in `aliasing_effects.rs:100`
2. **D2**: Restructure conservative fallback to always include function, skip only MutateTransitiveConditionally
3. **D3**: Change PropertyStore from `MutateTransitive` to `Mutate`, add lvalue Create, fix ordering
4. **D5**: Add `MutateTransitiveConditionally` to GetIterator
5. **D4**: Fix Await effect ordering to match upstream
6. **D6**: Add reassign/create distinction to StoreContext (requires tracking InstructionKind)

Fixes D1-D3 are expected to have the highest impact on the -57 regression.

---

## Additional Minor Divergences Noted

### FunctionExpression capture filtering

Upstream filters captures: `value.loweredFunc.func.context.filter(operand => operand.effect === Effect.Capture)`
Our code includes ALL context variables: `lowered_func.context.clone()`

This means we include Read-only context variables in the CreateFunction captures list. During `apply_effect` for CreateFunction, each capture gets a Capture effect applied, which may be downgraded to ImmutableCapture if the captured value is frozen. The extra captures don't cause harm (they become ImmutableCapture), but they differ from upstream's filtering.

### CreateFunction side effect / ref checks

Upstream checks `hasTrackedSideEffects` and `capturesRef` to determine if the function is mutable. Our code only checks `has_mutable_captures`. A function that captures a ref or has impure side effects should be treated as mutable even if all captures are frozen.

### PropertyDelete missing lvalue Create

Upstream creates lvalue as Primitive for PropertyDelete. Our code emits only `Mutate` without creating the lvalue. The fallback in `apply_signature` handles this, but the timing differs.
