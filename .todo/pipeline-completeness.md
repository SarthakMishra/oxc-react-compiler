# Pipeline Completeness Gaps

> Tracking stub/incomplete pipeline passes and entirely missing upstream passes.
> These are the most impactful gaps for correctness and upstream behavioral parity.

Last updated: 2026-03-12

---

## Part A: Stub / Minimal Passes (existing files, incomplete logic)

### Gap 1: `optimize_for_ssr.rs` -- minimal SSR stripping

**Upstream:** `src/Optimization/OptimizeForSSR.ts`
**Current state:** 20 lines. Only strips scope annotations from identifiers. Upstream does comprehensive work: strips `useMemoCache` calls, removes cache slot assignments, simplifies conditional cache checks, and may rewrite hook guard patterns.
**What's needed:**
- Strip `useMemoCache` import/call from the output
- Remove cache-related instructions (StoreLocal to cache slots, LoadLocal from cache slots)
- Remove conditional branches that check cache validity (`if (cache[n] !== Symbol.for(...)`)
- Preserve the computation code but remove all memoization scaffolding
- Review interaction with `OutputMode::SSR` in codegen -- some of this may belong there instead
**Depends on:** None

---

### Gap 2: `validate_no_ref_access_in_render.rs` -- naming heuristic only

**Upstream:** `src/Validation/ValidateNoRefAccessInRender.ts`
**Current state:** 38 lines. Uses `endsWith("Ref")` / `endsWith("ref")` naming convention to detect refs. High false negative rate -- misses refs with non-standard names. High false positive rate -- catches variables that happen to end in "ref" but are not React refs.
**What's needed:**
- Track `useRef()` call return values through the HIR (similar to useState tracking in Gap 9 of config-parity.md)
- Mark identifiers produced by `useRef()` as ref-typed in the type inference pass
- Use type information instead of naming heuristic for the primary check
- Keep naming heuristic as a fallback when type info is unavailable (behind `enable_treat_ref_like_identifiers_as_refs`)
**Depends on:** Type inference improvements (infer_types.rs)

---

### Gap 3: `validate_no_set_state_in_render.rs` -- naming heuristic only

**Upstream:** `src/Validation/ValidateNoSetStateInRender.ts`
**Current state:** 37 lines. Uses `starts_with("set") + uppercase` pattern. Same false positive/negative issues as ref detection.
**What's needed:**
- Track `useState()` return values through destructuring
- Mark the setter (second element) as a state setter in type info
- Use type information for the primary check
- Keep naming heuristic as fallback
**Depends on:** Type inference improvements, overlaps with config-parity.md Gap 9

---

### Gap 4: `validate_static_components.rs` -- PascalCase check only

**Upstream:** `src/Validation/ValidateStaticComponents.ts`
**Current state:** 30 lines. Only checks if a `FunctionExpression` has a PascalCase name. Doesn't verify whether the component is wrapped in `React.memo()`, doesn't analyze if it captures reactive state, and doesn't check if it's in a render-scoped position.
**What's needed:**
- Detect `React.memo()` wrapping and exclude those from warnings
- Analyze whether the inline component captures reactive variables (if it only uses props/context, it may be intentional)
- Check scope -- components defined at module level are fine; only flag those inside other components
- Upstream has more nuanced heuristics for when to warn vs. when to auto-fix
**Depends on:** None

---

### Gap 5: `outline_jsx.rs` -- effectively a no-op

**Upstream:** `src/Optimization/OutlineJsx.ts`
**Current state:** 59 lines. Comment says "HIR already flattens JSX into temporaries." The pass scans JSX children but makes no modifications.
**What's needed:**
- **First: verify the claim.** Compare our HIR lowering output against upstream's to confirm that our `build.rs` already creates the same temporary structure that upstream's `OutlineJsx` would produce.
- If the claim holds, document the intentional divergence with a `// DIVERGENCE:` comment and close this gap
- If the claim does NOT hold, implement the outlining logic: extract nested JSX children into separate instructions with their own temporaries
**Depends on:** None

---

### Gap 6: `outline_functions.rs` -- identifies candidates but no hoisting

**Upstream:** `src/Optimization/OutlineFunctions.ts`
**Current state:** 35 lines. Correctly identifies functions with empty context (no captures) as hoistable candidates, but does not actually hoist them. The comment says "codegen support needed."
**What's needed:**
- In the pass: mark hoistable functions with a flag on the instruction or identifier
- In codegen: emit marked functions at module scope instead of inside the component body
- Generate a module-level binding and replace the inline FunctionExpression with a reference to it
- Handle edge cases: functions that reference other hoistable functions, ordering
**Depends on:** Codegen changes

---

## Part B: Missing Upstream Passes (no corresponding file exists)

### Gap 7: `CollectHoistablePropertyLoads` -- CRITICAL for dependency precision

**Upstream:** `src/HIR/CollectHoistablePropertyLoads.ts`
**Current state:** No corresponding file. This is one of the most impactful missing passes.
**What's needed:**
- Determines which property loads can be safely hoisted based on non-null guarantees from control flow
- If `a.b` is accessed unconditionally in a block, then `a` is guaranteed non-null and `a.b` can be hoisted to the scope entry
- Uses dominator tree analysis to find unconditional property accesses
- Output feeds into `PropagateScopeDependencies` for precise dependency tracking
- Without this pass, optional chain patterns like `a?.b.c` produce overly conservative dependencies
**Depends on:** Gap 11 (ComputeUnconditionalBlocks)

---

### Gap 8: `CollectOptionalChainDependencies` -- CRITICAL for optional chain correctness

**Upstream:** `src/HIR/CollectOptionalChainDependencies.ts`
**Current state:** No corresponding file.
**What's needed:**
- Maps optional chain expressions (`a?.b?.c`) to their dependency semantics
- Determines the "base" of an optional chain and which property loads are conditional
- Prevents the dependency system from tracking `a?.b.c` as requiring `a.b.c` (which would throw if `a.b` is null)
- Produces a mapping from optional chain terminals to their safe dependency representations
- Integrates with `PropagateScopeDependencies`
**Depends on:** Gap 7 (CollectHoistablePropertyLoads)

---

### Gap 9: `DeriveMinimalDependenciesHIR` -- dependency tree minimization

**Upstream:** `src/HIR/DeriveMinimalDependenciesHIR.ts`
**Current state:** No corresponding file.
**What's needed:**
- Tree-based dependency minimization algorithm
- If a scope depends on both `props.a` and `props.a.b`, only track `props.a` (the shorter path subsumes the longer)
- Conversely, if only `props.a.b` is needed, don't track `props.a` (avoid unnecessary invalidation)
- Operates on a trie/tree structure of property paths
- Output replaces the raw dependency set with a minimal equivalent set
**Depends on:** None (utility module, consumed by PropagateScopeDependencies)

---

### Gap 10: `ScopeDependencyUtils` -- shared dependency utilities

**Upstream:** `src/ReactiveScopes/ScopeDependencyUtils.ts`
**Current state:** No corresponding file. Logic may be partially inlined in `propagate_dependencies.rs`.
**What's needed:**
- Audit `propagate_dependencies.rs` to identify which utility functions from upstream are missing
- Factor out shared dependency manipulation utilities (path comparison, dependency merging, property path normalization)
- Used by multiple passes: PropagateScopeDependencies, DeriveMinimalDependencies, CollectOptionalChainDependencies
**Depends on:** None

---

### Gap 11: `ComputeUnconditionalBlocks` -- unconditional execution analysis

**Upstream:** `src/HIR/ComputeUnconditionalBlocks.ts`
**Current state:** No corresponding file.
**What's needed:**
- Walk the CFG and identify blocks that execute unconditionally (not guarded by any conditional branch)
- Uses post-dominator analysis or equivalent
- Output is consumed by `CollectHoistablePropertyLoads` to determine which property accesses are guaranteed
**Depends on:** None (pure CFG analysis)

---

### Gap 12: `assertWellFormedBreakTargets` -- break target validation

**Upstream:** `src/Validation/AssertWellFormedBreakTargets.ts`
**Current state:** No corresponding file.
**What's needed:**
- Validate that all `break` and `continue` statements in the reactive function target valid labels
- Check that break targets respect reactive scope boundaries (a break cannot exit a reactive scope)
- Emit diagnostic if a break target is malformed
- Medium priority -- only affects programs with labeled break/continue inside memoized blocks
**Depends on:** None

---

### Gap 13: `PruneTemporaryLValues` -- temporary cleanup

**Upstream:** `src/HIR/PruneTemporaryLValues.ts`
**Current state:** No corresponding file.
**What's needed:**
- Post-SSA optimization pass that removes unnecessary temporary assignments
- If a temporary `t0 = expr` is used exactly once and the use is the immediately next instruction, inline the value
- Reduces instruction count and simplifies downstream analysis
- Low-medium priority -- temporaries don't affect correctness but do affect codegen readability
**Depends on:** SSA (enter_ssa.rs)
