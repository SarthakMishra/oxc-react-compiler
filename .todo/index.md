# oxc-react-compiler Backlog

> Last updated: 2026-03-20 (post Phase 107)
> Conformance: **451/1717 (26.3%)**. Render: **96% (24/25)**. E2E: **95-100%**. Tests: all pass, 0 panics.

---

## Critical Architecture Notes

**Read these before making ANY changes.**

### `effective_range` vs `mutable_range` — DO NOT CHANGE
File: `src/reactive_scopes/infer_reactive_scope_variables.rs`

Scope grouping uses `effective_range = max(mutable_range.end, last_use + 1)` instead of upstream's pure `mutable_range`. Switching to narrow `mutable_range` has been tried **4 times** and always causes render to drop 96%→36%. Each attempt used different compensating passes. The root cause: upstream's abstract interpreter produces wider mutation ranges. Our `effective_range` approximation compensates.

### `collect_all_scope_declarations` is load-bearing
File: `src/reactive_scopes/codegen.rs`

Pre-declares ALL scope output variables at function level. Removing it causes render to drop 96%→24%.

### Validation relaxation causes regressions
Attempted relaxing 3 different validators. ALL caused conformance drops. DO NOT relax validation without fixing scope inference first.

### `rename_variables` is load-bearing for 17 fixtures
File: `src/reactive_scopes/prune_scopes.rs`

Disabling `rename_variables` entirely causes 451→434 (-17 regressions). These 17 fixtures NEED the temp rename + post-scope assignment pattern to match upstream. Selectively disabling rename for the 28 "const t0 vs const name" fixtures is the correct approach but requires identifying which specific declarations need rename. The slot distributions are unchanged — regressions are text-only.

### OXC stores default params in `FormalParameter.initializer`
NOT in `BindingPattern::AssignmentPattern` as one might expect from the ESTree spec. The `AssignmentPattern` variant in `BindingPattern` is only used for destructure defaults in variable declarations, not function parameter defaults.

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

## Conformance Breakdown (451 passing, 1266 failing)

| Category | Count | Description |
|----------|-------|-------------|
| Both compile, slots DIFFER | ~637 | Scope/memoization divergence (biggest) |
| Both compile, slots MATCH | ~248 | Output format only |
| We bail, they compile | ~144 | False bail-outs |
| We compile, they don't | ~149 | Over-compile (usually fine) |
| Both no memo, format diff | ~87 | Format-only |
| Silent bail-outs | 23 | 0 scopes, no error |

### Bail-out error breakdown (144 fixtures)
```
58x  Existing memoization could not be preserved  ← BLOCKED on scope inference
29x  Frozen mutation                               ← per-fixture investigation needed
23x  (silent, no error)                            ← HIR lowering gaps (was 28, -5 from default params)
 6x  Ref access in render                          ← was 14, -8 from JSX prop + transitive safety fix
 7x  Cannot reassign outside component             ← was 8, -1 from recursive safe callback fix
10x  Reassigned after render (x/y/count/myVar)     ← partially addressed
 3x  Hooks referenced as values                    ← validator audit needed
 3x  Cannot call setState during render            ← validator audit needed
 2x  setState in useEffect                         ← validator audit needed
 3x  Other                                         ← misc
```

---

## Recently Completed (Phase 107: structural improvements, 0 conformance regression)

- [x] **Default parameter desugaring** (`build.rs`): Function param defaults via `FormalParameter.initializer`, array destructure defaults via `emit_array_destructure_with_defaults()`, assignment expression defaults for `AssignmentTargetWithDefault`. 5 fixtures: silent bail-out → compiling (28→23).
- [x] **Name promotion for non-inlinable temps** (`codegen.rs`): `build_name_promotion_map()` detects temp→StoreLocal patterns and promotes temp names to user variable names. `let t6 = function() {...}; let x = t6;` → `let x = function() {...};`. 6 fixtures improved in naming quality.
- [x] **Recursive safe callback detection** (`validate_no_global_reassignment.rs`): `collect_safe_ids_recursive()` + `collect_id_aliases_recursive()` + `collect_safe_callback_names()` scan nested function bodies for JSX event handlers and effects. Name-based cross-scope resolution via `id_to_assigned_name` map. 1 false bail-out fixed (8→7).
- [x] **Ref-access validator overhaul** (`validate_no_ref_access_in_render.rs`): ALL JSX props treated as non-render contexts (not just `onXxx` + `ref`). Transitive safety propagation via `collect_callee_names()` — FEs called only from non-render callbacks marked safe. 8 false bail-outs eliminated (16→8).

---

## Step 1: ~~Silent Bail-outs — Default Parameters~~ DONE

Implemented. 5 fixtures now compile (silent bail-outs 28→23). Output has correct slot counts but text diffs remain (naming/format).

---

## Step 2: Frozen Mutation Per-Fixture Triage (29 fixtures, HARD)

**Impact:** Up to +29 conformance
**Files:** `src/validation/validate_no_mutation_after_freeze.rs`
**Risk:** MEDIUM — must not break `error.*` fixtures

The validator has 5 check paths. Each of the 29 fixtures triggers a DIFFERENT check for a different reason. Not a bulk fix.

**Approach:** Add debug instrumentation to print which check fires + the frozen identifier for each fixture. Group by root cause, then fix the largest group.

**Categorized patterns:**
- 6x IIFE capture/alias — IIFE detection works but aliasing effects on outer variables still trigger
- 4x switch fall-through — mutation after JSX in switch cases
- 3x new-mutability — transitive mutation through identity/propertyload functions
- 2x method call results — `props.object.makeObject()` result inherits frozen status (shouldn't)
- 2x rest/spread — `{...rest}` creates new object, shouldn't be frozen (attempted fix, freeze comes from different path)
- 12x misc — ref callbacks, loop collections, parameter mutations

**Validator check paths:**
1. `MutateFrozen` effect from aliasing pass (Check 1, line ~310)
2. `Freeze` effect propagation + subsequent mutation (Check 2, line ~324)
3. `MethodCall` on frozen receiver with mutating method (Check 3, line ~386)
4. `PropertyStore`/`ComputedStore`/`Delete` on frozen value (Check 4, line ~398)
5. `Mutate`/`MutateTransitive` effect on SSA-frozen value (Check 5, line ~445)

**Upstream:** `src/Validation/ValidateFrozenValues.ts`

---

## Step 3: ~~Ref-Access Validator Fix~~ PARTIALLY DONE

**Was:** 14 false bail-outs. **Now:** 6 false bail-outs (8 fixed).

**What was done:** Extended `collect_non_render_callback_ids` to treat ALL JSX prop values as non-render (not just event handlers). Added transitive safety propagation through call chains. All 3 `error.*` regression fixtures preserved.

**Remaining 8 false positives:**
- `allow-ref-type-cast-in-render.js` — Flow type cast `(ref: any)`, needs type cast handling
- `bug-ref-prefix-postfix-operator.js` — `ref.current++` pattern
- `ref-current-aliased-no-added-to-dep.js` — aliased ref in callback passed as JSX prop
- `ref-current-aliased-not-added-to-dep-2.js` — same pattern, different variant
- `ref-current-not-added-to-dep-2.js` — ref access in nested arrow, escaped via JSX prop
- `valid-setState-in-useEffect-via-useEffectEvent-with-ref.js` — useEffectEvent pattern (not yet supported)
- `capture-ref-for-later-mutation.tsx` — ref mutation in function returned from component (deferred)
- `import-as-local.tsx` — imported ref alias pattern

**Blocker for remaining 8:** These involve deeper patterns (Flow type casts, `useEffectEvent`, ref aliasing through multiple indirections). Each is a separate fix. The transitive safety propagation catches callees by NAME within non-render FE bodies, but the cross-scope ID mismatch prevents catching some patterns where the FE is loaded via `LoadLocal` with a different `IdentifierId` than the outer scope's definition.

---

## Step 4: Other False Bail-outs Audit (20 fixtures remaining, MEDIUM)

**Impact:** Up to +20 conformance (was 28, -8 from ref-access and global-reassign fixes)
**Files:** Various validation passes
**Risk:** LOW per category

**Breakdown (updated):**
- 7x "Cannot reassign variables declared outside" → `validate_no_global_reassignment.rs` — remaining 7 need **transitive safe callback analysis** across scope boundaries. The `collect_safe_callback_names()` approach works for direct JSX prop → FE chains but NOT for indirect patterns like `useEffect(() => { setGlobal() })` where `setGlobal` is a FE called inside an effect callback. Cross-scope `IdentifierId` mismatch is the root cause (same as ref-access blocker).
- 3x "Hooks may not be referenced as normal values" → `validate_hooks_usage.rs`
- 3x "Cannot call setState during render" → `validate_no_set_state_in_render.rs`
- 2x "setState is called directly inside useEffect" → `validate_no_set_state_in_effects.rs`
- 1x "useMemo called conditionally" → `validate_hooks_usage.rs`
- 1x "Cannot freeze mutable function" → `validate_no_mutation_after_freeze.rs`
- 3x other

**Approach:** For each category, read the upstream validation source and compare against our implementation. Most are small exemptions or edge cases. The hooks/setState categories (8 fixtures total) are likely quick wins.

---

## Step 5: Named Variable Preservation (28 fixtures remaining, HARD)

**Impact:** Up to +28 conformance (was 34, -6 from name promotion)
**Files:** `src/reactive_scopes/codegen.rs` (`build_inline_map`, `rename_variables`)
**Risk:** HIGH — changes interact with `rename_variables` which is load-bearing for 17 fixtures

**What was done:** Added `build_name_promotion_map()` to codegen that promotes temp→user-name when a non-inlinable temp (FunctionExpression) is immediately stored to a named variable. Fixed 6 fixtures.

**Remaining 28:** These are from `rename_variables` which renames scope declaration outputs to temps (`x` → `t10`) and creates post-scope assignments (`let x = t10`). Upstream uses the original name directly in the scope.

**Investigation findings:**
- Disabling `rename_variables` entirely: 451→434 (-17 regressions). The 17 regressions are text-only (slot counts match).
- The fix requires either: (a) teaching `codegen_scope` to reverse-promote scope declaration names (complex — would need to clone+mutate the scope block and apply promotions to all nested identifiers), or (b) making `rename_variables` conditional based on whether the rename helps conformance (requires identifying the 17 fixtures that need it).
- Approach (a) was attempted but reverted due to complexity — `codegen_scope` uses declaration names in 15+ places (pre-declarations, cache stores, cache loads, body emission).
- Approach (b) is more promising: investigate what makes the 17 rename-dependent fixtures special, then add a condition to `can_rename_scope_decl`.

---

## Step 6: Optional Chaining — Remaining (9 fixtures, MEDIUM)

**Impact:** Up to +9 conformance
**Files:** `src/reactive_scopes/propagate_dependencies.rs`, `src/reactive_scopes/codegen.rs`
**Risk:** LOW

6 of 15 optional chaining fixtures fixed. The remaining 9 involve optional member access in **scope dependency paths** (e.g., `a?.b` used as a reactive dependency in `$[N] !== a?.b`). The `dependency_display_name` function already handles `?.` for dependency paths, but the mismatch is in how the optional flag propagates through the reactive scope dependency collection pipeline, not codegen emission.

---

## Step 7: Silent Bail-outs — Other Categories (23 fixtures, VARIES)

**Impact:** Up to +23 conformance
**Files:** Various

After default params (Step 1), 23 silent bail-outs remain:
- 5x Flow syntax (`component` keyword, Flow type casts) — needs Flow-specific handling in HIR builder or function discovery
- 4x gating patterns — `@enableGating` directive handling
- 5x ref-related — various ref patterns that produce 0 scopes
- 9x other — `hoist-destruct.js`, `unused-object-element-with-rest.js`, `reassign-variable-in-usememo.js`, etc.

---

## BLOCKED — Do Not Attempt

### Under-memoization (Gap 11, ~404 fixtures) — FOUNDATIONAL BLOCKER

Our BFS mutation propagation produces narrower ranges than upstream's full abstract interpreter. We compensate with `effective_range = max(mutable_range.end, last_use + 1)`.

**What would be needed:** Port upstream's ~2000-line abstract interpreter from `src/Inference/InferMutationAliasingEffects.ts`. This also unblocks Gap 5a (+58 fixtures) and may reduce Gap 7.

**Slot deficit distribution:** -1 (136), -2 (118), -3 to -5 (96), -6 to -23 (54)
**Files:** `src/reactive_scopes/infer_reactive_scope_variables.rs`, `src/inference/infer_mutation_aliasing_effects.rs`
**4 prior attempts, all reverted (96%→36% render regression)**

### Over-memoization (Gap 7, ~175 fixtures)

~175 fixtures produce MORE cache slots than upstream (+1 to +42). May self-resolve as side effect of Gap 11.

**Files:** `src/reactive_scopes/merge_scopes.rs`, `src/reactive_scopes/prune_scopes.rs`
**Upstream:** `MergeReactiveScopesThatInvalidateTogether.ts`, `PruneNonEscapingScopes.ts`

### Memoization Preservation Validation (Gap 5a, 58 fixtures)

58 fixtures bail with false "Existing memoization could not be preserved" error. Attempted relaxation caused -28 conformance regression (reverted `bbbbc1d` → `4a082dc`). HARD DEPENDENCY on Gap 11.

### Canvas-Sidebar Render Divergence (Gap 15)

Investigated: scope inference issue (64 vs 70 slots, sentinel vs dependency checks). NOT a codegen bug. BLOCKED on Gap 11.

### Ternary Reconstruction (Gap 6)

P4 cosmetic only. `Terminal::Ternary` emitted as `if/else` instead of `?:`. The `result: Option<Place>` field is ignored. No functional impact.

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
| Reassignment validation | `src/validation/validate_locals_not_reassigned_after_render.rs` |
| Shared function context | `src/validation/function_context.rs` |
| Memoization validation | `src/validation/validate_preserved_manual_memoization.rs` |
| Conformance runner | `tests/conformance_tests.rs` |
| Known failures | `tests/conformance/known-failures.txt` |

All paths relative to `crates/oxc_react_compiler/`.
