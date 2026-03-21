# oxc-react-compiler Backlog

> Last updated: 2026-03-21 (post Phase 108)
> Conformance: **451/1717 (26.3%)**. Render: **96% (24/25)**. E2E: **95-100%**. Tests: all pass, 0 panics.

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

Disabling `rename_variables` entirely causes 451→434 (-17 regressions). These 17 fixtures NEED the temp rename + post-scope assignment pattern to match upstream. The remaining 28 "const t0 vs const name" divergences need selective skip.

### OXC stores default params in `FormalParameter.initializer`
NOT in `BindingPattern::AssignmentPattern`. The `AssignmentPattern` variant in `BindingPattern` is only for destructure defaults in variable declarations.

### Block iteration order ≠ source order for loops
The HIR blocks are stored in creation order, but for-loop constructs create blocks out of source order (the after-loop block can appear before loop body blocks). The frozen mutation validator now uses `frozen_at` instruction ID tracking to enforce source ordering. Any new validator that walks blocks linearly must account for this.

### Cross-scope `IdentifierId` mismatch
Nested function bodies have their own `IdentifierId` numbering. A variable captured from an outer scope (e.g., `onClick` defined in the component, referenced inside `useMemo` callback) has DIFFERENT `IdentifierId`s at each scope level. Name-based resolution is needed for cross-scope tracking.

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
| Both compile, slots DIFFER | ~642 | Scope/memoization divergence (biggest) |
| Both compile, slots MATCH | ~249 | Output format only |
| We bail, they compile | ~138 | False bail-outs |
| We compile, they don't | ~149 | Over-compile (usually fine) |
| Both no memo, format diff | ~87 | Format-only |
| Silent bail-outs | 23 | 0 scopes, no error |

### Bail-out error breakdown (138 fixtures)
```
58x  Existing memoization could not be preserved  ← BLOCKED on scope inference
23x  Frozen mutation                               ← was 29, -6 from instruction ordering fix; 19 false bail-outs remain
23x  (silent, no error)                            ← HIR lowering gaps
 7x  Cannot reassign outside component             ← cross-scope IdentifierId blocker
 6x  Ref access in render                          ← was 14; 8 false bail-outs remain
10x  Reassigned after render (x/y/count/myVar)
 3x  Hooks referenced as values
 3x  Cannot call setState during render
 2x  setState in useEffect
 3x  Other
```

---

## Step 1: Frozen Mutation — Remaining (19 false bail-outs, HARD)

**Impact:** Up to +19 conformance
**Files:** `src/validation/validate_no_mutation_after_freeze.rs`, `src/inference/infer_mutation_aliasing_effects.rs`

**Instrumented triage (23 total false bail-outs → 19 after instruction ordering fix):**

| Check | Count | Root Cause | Fix |
|-------|-------|-----------|-----|
| Check 1 (MutateFrozen from aliasing) | 13 | Aliasing pass over-propagates freeze — IIFE captures, method call results inherit frozen from receiver | Needs `infer_mutation_aliasing_effects.rs` changes. Touches BLOCKED infrastructure. |
| Check 2 (MethodCall on frozen) | 1 | `repro-missing-dependency-if-within-while.js` — while-loop ordering issue not fixed by instruction ID check | Needs deeper loop analysis |
| Check 4 (Nested FE mutation) | 3 | FEs that mutate outer variables flagged even when outer var freshly created | Need to track local allocation |
| Unknown path | 2 | `hook-ref-callback.js`, `useMemo-multiple-returns.js` | Need investigation |

**Blocker:** 13/19 come from Check 1 which is in the aliasing pass. Fixing method call result freeze propagation in `infer_mutation_aliasing_effects.rs` would address the `repro-mutate-result-of-method-call-*` and `capturing-func-alias-*-iife` patterns but risks render regression (same infrastructure as the BLOCKED Gap 11).

---

## Step 2: Named Variable Preservation (28 fixtures, HARD)

**Impact:** Up to +28 conformance
**Files:** `src/reactive_scopes/codegen.rs` (`build_inline_map`, `rename_variables`)
**Risk:** HIGH — `rename_variables` is load-bearing for 17 fixtures

Name promotion map (`build_name_promotion_map`) fixes 6 non-inlinable temp patterns. Remaining 28 are from `rename_variables` scope output renames. Two approaches investigated:
- (a) Clone+mutate scope in `codegen_scope_with_promotions` — reverted, too complex (15+ name resolution points)
- (b) Make `can_rename_scope_decl` conditional — needs identifying the 17 rename-dependent fixtures

---

## Step 3: Remaining Validator False Bail-outs (~15 fixtures, MEDIUM)

| Category | Count | Status |
|----------|-------|--------|
| Cannot reassign outside component | 7 | Cross-scope `IdentifierId` blocker; transitive safe callback works for direct chains only |
| Ref access in render | 8 | Each remaining is a distinct pattern (Flow type casts, `useEffectEvent`, multi-indirection aliases) |
| Hooks referenced as values | 2 | 1 fixed (property access), 1 needs `@enableNameAnonymousFunctions` support |
| Reassigned after render | 10 | Some may benefit from per-body `directly_called` |
| setState in render | 3 | Matched upstream (both no-memo), not actual false bail-outs |
| setState in useEffect | 2 | Need investigation |

---

## Step 4: Optional Chaining — Remaining (7 fixtures, MEDIUM)

**Impact:** Up to +7 conformance (was 9, -2 from optional flag propagation fix)
**Files:** `src/hir/build.rs`, `src/reactive_scopes/propagate_dependencies.rs`
**Risk:** LOW

**What was done:** Fixed HIR builder to use `member.optional` from OXC AST (was hardcoded `false`). Fixed dependency propagation to use PropertyLoad's `optional` field. Fixes 2 fixtures.

**Remaining 7:** 2 still show `comments.edges` vs `comments?.edges` — the OXC AST nesting for deep optional chains may not set `optional: true` on inner member expressions inside `ChainExpression`. The remaining 3 are try-catch + optional (different issue — we don't lower try-catch), and 2 are nested/computed optional patterns.

---

## Step 5: Silent Bail-outs — Other Categories (23 fixtures, VARIES)

- 5x Flow syntax (`component` keyword, Flow type casts)
- 4x gating patterns — `@enableGating` directive
- 5x ref-related — various ref patterns producing 0 scopes
- 9x other

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
