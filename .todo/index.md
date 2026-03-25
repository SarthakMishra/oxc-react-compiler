# oxc-react-compiler Backlog

> Last updated: 2026-03-25 (post Stage 4e-B partial)
> Conformance: **443/1717 (25.8%)**. Render: **92% (23/25)**. E2E: **95-100%**. Tests: all pass, 0 panics, 0 unexpected divergences.
> Stage 4e-B (in progress): +1 fixture (442->443). Fixed hooks-in-for-loop detection via Terminal::Branch handling in validate_hooks_usage.rs.
> Known-failures: 1274. Error.* fixtures remaining in KF: 38 (36 top-level + 2 fbt/).

---

## Road to 600+ Conformance (411 → 600+, need +189)

### Failure Category Summary (post-investigation, revised)

| Category | Count | Description |
|----------|-------|-------------|
| Both compile, slots DIFFER | ~699 (53%) | Scope inference accuracy — different cache slot counts. **Largest pool, requires scope inference fixes.** |
| Both compile, slots MATCH | ~237 (18%) | Same slots, codegen structure diffs. **B2 pattern (temps vs original names) dominates: 40 fixtures.** |
| We compile, they don't | ~194 (15%) | Missing validations. **75 are UPSTREAM ERROR fixtures (7 more fixed in Stage 4e-A). 32 need preserveExistingMemoization. 5 remain for Todo error detection (22 done). 2 frozen-mutation remain (9 fixed in Stage 4d).** |
| We bail, they compile | ~73 (6%) | False-positive bail-outs (down from 108→89→~69 after Stage 2c, +4 IIFE false positives from Stage 4d name-based freeze tracking) |
| Both no memo (format diff) | 76 (6%) | Neither side memoizes. **Requires DCE + constant propagation passes — NOT quick wins.** |

### Key Investigation Findings (2026-03-25)

1. **"Both no memo" (76 fixtures) is NOT low-hanging fruit.** These require dead-code elimination and constant propagation compiler passes. Neither pass exists yet. This is significant compiler work, not cosmetic format fixes as originally assumed.

2. **"We compile, they don't" (225 fixtures) breakdown:**
   - **75 are UPSTREAM ERROR fixtures** — upstream intentionally bails with an error message as the expected output. We compile instead. These are correct behavior IF we add the matching error detection. However, many of these error messages come from validations we already partially have.
   - **32 need `validatePreserveExistingMemoizationGuarantees`** — our implementation exists but has gaps vs upstream
   - **27 need various `Todo` error detection** — upstream emits `Todo` errors for unimplemented features; we silently compile
   - **11 need frozen-mutation detection fixes** — our validation fires incorrectly or misses cases
   - Remaining: mixed validation gaps (ref-access, reassignment, hooks, etc.)

3. **"Slots MATCH" (237 fixtures) is dominated by B2 pattern** — 40 fixtures where we use temp variables but upstream preserves original variable names in scope outputs. This is the single largest tractable sub-pattern in slots-MATCH.

4. **Stage 2c (`_exp` directive handling) is COMPLETE** — moved 20 fixtures from "we bail, they compile" to "both compile" categories. Net conformance +0 because the newly-compiling fixtures land in slots-DIFFER/MATCH pools (their output doesn't match yet). But this unblocks those 20 fixtures for future scope/codegen fixes.

### Revised Path to 600+

The path is clearer but requires significant compiler infrastructure work:

| Work Item | Pool Size | Potential Gain | Difficulty |
|-----------|-----------|---------------|------------|
| Scope inference fixes (slots-DIFFER) | ~699 | +50-100 | HIGH — cascading regression risk |
| DCE + constant propagation (both-no-memo) | 76 | +30-50 | HIGH — new compiler passes needed |
| `validatePreserveExistingMemoizationGuarantees` gaps | 32 | +15-25 | MEDIUM — extend existing validation |
| Variable name preservation in codegen (B2) | 40 | +20-30 | MEDIUM — scope output naming changes |
| Declaration placement / instruction ordering (A1) | 55+ | +15-30 | HIGH — load-bearing code |
| Remaining bail-out fixes (2d-2g) | ~49 | +15-25 | MEDIUM — per-validation fixes |
| Todo error detection (remaining) | 12 | +5-8 | LOW — need hoisting, optional terminals, default params |
| Frozen-mutation validation fixes | 2 remain | +9 done (Stage 4d) | MEDIUM | 2 remaining need effect callback + JSX capture analysis |

**Conservative estimate:** +134-259 from 442 base = 576-701. Reaching 600 is feasible but requires scope inference work (the largest and highest-risk category).

---

### Stage 1: Codegen Structure — "Slots MATCH" Fixes (target: +40-60 fixtures)

**Pool:** 239 fixtures where slot count matches upstream but codegen differs.
**Root causes:** Variable naming (`t0` vs original names), instruction ordering within scopes, scope output placement, dependency list ordering.
**Risk:** LOW — these are the closest to passing, same semantic structure.

#### Stage 1a: Investigate "Slots MATCH" Patterns -- COMPLETE

Completed 2026-03-25. Full results in [slots-match-investigation.md](slots-match-investigation.md).
Found 4 sub-patterns: variable naming (126, 52.7%), instruction ordering (55, 23.0%), structural (58), other (44).

#### Stage 1b: Temp Variable Renumbering -- COMPLETE (+2 net, not +25-40)

Completed 2026-03-25. Implemented `renumber_temps_in_output` in `codegen.rs` (two-pass atomic rename, t0/t1/t2 sequential).
Also fixed `is_temp_place` pattern matching, Unicode safety in `replace_identifier_in_output`, and `$` word boundary.
**Gained only +2 fixtures** (403→405): `gating/multi-arrow-expr-export-gating-test.js`, `gating/multi-arrow-expr-gating-test.js`.
Estimate of +25-40 was wrong: most "naming" differences also involve instruction ordering or scope output name preservation,
which temp renumbering alone cannot fix. See [slots-match-investigation.md](slots-match-investigation.md) for revised analysis.

#### Stage 1c: Minor Codegen Fixes -- COMPLETE (+5 net, 405→410)

Completed 2026-03-25. C2 (return undefined): +5 fixtures. C5 (catch clause): +0 net (improves output but all catch fixtures also blocked by A1 instruction ordering). B4 (edge case naming): skipped — only 1 fixture and high complexity.
**Fixtures gained:** capturing-func-mutate-nested.js, capturing-function-decl.js, hoisting-recursive-call.ts, mutate-captured-arg-separately.js, reassign-object-in-context.js.

#### Stage 1d: Declaration Placement / Instruction Ordering (est: +15-30, HIGH risk)

- [ ] Redesign `collect_all_scope_declarations` to emit declarations at narrowest possible scope instead of function level
- [ ] Fix hook call ordering (hook calls before temp declarations, not after)
- [ ] This is the dominant remaining "slots MATCH" blocker — 55+ fixtures depend on instruction ordering, and many of the 126 "naming" fixtures also need this
- **Risk:** HIGH — `collect_all_scope_declarations` is load-bearing (removing it collapses render 96%→24%). Requires careful incremental approach.
- **Prerequisite:** Must understand exactly which declarations can be moved safely vs which must stay at function level

---

### Stage 2: False-Positive Bail-outs — "We Bail, They Compile" (target: +50-70 fixtures)

**Pool:** Originally 108 fixtures, now **89 remaining** after Stage 2b partial fixes.
**Risk:** MEDIUM — each bail-out removal must not introduce wrong output.

#### Stage 2a: Investigate Bail-out Categories -- COMPLETE

Completed 2026-03-25. Full results in [bail-out-investigation.md](bail-out-investigation.md).
Categorized all 108 bail-outs by error type. Found 4 overly aggressive file-level bail-outs (lint mode, incompatible imports, eslint suppression, runtime import check).

#### Stage 2b: Remove Overly Aggressive File-Level Bail-outs -- PARTIALLY COMPLETE (+1 net, 410→411)

Completed 2026-03-25. Removed 4 file-level bail-outs in `program.rs`:
- Removed `OutputMode::Lint` early return (+2 net passing, 42 fixtures now compile)
- Removed `has_known_incompatible_import` file-level bail (+0 net)
- Refined `has_compiler_runtime_import` to only bail on `c`/`useMemoCache` imports (+0 net)
- Removed `has_eslint_suppression_for_rules` file-level bail (+1 net passing)
Net result: bail-outs reduced 108→89, conformance +1 (410→411). 2 error.todo fixtures regressed (added to known-failures).
Remaining 89 bail-outs require per-validation fixes (see stages 2c-2f below).

#### Stage 2c: Fix `_exp` Directive Handling -- COMPLETE (+0 net, 20 fixtures moved)

Completed 2026-03-25. Fixed handling of `@validateNoDerivedComputationsInEffects_exp` directive fixtures.
These 20 fixtures now compile instead of bailing, but land in slots-DIFFER/MATCH pools (output doesn't match upstream yet).
Net conformance: +0. But these fixtures are now unblocked for future scope/codegen improvements.

#### Stage 2d: Fix Frozen-Mutation False Positives (11 original + 4 new IIFE = ~15 fixtures)

- [ ] Review `validate_no_mutation_after_freeze` / `InferMutableRanges` for over-reporting mutations on frozen values
- [ ] Compare our validation logic against upstream's to find divergence
- [ ] Implement targeted relaxations without losing true-positive detections
- [ ] **NEW (post Stage 4d):** Fix 4 IIFE-pattern false positives introduced by name-based freeze tracking (`capturing-func-alias-*-iife.js`). These fixtures mutate a captured variable inside an IIFE, but the name-based tracker incorrectly treats this as mutation-after-freeze. Future fix: implement scoped name tracking that distinguishes IIFE-internal mutations from true post-freeze mutations.
- **Risk:** MEDIUM — requires mutable range analysis refinements + scoped name tracking for IIFE patterns
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)

#### Stage 2e: Fix Ref-Access False Positives (8 fixtures)

- [ ] Review `validateNoRefAccessInRender` for over-eager patterns
- [ ] Some patterns (assigning ref-accessing functions to properties, ref type casts) should be allowed
- [ ] Compare against upstream validation
- **Risk:** MEDIUM
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)

#### Stage 2f: Fix Reassignment False Positives (10 fixtures)

- [ ] Review `validateLocalsNotReassignedAfterRender` for false positives
- [ ] Compare against upstream validation
- **Risk:** MEDIUM
- **Details:** [bail-out-investigation.md](bail-out-investigation.md)

#### Stage 2g: Other Bail-out Fixes (remaining ~40 fixtures)

- [ ] Fix remaining false-positive bail-outs: setState-in-render (4), setState-in-effect (7), hooks (3), preserve-memo (4), exhaustive-deps (3), silent (9), other (10)
- [ ] Each fix: compare upstream validation logic, adjust our thresholds
- [ ] Re-categorize after 2c-2f to identify new patterns

#### Stage 2h: Replan — Bail-out Residual (est: 0 fixtures, planning)

- [ ] Categorize remaining "we bail, they compile" after 2c-2g
- [ ] Update plan with new findings or mark as deferred

---

### Stage 3: Scope Inference — Small Slot Diffs (target: +30-50 fixtures)

**Pool:** 246 fixtures with ±1 slot diff, 55 with ±2 slot diff (301 total).
**Root causes:** Scope merging too aggressive/conservative, dependency over/under-counting, mutable range gaps.
**Risk:** HIGH — scope inference changes can cause cascading regressions.

#### Stage 3a: Investigate ±1 Slot Diff Patterns (est: 0 fixtures, prerequisite)

- [ ] Sample 30+ fixtures from the ±1 group
- [ ] For each: compare scope boundaries, identify which scope is extra/missing
- [ ] Categorize: (a) extra scope (over-splitting), (b) missing scope (under-splitting), (c) scope merged wrong, (d) dependency miscounted
- [ ] **Deliverable:** Root cause distribution with fixture counts
- **Critical:** Must identify whether fixes would cause regressions in currently-passing fixtures

#### Stage 3b: Fix Dominant ±1 Pattern (est: +15-25 fixtures)

- [ ] Implement fix for most common ±1 root cause from 3a
- [ ] **Regression check:** Run full conformance, verify no currently-passing fixtures break
- [ ] If regressions: revert, add to 3e investigation

#### Stage 3c: Fix Secondary ±1 Patterns (est: +10-15 fixtures)

- [ ] Fix remaining tractable ±1 patterns from 3a
- [ ] Regression check on each

#### Stage 3d: ±2 Slot Diff Quick Wins (est: +5-10 fixtures)

- [ ] Sample ±2 fixtures, check if any share root cause with ±1 fixes
- [ ] Fix only if low regression risk

#### Stage 3e: Replan — Slot Diff Residual (est: 0 fixtures, planning)

- [ ] Analyze remaining slot diff failures
- [ ] Determine if mutable_range switch (Phase 4d) would help or if deeper scope inference changes needed
- [ ] Update plan

---

### Stage 4: Validation Gaps — "We Compile, They Don't" (target: +50-75 fixtures)

**Pool:** ~225 fixtures where upstream bails with an error but we compile (incorrectly).
**Risk:** LOW — adding validations is safe (bail-out = pass-through = correct).

#### Stage 4a: Categorize Missing Validations -- COMPLETE (investigation)

Completed 2026-03-25 (extended investigation). Breakdown of ~225 "we compile, they don't" fixtures:

| Sub-category | Count | Action Needed |
|-------------|-------|---------------|
| UPSTREAM ERROR fixtures (expected output IS the error) | 29 remaining in KF (was 75, 22 fixed in Stages 4c+4e-A, others already passing) | Must bail (not transform) to pass — error message matching NOT required |
| `validatePreserveExistingMemoizationGuarantees` gaps | 32 | Extend existing preserve-memo validation |
| `Todo` error detection (unimplemented features) | 5 remaining (22 done) | 5 need hoisting, optional terminals, context var detection |
| Frozen-mutation detection gaps | 2 remain (9 fixed) | 9 fixed in Stage 4d; 2 remain (effect callback, JSX capture) |
| Other validation gaps (ref-access, reassignment, hooks) | ~80 | Various per-validation fixes |

#### Stage 4b: Implement `validatePreserveExistingMemoizationGuarantees` Fixes (32 fixtures)

- [ ] Audit our `validate_preserved_manual_memoization.rs` against upstream
- [ ] Identify which guarantee checks are missing (likely: dependency tracking, conditional memoization patterns)
- [ ] Implement missing checks
- [ ] **Risk:** MEDIUM — our implementation exists but has known gaps
- [ ] **Potential gain:** +15-25 fixtures (some may also need other fixes)

##### Partial Investigation Notes (2026-03-25)

An investigation was started but not completed. Key finding: our validation exists at Pass 61 but fails to detect errors because `finish_in_scope` is true -- our scope inference wraps `FinishMemoize` in reactive scopes, which causes the validation to skip checks it should be performing. Additionally, upstream has `validateInferredDep` checks that we skip entirely. These are **prerequisites** for Stage 4b to succeed:

1. Understand why `finish_in_scope` is true (scope inference wrapping `FinishMemoize` instructions)
2. Port the `validateInferredDep` checks from upstream
3. May require scope inference adjustments to avoid wrapping `FinishMemoize` in reactive scopes

**Status:** Investigation incomplete. Approach is understood but implementation requires deeper scope inference analysis. Consider doing Stage 4c first (lower risk, no blockers).

#### Stage 4c: Add Todo Error Detection -- PARTIALLY COMPLETE (+15 net, 411->426)

Completed 2026-03-25. Implemented bail-outs for 15 of 27 Todo-error fixtures:
- Try-without-catch blocks (2 fixtures) — added in `hir/build.rs`
- Computed object keys (4 fixtures) — added in `hir/build.rs`
- Value blocks in try/catch (7 fixtures) — added in `validation/validate_no_unsupported_nodes.rs`
- Throw in try (1 fixture) — added in `validation/validate_no_unsupported_nodes.rs`
- Fbt local variables (1 fixture) — added in `validation/validate_no_unsupported_nodes.rs`

**Key finding:** The 16 fixtures originally identified as targets were already passing. The actual Todo-error fixtures were in the known-failures list (UPSTREAM ERROR set). Of 27 in that set, 15 fixed, 12 remain.

**Remaining 5 Todo-error fixtures** (require more complex handling — 7 of original 12 fixed in Stage 4e-A):
- Hoisting patterns (2) — `error.todo-functiondecl-hoisting.tsx`, `error.todo-valid-functiondecl-hoisting.tsx` — need function-level hoisting infrastructure
- Optional terminal issues (1) — `error.todo-preserve-memo-deps-mixed-optional-nonoptional-property-chain.js` — need optional chaining terminal handling
- Update expression on context vars (1) — `error.todo-handle-update-context-identifiers.js` — BLOCKED: nested HIR builders don't emit LoadContext (see blocker report in Stage 4e-A)
- For-loop context vars (1) — `error.todo-for-loop-with-context-variable-iterator.js` — need for-loop context variable handling
- **Fixed in 4e-A (moved from this list):** hoisted-function-in-unreachable-code, hoist-function-decls, hook-call-spreads-mutable-iterator, default-param-accesses-local, fbt-as-local, bug-invariant-couldnt-find-binding-for-decl, hoisting-simple-function-declaration

#### Stage 4d: Fix Frozen-Mutation False Negatives -- COMPLETE (+9 net, 426->435)

Completed 2026-03-25. Implemented name-based freeze tracking in `validate_no_mutation_after_freeze.rs` to detect mutations on frozen values that were previously missed due to IdentifierId mismatches across scopes.
- **7 of 9 planned fixtures fixed** plus **2 bonus fixtures** (9 total gained)
- **Side effect:** 9 fixtures shifted from slots-MATCH/DIFFER to bail category (name-based tracking introduced false positives on IIFE patterns where a variable is captured and mutated inside an IIFE, but the freeze tracker sees the mutation as post-freeze)
- **2 planned fixtures remain:**
  - `error.assign-ref-in-effect-hint.js` — requires effect callback mutation checking (not just freeze tracking)
  - `error.invalid-jsx-captures-context-variable.js` — complex JSX capture pattern, needs deeper analysis
- **New regression:** 4 IIFE-pattern fixtures (`capturing-func-alias-*-iife.js`) now falsely bail. See Stage 2d note below.

#### Stage 4e: UPSTREAM ERROR Fixture Handling (39 error.* remain in KF post 4e-A, was 43)

**Critical correction (2026-03-25):** The conformance test does NOT require matching exact error messages. It only checks `!compile_result.transformed` (line 781 of conformance_tests.rs). To pass an UPSTREAM ERROR fixture, we just need to bail (not transform). This is much simpler than originally described.

**Revised breakdown of 39 error.* fixtures remaining in known-failures (post Stage 4e-A):**

| Sub-category | Count | What we need to bail |
|-------------|-------|---------------------|
| "Compilation Skipped: preserve-memo" | 11 | `validatePreserveExistingMemoizationGuarantees` must detect and bail — overlaps Stage 4b |
| "Todo: hoisting/optional/context-var/etc" | 7 | Remaining unsupported patterns: hoisting (3), optional member expr (2), context var update (1), missing source locs (1) — need deeper compiler infra |
| "Invariant: ..." (upstream internal errors) | 3 | MethodCall codegen (1), inconsistent destructuring (1), unnamed temporary (1) — 3 of original 6 fixed in 4e-A |
| "Error: This value cannot be modified" | 3 | Frozen-mutation detection — overlaps Stage 4d remaining |
| "Error: Cannot modify locals after render" | 2 | `validateLocalsNotReassignedAfterRender` gaps |
| "Error: Cannot access refs during render" | 4 | `validateNoRefAccessInRender` gaps (ref-like-name patterns + pass-ref + mutate-ref) |
| "Error: setState from useMemo" | 1 | setState-in-render validation gap (already partially fixed in 4cd3b20) |
| "Error: validate-*" | 3 | validate-blocklisted-imports (1), validate-object-entries/values-mutation (2) |
| Compiled output (NOT UPSTREAM ERROR) | 5 | Slots-DIFFER/MATCH issues, not bail-out issues |

**Tractable sub-tasks (no new infrastructure needed):**

- [x] **4e-A: Mixed bail-outs — COMPLETE (+7, 435->442)** — implemented 7 new bail-outs across 3 files: hoisted function decls in unreachable code (3 fixtures: `error.todo-hoist-function-decls.js`, `error.todo-hoisted-function-in-unreachable-code.js`, `error.hoisting-simple-function-declaration.js`), fbt parameter name detection (1: `fbt/error.todo-fbt-as-local.js`), default-param arrow/function expressions (1: `error.default-param-accesses-local.js`), catch clause destructuring (1: `error.bug-invariant-couldnt-find-binding-for-decl.js`), hook spread arguments (1: `error.todo-hook-call-spreads-mutable-iterator.js`). Files: `validate_no_unsupported_nodes.rs`, `build.rs`, `known-failures.txt`. **Note:** `error.todo-handle-update-context-identifiers.js` (Group 6, UpdateExpression on context vars) was NOT fixed — nested HIR builders don't emit `LoadContext` instructions, so context variables can't be detected by walking the nested HIR. See blocker report below.
- [~] **4e-B: Locals-reassigned + ref-access + setState bail-outs (5 fixtures)** — tighten existing validators (`validate_no_ref_access_in_render`, `validate_locals_not_reassigned_after_render`, setState checks, hooks-in-loop) to catch these specific patterns. **Progress:** +1 fixture (hooks-in-for-loop via Terminal::Branch handling in `validate_hooks_usage.rs` `find_conditional_blocks`). Remaining potential gain: +4.
- [ ] **4e-C: Frozen-mutation remaining (3 fixtures)** — overlaps Stage 4d remaining. Need effect callback mutation + JSX capture analysis. Potential gain: +3.
- [ ] **4e-D: Preserve-memo gaps (11 fixtures)** — overlaps Stage 4b. BLOCKED by `finish_in_scope` issue (see Stage 4b notes). Potential gain: +11 but requires scope inference fix.
- [ ] **4e-E: Todo remaining (7 fixtures)** — overlaps Stage 4c remaining. Need hoisting (3), optional member expr (2), context var update (1), missing source locs (1). Potential gain: +7 but requires new infrastructure. Context var update BLOCKED by nested HIR LoadContext gap.

**Stage 4e-A done: +7 fixtures gained.**
**Stage 4e-B progress: +1 fixture gained (443 total).** Fixed hooks-in-for-loop detection: `find_conditional_blocks` in `validate_hooks_usage.rs` now handles `Terminal::Branch` (for-loop continue/break targets), which was previously unmatched, causing the validator to miss hook calls inside for-loops. This is a targeted fix; remaining 4e-B fixtures (locals-reassigned, ref-access, setState patterns) still need investigation.
**Remaining tractable gain (4e-B): +4 fixtures, no new infrastructure.**
**Full potential (all remaining sub-tasks): +26 fixtures.**
**Risk:** LOW for 4e-B. MEDIUM-HIGH for 4e-C/D/E.

#### Blocker Report — Nested HIR LoadContext gap (2026-03-25)

**Affects:** `error.todo-handle-update-context-identifiers.js` (Stage 4e-E, 1 fixture)

**Approach attempted:** Walk the nested function HIR looking for `PostfixUpdate`/`PrefixUpdate` instructions whose operand is a "context variable" (captured from outer scope). Tried collecting local declaration IDs and treating any non-local variable as a context variable.

**Assumption that was wrong:** Expected nested HIR to contain `LoadContext` instructions for captured variables (as upstream does). In reality, our nested `HIRBuilder` emits `LoadLocal` for all variables, whether local or captured from outer scope.

**What was discovered:** The nested HIR builder creates a fresh scope and fresh `IdentifierId` numbering for each nested function. Variables captured from the parent are lowered as `LoadLocal` with a new ID, not `LoadContext`. The distinction between "local" and "context" is only resolved later in the pipeline (during scope inference / codegen), not at HIR construction time. This means a validation pass running on the raw nested HIR cannot distinguish context variables from locals.

**Prerequisites for a successful attempt:**

- Either emit `LoadContext` in nested HIR builders (requires threading parent scope bindings into child builder), OR
- Run this validation after scope inference when context variables are identified, OR
- Pass parent scope binding names to the validation and compare by name (fragile but possible)

**Do NOT attempt again until:** The HIR builder's nested function lowering is enhanced to distinguish context variables, or a post-scope-inference validation hook exists.

---

### Stage 5: "Both No Memo" — DCE + Constant Propagation (target: +30-50 fixtures)

**Pool:** 76 fixtures where neither side memoizes but output differs.
**Risk:** HIGH — requires implementing new compiler passes (DCE, constant propagation).

**Investigation finding (2026-03-25):** These are NOT cosmetic format diffs as originally assumed. The 76 fixtures produce different output because upstream runs dead-code elimination and constant propagation passes that simplify the output. Without these passes, our output includes dead assignments, unreachable branches, and un-folded constants that upstream eliminates.

#### Stage 5a: Dead Code Elimination Pass

- [ ] Implement DCE pass to remove unused variable assignments, unreachable branches
- [ ] Upstream reference: `src/Optimization/DeadCodeElimination.ts` (or equivalent)
- [ ] Must run after scope inference, before codegen
- [ ] **Risk:** HIGH — new pass, requires thorough testing
- [ ] **Prerequisite for:** Many of the 76 "both no memo" fixtures

#### Stage 5b: Constant Propagation / Folding

- [ ] Implement constant propagation to fold known-constant expressions
- [ ] Upstream reference: `src/Optimization/ConstantPropagation.ts` (or equivalent)
- [ ] **Risk:** HIGH — new pass

#### Stage 5c: Large Slot Diff Triage (est: +5-10 fixtures)

- [ ] Sample ±3+ slot diff fixtures for patterns sharing root cause with Stage 3 fixes
- [ ] Cherry-pick easy wins only

---

### Milestone Summary (revised post-investigation)

| Stage | Target | Cumulative (from 411) | Risk | Notes |
|-------|--------|-----------------------|------|-------|
| Stage 1b: Temp renumbering | +2 (done) | 405 | LOW | Completed. |
| Stage 1c: Minor codegen fixes | +5 (done) | 410 | LOW | Completed. |
| Stage 2a: Bail-out investigation | +0 (done) | 410 | -- | Completed. |
| Stage 2b: File-level bail-outs | +1 (done) | 411 | LOW | Completed. 108→89 bail-outs. |
| Stage 2c: `_exp` directive handling | +0 (done) | 411 | LOW | Completed. 20 fixtures moved to compile pools. |
| Stage 2d: Frozen-mutation false positives | +5-8 | 416-419 | MEDIUM | 11 fixtures |
| Stage 2e: Ref-access false positives | +3-5 | 419-424 | MEDIUM | 8 fixtures |
| Stage 2f: Reassignment false positives | +5-7 | 424-431 | MEDIUM | 10 fixtures |
| Stage 2g: Other bail-outs | +5-10 | 429-441 | MIXED | ~40 remaining fixtures |
| Stage 1d: Declaration placement | +15-30 | 444-471 | HIGH | collect_all_scope_declarations redesign |
| B2: Variable name preservation | +20-30 | 464-501 | MEDIUM | 40 fixtures, scope output naming |
| Stage 3: Scope inference (±1/±2 diffs) | +50-100 | 514-601 | HIGH | ~699 pool, cascading regression risk |
| Stage 4b: Preserve-memo validation | +15-25 | 529-626 | MEDIUM | 32 fixtures |
| Stage 4c: Todo error detection | +15 (done, 5 remain) | 426 | LOW | 22/27 done (15 in 4c + 7 in 4e-A). Remaining 5 need hoisting, optional terminals, context vars. |
| Stage 4d: Frozen-mutation false negatives | +9 (done) | 435 | MEDIUM | Completed. 7/9 planned + 2 bonus. 2 remain (effect callback, JSX capture). |
| Stage 4e-A: Upstream error bail-outs | +7 (done) | 442 | LOW | 7/43 done. 4e-B through 4e-E remain. |
| Stage 4e-B: Locals/ref/setState/hooks | +1 so far | 443 | LOW | 1/5 done (hooks-in-loop). 4 remain. |
| Stage 4e-C/D/E: Remaining upstream errors | +21-38 | 464-481 | MED-HIGH | 4e-C (3, MED), 4e-D (11, MED-HIGH), 4e-E (7, HIGH) |
| Stage 5: DCE + constant propagation | +30-50 | 604-754 | HIGH | 76 fixtures, new passes needed |
| **Total remaining** | **+161-311** | **604-754** | | From 443 base |

**Key learning from Stage 1b:** Temp renumbering alone is nearly worthless (+2). Naming and ordering are entangled — fixing one without the other does not pass conformance.

**Key learning from Stage 2a/2b:** Most bail-outs come from specific validations, not silent/0-scope issues. File-level bail-outs were low-hanging fruit (+1 net from removing 4).

**Key correction (2026-03-25):** The 88 error.* figure was pre-Stage-4c/4d. After Stage 4e-A, **39 error.* fixtures remain in known-failures** (37 top-level + 2 fbt/). Down from 43 pre-4e-A.

**Key learning from Stage 2c:** Fixing bail-outs does not directly increase conformance if the newly-compiling fixtures land in slots-DIFFER/MATCH pools. Bail-out fixes unblock fixtures for FUTURE scope/codegen improvements but yield +0 net on their own.

**Key learning from extended investigation (2026-03-25):**
- "Both no memo" is NOT format diffs — requires DCE + constant propagation (new compiler passes)
- "We compile, they don't" has 75 UPSTREAM ERROR fixtures — significant untapped pool if we match error formats
- Slots-MATCH B2 pattern (40 fixtures) is the single largest tractable codegen fix remaining
- `validatePreserveExistingMemoizationGuarantees` gaps account for 32 of the "we compile, they don't" fixtures

**Revised path to 600:** Reachable via scope inference fixes (Stage 3, +50-100) + validation gaps (Stage 4, +37-80 remaining) + codegen fixes (B2 + 1d, +35-60). DCE/constant propagation (Stage 5) could push well past 600 but is the hardest work. Conservative floor: ~577. Optimistic: 700+.

**Key principle:** Each stage starts with investigation (sub-task "a") that produces a fixture-level breakdown. If the investigation shows estimates are wrong, the plan is updated before implementation begins. No blind implementation.

---

## Deferred / Blocked Work

### Phase 2 Remaining: Impure Function Handling — DEFERRED

**Files:** `src/inference/infer_mutation_aliasing_effects.rs`, `src/validation/`
- Impure function handling in legacy signatures — requires `validate_no_impure_functions_in_render` integration

### Phase 4c: Remove `validate_no_mutation_after_freeze.rs` — BLOCKED

**Files:** `src/validation/validate_no_mutation_after_freeze.rs`
- Cannot remove yet — standalone validator has hook-call-freezes-captures logic not in effects pass
- Phase 119 gap: LoadGlobal hook names not resolved in id_to_name

### Phase 4d: Switch to `mutable_range` — NOT READY

**Files:** `src/reactive_scopes/infer_reactive_scope_variables.rs`
- 6 failed attempts. Over-splitting regressions. `use_mutable_range` flag preserved for A/B testing.
- May be revisited after Stage 3 scope inference improvements

### Phase 5: Fault Tolerance — BLOCKED until 600+

- PanicThreshold change to CriticalErrors requires ~600+ conformance
- 132 bail-out fixtures produce wrong output when compiled instead of bailed

### Performance: O(n^2+) Scaling — DEFERRED

- Effects/aliasing passes have O(n^2+) scaling
- Deferred until correctness work stabilizes

---

## Critical Architecture Notes

**Read these before making ANY changes.**

### `effective_range` vs `mutable_range` — STILL NEEDED
File: `src/reactive_scopes/infer_reactive_scope_variables.rs`
Uses `effective_range = max(mutable_range.end, last_use + 1)`. 6 failed switch attempts. The `use_mutable_range` flag on EnvironmentConfig is preserved for A/B testing.

### `collect_all_scope_declarations` is load-bearing
File: `src/reactive_scopes/codegen.rs`
Pre-declares ALL scope output variables at function level. Removing it causes render to drop 96%->24%.

### Block iteration order != source order for loops
HIR blocks stored in creation order; for-loop constructs create blocks out of source order.

### Cross-scope `IdentifierId` mismatch
Nested function bodies have their own `IdentifierId` numbering. Name-based resolution needed.

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

## Key File Reference

| Purpose | Path |
|---------|------|
| Pipeline orchestration | `src/entrypoint/pipeline.rs` |
| HIR types | `src/hir/types.rs` |
| HIR builder (AST->HIR) | `src/hir/build.rs` |
| Code generation | `src/reactive_scopes/codegen.rs` |
| Aliasing effects | `src/inference/aliasing_effects.rs` |
| Mutation effects | `src/inference/infer_mutation_aliasing_effects.rs` |
| Mutation ranges | `src/inference/infer_mutation_aliasing_ranges.rs` |
| Function analysis | `src/inference/analyse_functions.rs` |
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

---

## Upstream References

| Resource | URL |
|----------|-----|
| New aliasing model PR | `facebook/react#33494` |
| New model documentation | `compiler/.../Inference/MUTABILITY_ALIASING_MODEL.md` |
| Old code removal | `facebook/react#34028`, `#34029` |
| Fault tolerance | `facebook/react#35872` through `#35888` |
| Exhaustive deps validation | `facebook/react#34394` |
| Feature flag cleanup | `facebook/react#35825` |
| Fallback pipeline removed | `facebook/react#35827` |
| React Compiler v1.0 blog | `react.dev/blog/2025/10/07/react-compiler-1` |

---

## Lessons Learned

1. **effective_range is load-bearing.** 6 attempts to switch to mutable_range have shown regressions.
2. **collect_all_scope_declarations cannot be removed.** It prevents render collapse from 96% to 24%.
3. **PanicThreshold change to CriticalErrors requires ~600+ conformance.** 132 bail-out fixtures produce wrong output when compiled instead of bailed.
4. **Emitting 0-slot functions requires more error validations.** 68 divergences when attempted (Phase 121).
5. **Render regressions can be latent.** The PostfixUpdate/PrefixUpdate codegen bug existed for months.
6. **Fix low-risk bail-outs before high-risk scope inference.** Easy wins first.
7. **Preserve-memo validation needs `ManualMemoDependency` for full upstream fidelity.**
8. **Performance regression from Phases 113-130.** O(n^2+) scaling in effects/aliasing passes. Deferred.
9. **Expected file generation must use `compilationMode: "all"`.** Fixed in Phase 133.
10. **Each stage must start with investigation.** Blind implementation wastes effort. Investigate → plan → implement → verify → replan.
11. **Naming diffs co-occur with ordering diffs.** Temp renumbering alone gained only +2 (expected +25-40). Most "variable naming" fixtures also differ in instruction ordering or declaration placement. Fixing names without fixing ordering does not pass conformance.
12. **File-level bail-outs are wasteful.** 4 overly aggressive file-level bail-outs (lint mode, incompatible imports, eslint suppression, runtime import) were blocking 19 fixtures unnecessarily. Upstream handles these per-function. Always prefer per-function bail-outs over file-level.
13. **`validateNoDerivedComputationsInEffects` is the largest single bail-out source.** 20 of 89 remaining false-positive bail-outs come from this one validation. Fixing it is the highest-ROI next step for bail-out reduction.
14. **Fixing bail-outs yields +0 net conformance if output still differs.** Stage 2c moved 20 fixtures from bail to compile, but all landed in slots-DIFFER/MATCH pools. Bail-out fixes UNBLOCK fixtures for future scope/codegen fixes but do not directly increase conformance. Plan accordingly.
15. **"Both no memo" requires DCE + constant propagation.** Originally assumed to be cosmetic format diffs. Actually requires dead-code elimination and constant folding passes that don't exist yet. 76 fixtures affected.
16. **"We compile, they don't" has a large UPSTREAM ERROR sub-pool.** 75 of ~225 fixtures have error messages as expected output. These are a significant conformance opportunity if we match upstream error formats precisely.
17. **B2 (variable name preservation) is the dominant tractable slots-MATCH pattern.** 40 fixtures where we use temps but upstream preserves original names in scope outputs. Single largest fixable sub-pattern in the 237-fixture slots-MATCH pool.
18. **Todo error fixtures live in known-failures, not in the "not in KF" set.** The 16 fixtures initially identified as Todo-error targets were already passing. The actual Todo-error fixtures were in the UPSTREAM ERROR subset of known-failures. Always check known-failures.txt for UPSTREAM ERROR fixtures when looking for validation gaps.
19. **Low-hanging Todo errors yield good ROI.** Stage 4c gained +15 from simple pattern detection (try-without-catch, computed keys, value-blocks-in-try, throw-in-try, fbt locals). The remaining 12 need deeper compiler infrastructure (hoisting, optional terminals, default params).
20. **UPSTREAM ERROR conformance only checks `!transformed`, not error message content.** The conformance test (line 775-781 of conformance_tests.rs) passes an UPSTREAM ERROR fixture if `compile_result.transformed` is false. We do NOT need to match the exact upstream error string. This makes the task much simpler: just bail, don't need error format matching.
21. **Name-based freeze tracking trades false negatives for false positives.** Stage 4d gained +9 by tracking frozen identifiers by name (solving cross-scope IdentifierId mismatch), but introduced 4 IIFE-pattern false positives where captured variables are mutated inside IIFEs. Net gain is still positive (+9 gained, -4 shifted to bail = +5 net new passing + 4 category shifts). Future improvement: scoped name tracking that understands IIFE boundaries.
22. **Nested HIR builders don't emit LoadContext instructions.** When a nested function is lowered by a child `HIRBuilder`, context variables (captured from outer scope) are represented as plain `LoadLocal` in the nested HIR, not `LoadContext`. This means walking the nested HIR cannot distinguish context variables from local variables. The upstream compiler uses `LoadContext` to identify captured variables in nested lambdas. Fixing `error.todo-handle-update-context-identifiers.js` requires either (a) emitting `LoadContext` in nested builders, or (b) passing parent scope binding information to the validation pass. This is a structural limitation, not a simple pattern-matching fix.
