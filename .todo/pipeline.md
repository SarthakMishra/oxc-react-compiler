# Pipeline Orchestration Gaps

> The pipeline orchestrator (`entrypoint/pipeline.rs`) and program entry point
> (`entrypoint/program.rs`) are the backbone of end-to-end compilation. Currently,
> `compile_program` discovers functions but never transforms them, and `run_pipeline`
> stops at pass 28.

---

## Gap 1: Wire compile_program to BuildHIR

**Upstream:** `packages/babel-plugin-react-compiler/src/Entrypoint/Program.ts` (compileFn)
**Current state:** `compile_program` in `entrypoint/program.rs` discovers functions via
`discover_functions` but the loop at line 50-63 just returns `transformed: false`. It never
calls `HIRBuilder::build()` to lower the discovered function's AST to HIR.
**What's needed:**
- For each `DiscoveredFunction`, extract the function's AST node from the parsed program
- Call `HIRBuilder::new().build(function_body)` to produce an `HIR`
- Pass the `HIR` to `run_pipeline()`
- Collect the function's params, id, loc, and directives for later use in `build_reactive_function`
- Handle errors: if any function fails to compile, return the original source for that function
**Depends on:** None (BuildHIR already exists at 2,626 lines in `hir/build.rs`)

---

## Gap 2: Wire Reactive Scope Construction Passes 29-46

**Upstream:** `packages/babel-plugin-react-compiler/src/ReactiveScopes/` (multiple files)
**Current state:** `run_pipeline` in `entrypoint/pipeline.rs` stops after pass 28 with
`// TODO` comments at lines 144-149. All the individual passes exist as functions in
`reactive_scopes/*.rs` and `inference/infer_reactive_places.rs`, but they are never called.
**What's needed:**
Add calls to `run_pipeline` for passes 29-46 in order:
```
29: infer_reactive_places
30: validate_exhaustive_dependencies (conditional)
31: rewrite_instruction_kinds_based_on_reassignment (NOTE: currently at pass 12 position -- move to correct position)
32: validate_static_components (conditional)
33: infer_reactive_scope_variables
34: memoize_fbt_and_macro_operands_in_same_scope
35: outline_jsx (conditional)
36: name_anonymous_functions (NOTE: currently at wrong position -- move to pass 36)
37: outline_functions (conditional)
38: align_method_call_scopes
39: align_object_method_scopes
40: prune_unused_labels_hir
41: align_reactive_scopes_to_block_scopes_hir
42: merge_overlapping_reactive_scopes_hir
43: build_reactive_scope_terminals_hir
44: flatten_reactive_loops_hir
45: flatten_scopes_with_hooks_or_use_hir
46: propagate_scope_dependencies_hir
```
- All these functions already exist -- they just need to be called
- Fix pass ordering: `rewrite_instruction_kinds` is currently at pass 12 position but
  upstream has it at pass 31; `name_anonymous_functions` is at wrong position
- The `infer_reactive_scope_variables` return value (Vec<ReactiveScope>) needs to be
  applied back onto the HIR identifiers before the subsequent alignment/merge passes can work
**Depends on:** Gap 1 (need HIR to pass through the pipeline)

---

## Gap 3: Wire Build Reactive Function and RF Optimization Passes 47-60

**Upstream:** `packages/babel-plugin-react-compiler/src/ReactiveScopes/BuildReactiveFunction.ts`
and `packages/babel-plugin-react-compiler/src/ReactiveScopes/` (prune/merge/rename files)
**Current state:** `build_reactive_function` exists and is complete (298 lines). All RF
optimization passes exist in `reactive_scopes/prune_scopes.rs` (840 lines). But none are
called from the pipeline.
**What's needed:**
After pass 46, the pipeline needs to:
1. Call `build_reactive_function(hir, params, id, loc, directives)` to produce a `ReactiveFunction`
2. Call RF optimization passes 48-60 on the `ReactiveFunction`:
   - `prune_unused_labels`
   - `prune_non_escaping_scopes`
   - `prune_non_reactive_dependencies`
   - `prune_unused_scopes`
   - `merge_reactive_scopes_that_invalidate_together`
   - `prune_always_invalidating_scopes`
   - `propagate_early_returns`
   - `prune_unused_lvalues`
   - `promote_used_temporaries`
   - `extract_scope_declarations_from_destructuring`
   - `stabilize_block_ids`
   - `rename_variables`
   - `prune_hoisted_contexts`
3. Return the `ReactiveFunction` from `run_pipeline` (currently returns `Result<(), ()>`,
   needs to return `Result<ReactiveFunction, ()>`)
**Depends on:** Gap 2 (reactive scopes must be constructed before building RF)

---

## Gap 4: Wire Codegen and Source Replacement

**Upstream:** `packages/babel-plugin-react-compiler/src/Entrypoint/Program.ts` (insertNewFunctionNode)
**Current state:** `codegen_function` exists (690 lines in `reactive_scopes/codegen.rs`) and
`apply_compilation` exists for text replacement. But `compile_program` never calls them.
**What's needed:**
- After `run_pipeline` returns a `ReactiveFunction`, call `codegen_function(&rf)` to produce
  compiled JS code
- Collect `(Span, String)` pairs for each successfully compiled function
- Call `apply_compilation(source, &compiled_functions)` to produce the final output
- Set `transformed = true` if any function was successfully compiled
- Pass 61 (`validate_preserved_manual_memoization`) should run on the RF before codegen
  when `enable_preserve_existing_memoization_guarantees` is set
**Depends on:** Gap 3 (need ReactiveFunction to codegen)

---

## Gap 5: Wire Validation Passes 12-13

**Upstream:** `packages/babel-plugin-react-compiler/src/Validation/ValidateHooksUsage.ts`,
`packages/babel-plugin-react-compiler/src/Validation/ValidateNoCapitalizedCalls.ts`
**Current state:** `validate_hooks_usage` exists as a complete function (116 lines) but the
pipeline has it commented out at line 63-67 with `// TODO:` comments.
`validate_no_capitalized_calls` also exists but is commented out.
**What's needed:**
- Remove the TODO comments and wire the actual function calls in the pipeline
- These are conditional on `config.validate_hooks_usage` and `config.validate_no_capitalized_calls`
**Depends on:** None

---

## Gap 6: Wire validate_exhaustive_dependencies

**Upstream:** `packages/babel-plugin-react-compiler/src/Validation/ValidateExhaustiveDependencies.ts`
**Current state:** `validate_exhaustive_dependencies` exists as a complete function but is
never called from the pipeline. It should be pass 30, between `infer_reactive_places` and
`rewrite_instruction_kinds`.
**What's needed:**
- Add call to `validate_exhaustive_dependencies(hir, errors)` at pass 30 position,
  conditional on config
**Depends on:** Gap 2 (pass 30 is part of the reactive scope construction phase)

---

## Gap 7: Wire validate_locals_not_reassigned_after_render

**Upstream:** `packages/babel-plugin-react-compiler/src/Validation/ValidateLocalsNotReassignedAfterRender.ts`
**Current state:** `validate_locals_not_reassigned_after_render.rs` exists but is never
called from the pipeline. It should be pass 21, right after `infer_mutation_aliasing_ranges`.
**What's needed:**
- Add call at pass 21 position (currently pass 21 is listed as `assert_valid_mutable_ranges`
  in the pipeline, but upstream has `validate_locals_not_reassigned_after_render` at 21 and
  `assert_valid_mutable_ranges` at 22)
**Depends on:** None (the function exists)

---

## Gap 8: Wire validate_preserved_manual_memoization

**Upstream:** `packages/babel-plugin-react-compiler/src/Validation/ValidatePreservedManualMemoization.ts`
**Current state:** The function exists and operates on `ReactiveFunction` (pass 61). It is
never called because the pipeline never reaches RF construction.
**What's needed:**
- Call after RF optimization passes (pass 60), before codegen (pass 62)
- Conditional on `config.enable_preserve_existing_memoization_guarantees`
**Depends on:** Gap 3 (needs ReactiveFunction to exist)
