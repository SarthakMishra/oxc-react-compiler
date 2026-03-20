# oxc-react-compiler Backlog

> Last updated: 2026-03-20 (post Phase 106)
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
| Both compile, slots DIFFER | ~628 | Scope/memoization divergence (biggest) |
| Both compile, slots MATCH | ~244 | Output format only |
| We bail, they compile | ~158 | False bail-outs |
| We compile, they don't | ~147 | Over-compile (usually fine) |
| Both no memo, format diff | ~89 | Format-only |
| Silent bail-outs | 28 | 0 scopes, no error |

### Bail-out error breakdown (158 fixtures)
```
58x  Existing memoization could not be preserved  ← BLOCKED on scope inference
29x  Frozen mutation                               ← per-fixture investigation needed
28x  (silent, no error)                            ← HIR lowering gaps
14x  Ref access in render                          ← needs per-body directly_called
 8x  Cannot reassign outside component             ← validator audit needed
 6x  Reassigned after render (x)                   ← partially addressed
 3x  Hooks referenced as values                    ← validator audit needed
 3x  Cannot call setState during render            ← validator audit needed
 2x  setState in useEffect                         ← validator audit needed
 2x  Reassigned after render (count)               ← partially addressed
 3x  Other                                         ← misc
```

---

## Recently Completed (this session: 437→451)

- [x] **Scope declaration rename fix** (`prune_scopes.rs`): `is_last_assignment_in_scope` blocks rename when other instructions follow → +8 conformance
- [x] **Reassignment validator rewrite** (`function_context.rs`): ID-based alias tracking replaces broken name-based `render_only_fns` → 16 fewer bail-outs, 0 regressions
- [x] **LoadLocal read count** (`prune_scopes.rs`): LoadLocal/LoadContext now counted as read in rename eligibility → correctness fix
- [x] **Optional chaining** (`types.rs`, `build.rs`, `codegen.rs`): `optional: bool` on HIR types + dual `MethodCall` flags + codegen `?.` emission → +6 conformance

---

## Step 1: Silent Bail-outs — Default Parameters (5 fixtures, MEDIUM)

**Impact:** +5 conformance (or +5 bail-outs → compile)
**Files:** `src/hir/build.rs` (`lower_function_params` or `lower_variable_declarator`)
**Risk:** LOW — purely additive

5 fixtures silently bail because we don't lower default parameter values. Upstream converts `function f(x = val)` to `function f(x) { x = x === undefined ? val : x; }`.

**Fixtures:** `default-param-array-with-unary.js`, `default-param-calls-global-function.js`, `destructure-default-array-with-unary.js`, `destructuring-array-default.js`, `destructuring-assignment-array-default.js`

**What's needed:** In `lower_function_params` or the destructuring lowering, emit a conditional ternary `param === undefined ? default_expr : param` when parameters have default values. The OXC AST's `BindingPattern` has default value info.

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

## Step 3: Ref-Access Validator Fix (14 fixtures, HARD)

**Impact:** Up to +14 conformance (12 fixed in attempt, but 3 `error.*` regressions)
**Files:** `src/validation/validate_no_ref_access_in_render.rs`, `src/validation/function_context.rs`
**Risk:** MEDIUM — 3 known regressions to solve

**Attempted:** Using `collect_directly_called_fe_ids` from `function_context.rs` to skip FEs that escape render. Fixed 12 of 14 but regressed 3 `error.*` fixtures (net -3). Reverted.

**The 3 regressions (indirect render-time calls):**
1. `error.invalid-ref-in-callback-invoked-during-render.js` — FE called inside `.map()` callback
2. `error.invalid-aliased-ref-in-callback-invoked-during-render-.js` — aliased callback
3. `error.capture-ref-for-mutation.tsx` — chained call `handleKey('left')()`

**Fix needed:** Compute `directly_called` per-function-body, not just top-level HIR. Run `collect_directly_called_fe_ids` on each nested HIR when recursing. This way, nested FEs called within a directly-called FE's body are also treated as render-time.

**Important:** Keep the ref validator's narrow `collect_non_render_callback_ids` — do NOT use `collect_post_render_fn_ids` for the ref validator because `useReducer`/`useState` initializers run during render (marking all hook args as post-render causes 3 additional ref-access regressions).

**Remaining false positives after 3-regression fix (2):**
- `ref-current-aliased-no-added-to-dep.js`
- `valid-setState-in-useEffect-via-useEffectEvent-with-ref.js`

---

## Step 4: Other False Bail-outs Audit (28 fixtures, MEDIUM)

**Impact:** Up to +28 conformance
**Files:** Various validation passes
**Risk:** LOW per category

**Breakdown:**
- 8x "Cannot reassign variables declared outside of the component" → `validate_no_global_reassignment.rs`
- 3x "Hooks may not be referenced as normal values" → `validate_hooks_usage.rs`
- 3x "Cannot call setState during render" → `validate_no_set_state_in_render.rs`
- 2x "setState is called directly inside useEffect" → `validate_no_set_state_in_effects.rs`
- 1x "useMemo called conditionally" → `validate_hooks_usage.rs`
- 1x "Cannot freeze mutable function" → `validate_no_mutation_after_freeze.rs`
- 6x reassigned after render (x/y/count/myVar) → `validate_locals_not_reassigned_after_render.rs` — some may need the per-body `directly_called` fix from Step 3
- 4x other

**Approach:** For each category, read the upstream validation source and compare against our implementation. Most are small exemptions or edge cases.

---

## Step 5: Named Variable Preservation (34 fixtures, HARD)

**Impact:** Up to +34 conformance
**Files:** `src/reactive_scopes/codegen.rs` (`build_inline_map`)
**Risk:** MEDIUM — changes to codegen affect all output

**Root cause:** The 34 remaining `const t0 vs const <name>` divergences come from codegen's `build_inline_map`, not `rename_variables`. When a scope declaration's identifier flows through `StoreLocal → LoadLocal → CallExpression arg`, codegen inlines through a temp rather than preserving the original name. Only 4/34 are post-scope rename patterns; 30 are inline map temps.

**What's needed:**
- Study how `build_inline_map` decides to inline vs emit through named variables
- Teach it to NOT inline through a temp when the source is a named scope declaration
- Upstream's `CodegenReactiveFunction.ts` preserves original names by tracking which intermediates correspond to user-declared variables

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
| Reassignment validation | `src/validation/validate_locals_not_reassigned_after_render.rs` |
| Shared function context | `src/validation/function_context.rs` |
| Memoization validation | `src/validation/validate_preserved_manual_memoization.rs` |
| Conformance runner | `tests/conformance_tests.rs` |
| Known failures | `tests/conformance/known-failures.txt` |

All paths relative to `crates/oxc_react_compiler/`.
