# Codegen Correctness -- Variable Naming & Output Quality

> Discovered during real-world benchmarking against babel-plugin-react-compiler.
> Speed is 80-95x faster, but output is not yet correct/executable.

---

## Gap 1: Variable reference naming mismatch (`_tN` vs `tN`)

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Current state:** The `promote_used_temporaries` pass (Pass 56) renames **lvalue** identifiers from `None` to `Some("t{id}")`, but does not walk into **operand** places of other instructions. When codegen calls `place_name()` for an operand whose `name` is still `None`, it falls back to `format!("_t{}", id)` -- producing `_t56` instead of `t56`.

**What's needed:**

- Fix `promote_used_temporaries` in `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs` to walk ALL places in each instruction (operands, not just lvalues)
- Or alternatively, fix `place_name()` in `codegen.rs` to use the same `"t{id}"` format (without underscore) as the fallback. This is the simpler fix but the upstream compiler renames all references in the pass, so the pass fix is more correct.
- The helper needs to walk into: `CallExpression` callee + args, `MethodCall` receiver + args, `PropertyLoad` object, `PropertyStore` object + value, `BinaryExpression` left + right, `UnaryExpression` value, `JsxExpression` tag + props + children, `JsxFragment` children, `ObjectExpression` property keys/values, `ArrayExpression` elements, `TemplateLiteral` subexpressions, `NewExpression` callee + args, `Await` value, `LoadLocal` place, `StoreLocal` value, terminal places (Return, Throw, If test, etc.), and scope dependency/declaration identifiers
- Must also rename identifiers in `ReactiveTerminal` variants (e.g., `Return { value }`, `If { test }`, etc.)

**Depends on:** None -- this is the highest priority fix since it makes ALL output broken.

**Impact:** Without this fix, every compiled function produces syntactically invalid JavaScript. This blocks all downstream correctness work.

**Files to modify:**
- `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs` (lines 472-497, `promote_used_temporaries` and `promote_temps_in_block`)

---

## Gap 2: JSX element names use broken `_tN` references

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Current state:** JSX codegen in `codegen.rs` emits `<_t66 src={_t67} />` instead of `<t66 src={t67} />`. This is a direct consequence of Gap 1 -- JSX tag and attribute places have `name: None` because `promote_used_temporaries` didn't walk into JSX operands.

**What's needed:** Fixing Gap 1 will fix this automatically. No separate work needed.

**Depends on:** Gap 1

---

## Gap 3: `place_name()` fallback uses underscore prefix

**Upstream:** `src/ReactiveScopes/CodegenReactiveFunction.ts`
**Current state:** `place_name()` at line 551-553 of `codegen.rs` uses `format!("_t{}", id)` as the fallback for unnamed identifiers. The underscore prefix creates invalid variable references even when the declaration uses `t{id}` (set by `promote_used_temporaries`).

**What's needed:**

- Change `place_name()` fallback from `format!("_t{}", id)` to `format!("t{}", id)` to match the naming convention used by `promote_used_temporaries`
- Same change needed in `codegen_scope()` at lines 397, 415, 434, 452 where `map_or_else(|| format!("_t{}", ...))` patterns appear
- This is a belt-and-suspenders fix -- even after fixing Gap 1, any identifiers that slip through should use consistent naming

**Depends on:** None (can be done independently as a quick fix, or alongside Gap 1)

**Files to modify:**
- `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` (line 552, lines 397, 415, 434, 452)
