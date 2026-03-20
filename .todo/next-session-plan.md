# Next Session Plan

> Written: 2026-03-20 (post Phase 106 + validation fix). Self-contained.

## Current Metrics

| Metric | Value |
|--------|-------|
| Conformance | 445/1717 (25.9%) |
| Render equivalence | 96% (24/25) |
| Bail-outs | 158 (down from 174) |
| E2E coverage | 95-100% across 4 Vite projects |

## What Was Done This Session

1. **Scope declaration rename fix** (`prune_scopes.rs`): +8 conformance via `is_last_assignment_in_scope`
2. **Reassignment validator fix** (`validate_locals_not_reassigned_after_render.rs`): New `function_context.rs` with ID-based alias tracking. 16 fewer bail-outs, 0 regressions.

## What's Remaining (ordered by feasibility)

### Ready Now: Ref-Access Validator (14 fixtures)

The ref-access validator (`validate_no_ref_access_in_render.rs`) still uses the old narrow `collect_non_render_callback_ids`. It needs a DIFFERENT approach than the reassignment validator because:
- `useState`/`useReducer` initializers run DURING render (ref access inside them IS invalid)
- The reassignment validator marks ALL hook args as post-render, which would cause 3 regressions for ref-access

**Approach:** Use `collect_directly_called_fe_ids` from `function_context.rs` to detect render-only FEs, then skip ref-access checking for those. Keep the existing narrow `collect_non_render_callback_ids` for hook detection. The fix combines both: skip FEs that are either (a) known non-render callbacks (useEffect etc.) OR (b) directly-called render-only FEs.

### Ready Now: Named Variable Preservation (~56 fixtures)

Many fixtures have temp names from codegen inlining of function expressions and intermediate values. Requires study of upstream's `CodegenReactiveFunction.ts` naming logic.

### Ready Now: Optional Chaining (15 fixtures)

Add `optional: bool` to HIR types and propagate through codegen.

### BLOCKED: Canvas-Sidebar Render Divergence

Investigated and confirmed as a scope inference issue (64 slots vs Babel's 70). OXC uses `Symbol.for("react.memo_cache_sentinel")` sentinel checks while Babel uses dependency-based checks. This is Gap 11 (under-memoization) — requires porting upstream's full abstract interpreter. DO NOT attempt.

### BLOCKED: Scope Inference (Gaps 7, 11)

Under-memoization (404 fixtures) and over-memoization (175 fixtures). Requires porting upstream's ~2000-line abstract interpreter. 4 prior attempts all reverted (96%→36% render regression).

## Build & Test

```bash
cargo test                          # All Rust tests
cargo test --test conformance_tests -- --nocapture  # Conformance
cd napi/react-compiler && npx @napi-rs/cli build --release  # NAPI
cd benchmarks && npm run render:compare   # Render
cd benchmarks && npm run e2e:quick        # E2E
```

## Key Files

| Purpose | Path |
|---------|------|
| Code generation | `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` |
| Scope pruning | `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs` |
| Ref access validation | `crates/oxc_react_compiler/src/validation/validate_no_ref_access_in_render.rs` |
| Reassignment validation | `crates/oxc_react_compiler/src/validation/validate_locals_not_reassigned_after_render.rs` |
| Shared function context | `crates/oxc_react_compiler/src/validation/function_context.rs` |
| Conformance runner | `crates/oxc_react_compiler/tests/conformance_tests.rs` |

## CRITICAL: Do NOT Attempt

- Scope inference changes, `mutable_range` narrowing, validation relaxation (Gap 5a)
- Canvas-sidebar (scope inference issue, not codegen)
