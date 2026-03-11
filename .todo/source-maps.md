# Source Map Support

> Wire up source map generation from the Rust codegen layer through NAPI to the Vite plugin,
> so that browser DevTools map compiled React code back to the original source.

---

### Gap 1: Expose Source Map from compile_program

**Upstream:** N/A (upstream Babel plugin delegates to Babel's own source map infrastructure)
**Current state:** `codegen_function_with_source_map` in `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` already produces a `SourceMap` with VLQ encoding and JSON serialization (`to_json`). However, `compile_program` in `entrypoint/program.rs` calls `codegen_function` (without source map) and the `CompileResult` struct has no source map field.
**What's needed:**
- Add an `Option<String>` source_map field to `CompileResult`
- When source maps are requested, call `codegen_function_with_source_map` instead of `codegen_function`
- Compose per-function source maps into a single source map for the whole file (the `apply_compilation` step does text splicing; source map offsets must account for the non-compiled code that passes through unchanged)
- Use `SourceMap::to_json` to serialize the final map
**Depends on:** None

### Gap 2: Pass Source Map Through NAPI Binding

**Upstream:** N/A
**Current state:** `TransformResult` in `napi/react-compiler/src/lib.rs` has `code` and `transformed` fields but no source map. The Vite plugin returns `map: null` on line 56.
**What's needed:**
- Add `pub source_map: Option<String>` to `TransformResult`
- In `transform_react_file`, pass the source map from `CompileResult` through to the NAPI result
- Update the TypeScript type declarations if there are any `.d.ts` files
**Depends on:** Gap 1

### Gap 3: Wire Source Map in Vite Plugin

**Upstream:** N/A
**Current state:** `vite-plugin/index.ts` line 56 returns `map: null`.
**What's needed:**
- Parse the source map JSON string from the NAPI result
- Return it as `map` in the Vite transform result (Vite accepts a source map object or JSON string)
- Verify that Vite correctly chains the source map with other plugins (e.g., `@vitejs/plugin-react` for JSX transform)
- Add an option to disable source maps for production builds if needed
**Depends on:** Gap 2

### Gap 4: Whole-File Source Map Composition

**Upstream:** N/A
**Current state:** The codegen source map only covers individual compiled functions. The `apply_compilation` function splices compiled code into the original source, but does not produce a source map that accounts for unmodified regions.
**What's needed:**
- For unmodified regions of the file (code between compiled functions), generate identity mappings (line N, col M maps to line N, col M)
- For compiled function regions, offset the per-function source map entries to account for their position in the final output
- The result should be a single v3 source map covering the entire output file
- Consider using `oxc_sourcemap` crate if available, or extend the existing `SourceMap` struct
**Depends on:** Gap 1
