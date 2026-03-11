# Source Map Support

> Wire up source map generation from the Rust codegen layer through NAPI to the Vite plugin,
> so that browser DevTools map compiled React code back to the original source.

---

### Gap 1: Expose Source Map from compile_program ✅

~~**Upstream:** N/A (upstream Babel plugin delegates to Babel's own source map infrastructure)~~
~~**Current state:** `codegen_function_with_source_map` in `crates/oxc_react_compiler/src/reactive_scopes/codegen.rs` already produces a `SourceMap` with VLQ encoding and JSON serialization (`to_json`). However, `compile_program` in `entrypoint/program.rs` calls `codegen_function` (without source map) and the `CompileResult` struct has no source map field.~~

**Completed**: Exposed source map from `compile_program` by adding a source map field to `CompileResult`, calling `codegen_function_with_source_map` when source maps are requested, and serializing via `SourceMap::to_json`.

### Gap 2: Pass Source Map Through NAPI Binding ✅

~~**Upstream:** N/A~~
~~**Current state:** `TransformResult` in `napi/react-compiler/src/lib.rs` has `code` and `transformed` fields but no source map. The Vite plugin returns `map: null` on line 56.~~

**Completed**: Added `source_map: Option<String>` to `TransformResult` in the NAPI binding and wired `CompileResult`'s source map through to the NAPI result.

### Gap 3: Wire Source Map in Vite Plugin ✅

~~**Upstream:** N/A~~
~~**Current state:** `vite-plugin/index.ts` line 56 returns `map: null`.~~

**Completed**: Wired the source map JSON from the NAPI binding into the Vite plugin's transform result, replacing the `map: null` return with the actual source map.

### Gap 4: Whole-File Source Map Composition ✅

~~**Upstream:** N/A~~
~~**Current state:** The codegen source map only covers individual compiled functions. The `apply_compilation` function splices compiled code into the original source, but does not produce a source map that accounts for unmodified regions.~~

**Completed**: Implemented whole-file source map composition that generates identity mappings for unmodified regions, offsets per-function source map entries to their position in the final output, and produces a single v3 source map covering the entire output file. All Priority 2 source map tasks are now complete.
