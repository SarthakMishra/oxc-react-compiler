# Codegen Structure Divergences (150 fixtures)

Slot count matches (`_c(N)` identical) but the generated code within
scopes differs. These are the closest-to-passing fixtures.

## Structure Divergences

**Symptom:** `_c(N)` matches but token comparison fails.

**Likely root causes (in order of impact):**

### 1. Temp variable inlining differences

Our codegen emits intermediate SSA temporaries that upstream inlines.
For example, we emit `const t0 = props.x; return t0;` where upstream
emits `return props.x;`.

Cross-scope temp inlining infrastructure is already built in `codegen.rs`
but may not cover all patterns. Key cases:
- Temps used once should be inlined at their use site
- Temps that are just LoadLocal of a named variable should be replaced
  with the variable name
- Property chains (`t0 = a; t1 = t0.b; t2 = t1.c`) should collapse
  to `a.b.c`

**Upstream:** `CodegenReactiveFunction.ts` -- temps are inlined during
IR-to-source translation.

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` (temp use counting, inlining)

### 2. Scope declaration ordering

Our codegen may emit scope declarations in a different order than upstream.
Within a scope guard's if-block, the order of `$[N] = value` assignments
matters for comparison.

### 3. Dependency ordering within scope guards

The order of dependency checks in the if-condition (`$[0] !== deps0 ||
$[1] !== deps1`) may differ from upstream.

### 4. Scope nesting structure

When scopes are nested (scope A contains scope B), the nesting structure
may differ even when total slot counts match.

## Diagnostic Approach

These 150 fixtures are the "closest to passing" -- they already have
correct scope structure. Fixing temp inlining alone could unlock a
significant portion.

1. Sample 10 fixtures, diff our output token-by-token against expected
2. Categorize: is the diff temp naming, ordering, or structural?
3. Focus on the most common diff pattern first

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
