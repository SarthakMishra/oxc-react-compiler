# Codegen Structure Divergences (270 fixtures)

Slot count matches (`_c(N)` identical) but the generated code within
scopes differs. These are the closest-to-passing fixtures. This number
grew from 150 to 270 as other fixes brought more fixtures into the
"slots match" category.

## Structure Divergences

**Symptom:** `_c(N)` matches but token comparison fails.

### 1. Scope output variable renaming (rename_variables) -- DONE

**Completed**: Implemented `rename_variables` pass in `prune_scopes.rs`.
Sequential temp names (`t0`, `t1`, ...) for reactive scope declaration
outputs, with collision avoidance and eligibility checks. File:
`crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`.

### 2. Temp variable inlining differences

Our codegen emits intermediate SSA temporaries that upstream inlines.
For example, we emit `const t0 = props.x; return t0;` where upstream
emits `return props.x;`.

**Improvements already made:**
- Cross-scope LoadLocal inlining for named variables (Phase 81)
- Post-SSA LoadLocal temp inlining pass (+35 conformance, Phase 82)

**Remaining patterns:**
- Property chain collapse (`t0 = a; t1 = t0.b` -> `a.b`)
- Temps in scope dependency positions
- Destructuring pattern temps
- More complex multi-use temp patterns

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`

### 3. Scope declaration ordering

Within a scope guard's if-block, the order of `$[N] = value` assignments
may differ from upstream.

### 4. Dependency ordering within scope guards

The order of dependency checks in the if-condition may differ.

### 5. Scope nesting structure

When scopes are nested, the nesting structure may differ even when
total slot counts match.

## Diagnostic Approach

These 270 fixtures are the "closest to passing" -- they already have
correct scope structure. Fixing temp inlining alone could unlock a
significant portion.

1. Sample 10 fixtures, diff our output token-by-token against expected
2. Categorize: is the diff temp naming, ordering, or structural?
3. Focus on the most common diff pattern first

**Key files:**
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`
