# Documentation

> Update README.md and add usage documentation for the Vite plugin,
> lint rules, and configuration options.

---

### Gap 1: Vite Plugin Usage Guide

**Upstream:** N/A
**Current state:** README.md has a minimal development section (cargo check/test/build). No user-facing documentation for how to use the Vite plugin in a React project.
**What's needed:**
- Installation instructions (`npm install @oxc-react/vite`)
- `vite.config.ts` example showing basic setup with `reactCompiler()`
- Explain interaction with `@vitejs/plugin-react` (ordering, JSX transform)
- Show all available options (`compilationMode`, `outputMode`, `target`, `include`, `exclude`) with descriptions
- Note that this replaces `babel-plugin-react-compiler` in the Vite pipeline
**Depends on:** None

### Gap 2: Lint Rules Documentation

**Upstream:** N/A
**Current state:** No documentation of which lint rules are available or how to enable them.
**What's needed:**
- List all Tier 1 rules with descriptions and examples of code they catch
- Explain Tier 2 rules and their requirements (full pipeline analysis)
- Show how to use lint rules via the NAPI binding (`lintReactFile`)
- Mention future oxlint integration path
**Depends on:** None

### Gap 3: Configuration Reference

**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/EntryPoint/Options.ts`
**Current state:** Options are defined in `crates/oxc_react_compiler/src/entrypoint/options.rs` and `napi/react-compiler/vite-plugin/options.ts` but not documented for users.
**What's needed:**
- Table of all configuration options with types, defaults, and descriptions
- `compilationMode`: explain `infer` vs `all` vs `syntax` vs `annotation`
- `outputMode`: explain `client` vs `ssr` vs `lint`
- `target`: explain React version targeting
- `include`/`exclude`: file filtering patterns
**Depends on:** None

### Gap 4: Known Limitations Section

**Upstream:** N/A
**Current state:** README has a warning that this is AI-generated and not production-ready, but no specific limitations listed.
**What's needed:**
- List known behavioral divergences from upstream (once conformance suite reveals them)
- Document unsupported patterns (if any)
- Note source map support status
- Explain the PoC nature and what "production-ready" would require
**Depends on:** Conformance suite results (conformance.md Gap 3)
