# Validation & Coverage Gaps

These issues reduce conformance coverage but do not break the core compilation pipeline for patterns we do handle.

---

## Gap 5: False Bail-outs (208 Fixtures)

**Priority:** P2 -- lower coverage, no runtime breakage

**Current state:** We bail out of compilation too conservatively in 208 conformance fixtures. The compiler reports an error and skips compilation, but the upstream compiler successfully compiles these fixtures. Breakdown:

- **58 false "memoization preservation" errors** -- We incorrectly detect that memoization would not be preserved
- **26 false "frozen mutation" errors** -- We flag mutations of frozen values that the upstream compiler allows (recent fix in `ca2374d` may have addressed some)
- **16 false "reassigned after render" errors** -- We report reassignment-after-render violations that the upstream compiler does not flag
- **~108 other false bail-outs** -- Various other conservative checks that reject valid programs

**What's needed:**
- Audit each validation pass against its upstream TypeScript equivalent
- Each sub-category should be investigated independently; they likely have different root causes
- Start with the 58 memoization preservation errors (largest category)

**Upstream:**
- `src/Validation/ValidatePreservingMemoization.ts`
- `src/Validation/ValidateNoRefAccessInRender.ts`
- `src/Validation/ValidateFrozenValues.ts`

**Depends on:** None

---

## Gap 6: Silent Bail-outs (66 Fixtures)

**Priority:** P2 -- missing features

**Current state:** 66 conformance fixtures produce 0 reactive scopes and no error message. Categories include:
- **Try/catch blocks** -- Not building HIR for try/catch
- **Sequence expressions** -- Comma-separated expressions not lowered into HIR
- **Destructuring defaults** -- Default values in destructuring patterns
- **Type constructs** -- Type assertions or other type-level constructs blocking HIR construction

**What's needed:**
- Categorize the 66 fixtures by failure pattern
- Prioritize categories by frequency (fix the pattern that covers the most fixtures first)

**Depends on:** None

---

## Gap 7: toolbar -- semantic_difference Bail

**Priority:** P2 -- 0 scopes emitted for a fixture Babel successfully compiles

**Current state:** The toolbar benchmark fixture produces 0 reactive scopes because we bail with a `semantic_difference` error. Babel successfully compiles this fixture. This is likely a false positive in our semantic equivalence checking.

**What's needed:**
- Investigate which semantic check triggers the bail-out
- Determine if false positive (relax the check) or genuine codegen issue (fix the output)

**Depends on:** None
