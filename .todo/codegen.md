# Codegen Gaps

> The codegen pass (`reactive_scopes/codegen.rs`, 690 lines) is functionally complete
> for generating JavaScript from ReactiveFunction trees. One gap remains.

---

## Gap 1: Source Map Generation

**Upstream:** babel-plugin-react-compiler relies on Babel's built-in source map support
**Current state:** `codegen.rs` has `SourceMap` and `SourceMapEntry` types defined (lines
643-677) but they are never populated during code generation. The `codegen_function` and
`codegen_instruction` functions don't track line/column positions.
**What's needed:**
- Track output line/column during codegen (increment on newlines, track column position)
- For each instruction, record a mapping from the generated position to the original
  source position (using `instr.loc` which is an `oxc_span::Span`)
- Return the `SourceMap` alongside the generated code
- Serialize to VLQ-encoded source map format (or integrate with `oxc_sourcemap`)
- Wire into the Vite plugin's `transform` return value (currently returns `map: null`)
**Depends on:** None (codegen is complete, this is additive)
