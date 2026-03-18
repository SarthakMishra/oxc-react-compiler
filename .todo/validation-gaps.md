# Validation & Coverage Gaps

These issues reduce conformance coverage but do not break the core compilation pipeline for patterns we do handle. They should be addressed after the P0 codegen emission bugs are resolved.

---

## Gap 5: False Bail-outs (208 Fixtures)

**Priority:** P2 -- lower coverage, no runtime breakage

**Current state:** We bail out of compilation too conservatively in 208 conformance fixtures. The compiler reports an error and skips compilation, but the upstream compiler successfully compiles these fixtures. Breakdown:

- **58 false "memoization preservation" errors** -- We incorrectly detect that memoization would not be preserved, when the upstream compiler determines it is safe
- **26 false "frozen mutation" errors** -- We flag mutations of frozen values that the upstream compiler allows (likely cases where the value is not actually frozen at that point)
- **16 false "reassigned after render" errors** -- We report reassignment-after-render violations that the upstream compiler does not flag
- **~108 other false bail-outs** -- Various other conservative checks that reject valid programs

**What's needed:**

- Audit each validation pass against its upstream TypeScript equivalent
- For "memoization preservation": check if our scope analysis is creating false dependencies that make memoization look unsafe
- For "frozen mutation": verify the frozen-value tracking accounts for SSA versioning correctly (recent fix in `ca2374d` may have addressed some of these)
- For "reassigned after render": check if we correctly identify which assignments happen during render vs in callbacks/effects
- Each sub-category should be investigated independently; they likely have different root causes

**Upstream files:**
- `src/Validation/ValidatePreservingMemoization.ts`
- `src/Validation/ValidateNoRefAccessInRender.ts`
- `src/Validation/ValidateFrozenValues.ts`

**Depends on:** None (independent of codegen fixes), but lower priority

---

## Gap 6: Silent Bail-outs (66 Fixtures)

**Priority:** P2 -- missing features

**Current state:** 66 conformance fixtures produce 0 reactive scopes and no error message. The compiler silently produces uncompiled output. These represent patterns we fail to recognize as compilable. Categories include:

- **Try/catch blocks** -- We may not be building HIR for try/catch, causing the entire function to be skipped
- **Sequence expressions** -- Comma-separated expressions (`(a, b, c)`) may not be lowered into HIR
- **Destructuring defaults** -- Default values in destructuring patterns (`const { x = 5 } = obj`) may not be handled
- **Flow/TypeScript type constructs** -- Type assertions, satisfies expressions, or other type-level constructs that should be stripped but may be blocking HIR construction
- **Feature gating** -- Some patterns may be behind feature flags that we don't implement

**What's needed:**

- Categorize the 66 fixtures by failure pattern (which HIR construction step fails or produces empty output)
- For each category, determine whether the fix is in the parser/HIR lowering, the scope analysis, or elsewhere
- Prioritize categories by frequency (fix the pattern that covers the most fixtures first)

**Depends on:** None (independent of codegen fixes), but lower priority
