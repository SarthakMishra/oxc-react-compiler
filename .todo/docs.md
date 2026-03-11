# Documentation

> Update README.md and add usage documentation for the Vite plugin,
> lint rules, and configuration options.

---

### Gap 1: Vite Plugin Usage Guide ✅

~~**Upstream:** N/A~~
~~**Current state:** README.md has a minimal development section. No user-facing documentation for how to use the Vite plugin in a React project.~~

**Completed**: Added Vite plugin usage guide section to `README.md` covering installation, `vite.config.ts` setup, plugin ordering with `@vitejs/plugin-react`, and all available options.

### Gap 2: Lint Rules Documentation ✅

~~**Upstream:** N/A~~
~~**Current state:** No documentation of which lint rules are available or how to enable them.~~

**Completed**: Added lint rules documentation section to `README.md` covering all Tier 1 and Tier 2 rules with descriptions, usage via NAPI binding, and future oxlint integration.

### Gap 3: Configuration Reference ✅

~~**Upstream:** `compiler/packages/babel-plugin-react-compiler/src/EntryPoint/Options.ts`~~
~~**Current state:** Options are defined in source but not documented for users.~~

**Completed**: Added configuration reference section to `README.md` with a table of all options (`compilationMode`, `outputMode`, `target`, `include`, `exclude`) including types, defaults, and descriptions.

### Gap 4: Known Limitations Section ✅

~~**Upstream:** N/A~~
~~**Current state:** README has a warning that this is AI-generated and not production-ready, but no specific limitations listed.~~

**Completed**: Added known limitations section to `README.md` covering behavioral divergences, unsupported patterns, source map status, and what production-readiness would require.
