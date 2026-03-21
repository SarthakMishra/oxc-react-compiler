# oxc-react-compiler Backlog

> Last updated: 2026-03-21 (post Phase 110)
> Conformance: **456/1717 (26.6%)**. Render: **96% (24/25)**. E2E: **95-100%**. Tests: all pass, 0 panics.

---

## Critical Architecture Notes

**Read these before making ANY changes.**

### `effective_range` vs `mutable_range` — DO NOT CHANGE
File: `src/reactive_scopes/infer_reactive_scope_variables.rs`

Scope grouping uses `effective_range = max(mutable_range.end, last_use + 1)` instead of upstream's pure `mutable_range`. Switching to narrow `mutable_range` has been tried **4 times** and always causes render to drop 96%→36%.

### `collect_all_scope_declarations` is load-bearing
File: `src/reactive_scopes/codegen.rs`

Pre-declares ALL scope output variables at function level. Removing it causes render to drop 96%→24%.

### `rename_variables` is load-bearing for 17 fixtures
File: `src/reactive_scopes/prune_scopes.rs`

Disabling `rename_variables` entirely causes 456→439 (-17 regressions). These 17 fixtures NEED the temp rename + post-scope assignment pattern. The remaining 28 "const t0 vs const name" divergences need selective skip.

### OXC stores default params in `FormalParameter.initializer`
NOT in `BindingPattern::AssignmentPattern`. The `AssignmentPattern` variant in `BindingPattern` is only for destructure defaults in variable declarations.

### Block iteration order ≠ source order for loops
The HIR blocks are stored in creation order, but for-loop constructs create blocks out of source order. The frozen mutation validator uses `frozen_at` instruction ID tracking to enforce source ordering. Any new validator that walks blocks linearly must account for this.

### Cross-scope `IdentifierId` mismatch
Nested function bodies have their own `IdentifierId` numbering. A variable captured from an outer scope has DIFFERENT `IdentifierId`s at each scope level. Name-based resolution is needed for cross-scope tracking. This blocks 7 global-reassign + parts of 8 ref-access validators.

### Build & test
```bash
cargo test                                            # All Rust tests
cargo test --test conformance_tests -- --nocapture    # Conformance (1717 fixtures)
cargo insta test --accept                             # Update snapshots
cd napi/react-compiler && npx @napi-rs/cli build --release  # Rebuild NAPI
cd benchmarks && npm run render:compare               # Render comparison
cd benchmarks && npm run e2e:quick                    # E2E Vite builds
```

---

## Conformance Breakdown (456 passing, 1261 failing)

| Category | Count | Description |
|----------|-------|-------------|
| Both compile, slots DIFFER | ~649 | Scope/memoization divergence (biggest) |
| Both compile, slots MATCH | ~240 | Output format only |
| We bail, they compile | ~135 | False bail-outs |
| We compile, they don't | ~149 | Over-compile (usually fine) |
| Both no memo, format diff | ~87 | Format-only — **all 87 produce identical raw text** |
| Silent bail-outs | 23 | 0 scopes, no error |

### Bail-out error breakdown (135 fixtures)
```
58x  Existing memoization could not be preserved  ← BLOCKED on scope inference
23x  Frozen mutation                               ← 19 false bail-outs; Check 1 (aliasing) is 13 of those
23x  (silent, no error)                            ← HIR lowering gaps
 8x  Reassigned after render                       ← sync method callbacks fixed; remaining need invoke() pattern
 7x  Cannot reassign outside component             ← cross-scope IdentifierId blocker
 6x  Ref access in render                          ← 8 false bail-outs remain
 3x  Hooks referenced as values
 3x  Cannot call setState during render
 2x  setState in useEffect
 2x  Other
```

---

## Next Steps (Prioritized)

### Step 1: "Both No Memo" Normalization Fix (87 fixtures, QUICK WIN)

**Impact:** Up to **+87 conformance** (456→543)
**Files:** `tests/conformance_tests.rs` (normalization logic)
**Risk:** LOW — only changes test comparison, not compiler code
**Confidence:** HIGH — all 87 fixtures produce identical raw text

**The finding:** All 87 "both no memo" fixtures produce raw text IDENTICAL to the expected output. They diverge only after the `normalize_via_oxc` step (OXC parse→transform→print roundtrip). This means the OXC transformer processes our output and the expected output slightly differently — likely due to: import statement reformatting, JSX transform differences, semicolon insertion, or expression statement normalization.

**Approach:**
1. Pick 3 fixtures and dump pre/post normalization for both sides to identify the exact normalization divergence
2. If the divergence is in the normalizer (e.g., OXC transformer changes import ordering or adds semicolons), fix the `normalize_output` function to compensate
3. If the divergence is in the OXC transformer itself (e.g., JSX lowering produces different code for semantically identical input), add targeted normalization rules

**Why this is the #1 priority:** The largest single batch of potential conformance gains. No compiler changes needed. Pure test infrastructure fix.

---

### Step 2: Named Variable Preservation — Conditional Rename (28 fixtures, MEDIUM)

**Impact:** Up to +28 conformance
**Files:** `src/reactive_scopes/prune_scopes.rs` (`can_rename_scope_decl`)
**Risk:** MEDIUM — must not regress the 17 rename-dependent fixtures

**Approach (b):** Identify the 17 fixtures that NEED rename and add a condition to `can_rename_scope_decl`:
1. Run conformance with `rename_variables` disabled
2. Diff the passing list against baseline to find the 17 regressions
3. Study what pattern those 17 share (likely: scope output is used as a dependency check value, and upstream emits `$[N] !== t0` not `$[N] !== x`)
4. Add the condition: only rename when the scope output appears in a dependency check AND the original name would collide with a later declaration
5. Validate: no regressions on the 17, +28 from the rest

---

### Step 3: Frozen Mutation — Targeted Fixes (6 fixtures, MEDIUM)

**Impact:** +3 to +6 conformance

**3a. Check 4 (Nested FE mutation, 3 fixtures):**
Track which outer variables are freshly allocated (from ArrayExpression, ObjectExpression, NewExpression). FEs that mutate these should not be flagged since the value is new and not frozen. Add a `locally_allocated_ids: FxHashSet<IdentifierId>` computed before Check 4, skip when the mutated name maps to a locally-allocated ID.

**3b. Unknown path (2 fixtures):**
`hook-ref-callback.js` and `useMemo-multiple-returns.js` — add debug instrumentation to find which error path fires. Likely a check that's not covered by the existing 5-check debug framework.

**3c. Check 2 (while-loop, 1 fixture):**
`repro-missing-dependency-if-within-while.js` — the instruction ID ordering fix didn't help because the freeze happens via name-based tracking (not ID-based). Needs investigation of the name-based freeze path.

---

### Step 4: Cross-Scope IdentifierId Resolution (FOUNDATIONAL, +10-15 fixtures)

**Impact:** Unblocks 7 global-reassign + parts of 8 ref-access + parts of 8 reassigned-after-render
**Files:** `src/validation/function_context.rs` (new shared utility)
**Risk:** LOW — additive utility, no existing code changed

**The problem:** When `onClick` is defined in the component and referenced inside a `useMemo` callback, the two scopes use different `IdentifierId`s for the same variable. Current workaround: name-based resolution (works for direct chains, fails for multi-hop aliases).

**Approach:** Build a **cross-scope identity map** during HIR construction:
1. In the HIR builder, when creating a nested function builder, pass the parent's `name → IdentifierId` binding map
2. When the nested builder encounters a `LoadLocal` for a captured variable, record a mapping: `inner_id → outer_id`
3. Store this map on the `HIRFunction` (new field: `captured_id_map: FxHashMap<IdentifierId, IdentifierId>`)
4. Validators can use this map to resolve inner IDs back to outer IDs
5. This eliminates the need for name-based workarounds in ref-access, global-reassign, and reassigned-after-render validators

---

### Step 5: Optional Chaining — Remaining (5 fixtures, MEDIUM)

**Remaining:** 3 try-catch + optional, 1 nested optional member, 1 computed optional.

**5a. Try-catch lowering (3 fixtures):** `try-catch-logical-and-optional.js`, `try-catch-nested-optional-chaining.js`, `try-catch-optional-call.js`. These need basic try-catch HIR lowering — emit the try body, catch body as separate blocks. Non-trivial but well-scoped. Upstream lowers try-catch to sequential blocks with error handling.

**5b. Nested optional + computed (2 fixtures):** May need deeper investigation of how OXC represents nested optional chains in the AST.

---

### Step 6: Silent Bail-outs — Non-Flow Categories (9 fixtures, VARIES)

After excluding Flow (5x) and gating (4x) fixtures, 9 silent bail-outs remain:
- `infer-functions-component-with-ref-arg.js` — `@compilationMode:"infer"` with 2 params
- `repro-mutate-ref-in-function-passed-to-hook.js` — ref mutation pattern
- `return-ref-callback-structure.js` / `return-ref-callback.js` — ref callback returns
- `useCallback-ref-in-render.js` / `useImperativeHandle-ref-mutate.js` — hook patterns
- `repro-invalid-destructuring-reassignment-undefined-variable.js` — Flow `@compilationMode:"infer"`
- `unused-object-element-with-rest.js` — scope inference gap (rest spread doesn't create scope)
- `hoist-destruct.js` — Flow `component` keyword

Most of these are either Flow-dependent or scope inference gaps (BLOCKED). Only 2-3 are potentially fixable.

---

### Step 7: Frozen Mutation Check 1 — Aliasing Pass (13 fixtures, HARD)

**Impact:** Up to +13 conformance
**Files:** `src/inference/infer_mutation_aliasing_effects.rs`
**Risk:** HIGH — touches the same infrastructure as the BLOCKED Gap 11

**Root cause:** The aliasing pass over-propagates freeze effects:
- IIFE captures propagate freeze to outer variables (6 fixtures)
- Method call results inherit frozen status from receiver (2 fixtures)
- Transitive mutation through identity/propertyload functions (3 fixtures)
- Other (2 fixtures)

**Approach:** Targeted fix for method call results:
- In the aliasing pass, when processing a `MethodCall` or `CallExpression`, the return value should NOT inherit the `Freeze` status of the receiver/arguments
- New objects created by function calls are unfrozen by default
- This is a narrow change but must be validated against the render benchmark (96% → no regression)

---

## BLOCKED — Do Not Attempt

### Under-memoization (Gap 11, ~404 fixtures) — FOUNDATIONAL BLOCKER
Port upstream's ~2000-line abstract interpreter from `src/Inference/InferMutationAliasingEffects.ts`. 4 prior attempts, all reverted (96%→36% render regression). Also unblocks Gap 5a (+58) and may reduce Gap 7.

### Over-memoization (Gap 7, ~175 fixtures)
May self-resolve as side effect of Gap 11.

### Memoization Preservation (Gap 5a, 58 fixtures)
HARD DEPENDENCY on Gap 11.

### Ternary Reconstruction (Gap 6)
P4 cosmetic only. No functional impact.

---

## Key File Reference

| Purpose | Path |
|---------|------|
| Pipeline orchestration | `src/entrypoint/pipeline.rs` |
| HIR types | `src/hir/types.rs` |
| HIR builder (AST→HIR) | `src/hir/build.rs` |
| Code generation | `src/reactive_scopes/codegen.rs` |
| Scope grouping | `src/reactive_scopes/infer_reactive_scope_variables.rs` |
| Scope dependencies | `src/reactive_scopes/propagate_dependencies.rs` |
| Scope pruning + rename | `src/reactive_scopes/prune_scopes.rs` |
| Frozen mutation validation | `src/validation/validate_no_mutation_after_freeze.rs` |
| Ref access validation | `src/validation/validate_no_ref_access_in_render.rs` |
| Global reassignment validation | `src/validation/validate_no_global_reassignment.rs` |
| Hooks usage validation | `src/validation/validate_hooks_usage.rs` |
| Shared function context | `src/validation/function_context.rs` |
| Memoization validation | `src/validation/validate_preserved_manual_memoization.rs` |
| Conformance runner | `tests/conformance_tests.rs` |
| Known failures | `tests/conformance/known-failures.txt` |

All paths relative to `crates/oxc_react_compiler/`.
