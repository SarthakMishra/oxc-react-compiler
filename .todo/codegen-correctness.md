# Codegen Correctness -- Variable Naming & Output Quality

> Discovered during real-world benchmarking against babel-plugin-react-compiler.
> Speed is 80-95x faster. All variable naming issues are now resolved.

---

## Gap 1: Variable reference naming mismatch (`_tN` vs `tN`) ✅

~~**Previous:** The `promote_used_temporaries` pass renamed only lvalue identifiers, leaving operand references as `_tN` instead of `tN`, producing broken JavaScript output.~~

**Completed**: Fixed `promote_used_temporaries` in `prune_scopes.rs` to walk ALL places in each instruction -- operands, terminals, scope deps/decls -- not just lvalues. All `_tN` references are now consistently `tN`. File: `crates/oxc_react_compiler/src/reactive_scopes/prune_scopes.rs`.

---

## Gap 2: JSX element names use broken `_tN` references ✅

~~**Previous:** JSX codegen emitted `<_t66 src={_t67} />` instead of `<t66 src={t67} />` due to Gap 1.~~

**Completed**: Fixed automatically by Gap 1 fix. All JSX tag and attribute places now use correct `tN` naming.

---

## Gap 3: `place_name()` fallback uses underscore prefix ✅

~~**Previous:** `place_name()` and `codegen_scope()` used `format!("_t{}", id)` as the fallback, creating inconsistent naming.~~

**Completed**: Changed all `format!("_t{}", id)` to `format!("t{}", id)` in `codegen.rs` -- both the `place_name()` function and `codegen_scope()` patterns. File: `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs`.
