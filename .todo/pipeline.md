# Pipeline Orchestration

> Top-level pipeline that chains all 62 passes together, program-level compilation loop, and optional passes.
> Upstream: `src/Entrypoint/Pipeline.ts`, `src/Entrypoint/Program.ts`
> Rust modules: `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`, `program.rs`

---

### Gap 1: Pipeline Orchestrator

**Upstream:** `src/Entrypoint/Pipeline.ts`
**Current state:** `entrypoint/pipeline.rs` is a stub.
**What's needed:**

The main `compile_function()` entry point that runs all 62 passes in order:

```rust
pub fn compile_function(func: &mut HIRFunction, env: &Environment) -> Result<ReactiveFunction, CompilerError> {
    // Phase 1: HIR Construction & Early Cleanup
    prune_maybe_throws(&mut func.body);                      // #2
    validate_context_variable_lvalues(&func.body)?;           // #3
    validate_use_memo(&func.body)?;                           // #4
    if !env.config.preserve_memoization { drop_manual_memoization(&mut func.body); } // #5
    inline_iife(&mut func.body);                              // #6
    merge_consecutive_blocks(&mut func.body);                  // #7

    // Phase 2: SSA
    enter_ssa(&mut func.body);                                // #8
    eliminate_redundant_phi(&mut func.body);                   // #9

    // ... (all 62 passes)

    // Phase 11: Codegen
    validate_preserved_manual_memoization(&rf)?;              // #61
    let output = codegen_function(&rf, env);                  // #62
    Ok(output)
}
```

- Each pass is gated by configuration flags where appropriate
- Error handling: some passes bail on errors, others collect and continue
- `PanicThreshold` determines whether to bail or continue on errors
- Lint mode: run passes but skip codegen, return diagnostics
- Pass timing instrumentation (optional, for performance profiling)

**Depends on:** All individual passes must exist (even as stubs) to wire the pipeline

---

### Gap 2: Program-Level Compilation Loop

**Upstream:** `src/Entrypoint/Program.ts`
**Current state:** `entrypoint/program.rs` is a stub.
**What's needed:**

The top-level entry point that processes an entire file:

```rust
pub fn compile_program(
    allocator: &Allocator,
    source: &str,
    source_type: SourceType,
    options: &PluginOptions,
) -> CompileResult {
    // 1. Parse with OXC
    let parser_ret = Parser::new(allocator, source, source_type).parse();

    // 2. Run semantic analysis
    let semantic = SemanticBuilder::new(source).build(&parser_ret.program);

    // 3. Discover compilable functions
    let functions = discover_functions(&parser_ret.program, &semantic, &options);

    // 4. For each function:
    let mut edits = Vec::new();
    for (func_node, func_type) in functions {
        // a. Lower to HIR
        let hir = lower(&func_node, &semantic, &options.env_config);
        // b. Run pipeline
        match compile_function(&mut hir, &options.env_config) {
            Ok(compiled) => edits.push(Edit { span: func_node.span(), replacement: compiled }),
            Err(e) => { /* collect error or bail */ }
        }
    }

    // 5. Apply edits to source
    let output = apply_edits(source, &edits);

    // 6. Insert imports if any functions were compiled
    if !edits.is_empty() {
        insert_compiler_runtime_import(&mut output, &options);
    }

    CompileResult { code: output, transformed: !edits.is_empty(), diagnostics }
}
```

- `CompileResult` struct: `code: String`, `source_map: Option<String>`, `transformed: bool`, `diagnostics: Vec<OxcDiagnostic>`
- Edit application: replace function body spans with compiled output
- Handle multiple functions in one file
- Handle nested functions (compiled as part of parent via `AnalyseFunctions`)

**Depends on:** Gap 1, Function discovery (build-hir.md Gap 8), Codegen (codegen.md Gap 1)

---

### Gap 3: Lint Mode

**Upstream:** `eslint-plugin-react-compiler` uses `outputMode: 'lint'`
**Current state:** Nothing implemented.
**What's needed:**

When `OutputMode::Lint` is set:

- Run the full pipeline (all analysis and validation passes)
- Skip codegen (pass #62)
- Collect all diagnostics from validation passes
- Return diagnostics without modifying the source
- This is used by:
  - oxlint Tier 2 rules that need compiler analysis
  - The eslint plugin compatibility mode

**Depends on:** Gap 1 (pipeline with error collection)

---

### Gap 4: OptimizeForSSR

**Upstream:** `src/Optimization/OptimizeForSSR.ts` (inferred)
**Pipeline position:** Pass #17, Phase 5
**Current state:** No file exists yet.
**What's needed:**

SSR-specific optimization:

- When `OutputMode::SSR`, skip memoization entirely (SSR is single-render)
- Remove reactive scope creation
- Simplify output to just execute the function body
- May remove hook calls that are SSR-irrelevant

**Depends on:** InferMutationAliasingEffects (runs right after it)

---

### Gap 5: OutlineJSX

**Upstream:** `src/ReactiveScopes/OutlineJSX.ts` (inferred)
**Pipeline position:** Pass #35, Phase 8
**Current state:** No file exists yet.
**What's needed:**

Optional: extract JSX expressions into separate outlined functions:

- For JSX subtrees that are independently memoizable
- Creates smaller, more granular reactive scopes
- Controlled by `EnvironmentConfig.enable_jsx_outlining`
- Can be deferred (optional optimization)

**Depends on:** InferReactiveScopeVariables

---

### Gap 6: NameAnonymousFunctions

**Upstream:** `src/ReactiveScopes/NameAnonymousFunctions.ts` (inferred)
**Pipeline position:** Pass #36, Phase 8
**Current state:** No file exists yet.
**What's needed:**

Give names to anonymous function expressions for better debugging:

- Assign names based on the variable they're assigned to
- Or based on their usage context (e.g., `onClick` for an event handler prop)
- Optional pass

**Depends on:** InferReactiveScopeVariables

---

### Gap 7: OutlineFunctions

**Upstream:** `src/ReactiveScopes/OutlineFunctions.ts` (inferred)
**Pipeline position:** Pass #37, Phase 8
**Current state:** No file exists yet.
**What's needed:**

Optional: extract function expressions into separate outlined functions:

- Similar to OutlineJSX but for function expressions
- Controlled by `EnvironmentConfig.enable_function_outlining`
- Can be deferred (optional optimization)

**Depends on:** InferReactiveScopeVariables
