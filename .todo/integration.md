# Integration, NAPI, Vite Plugin, and Release

> NAPI bindings, Vite plugin, configuration parsing, and release packaging.
> Upstream: `src/Babel/BabelPlugin.ts` (replaced by NAPI+Vite)
> Rust modules: `napi/react-compiler/src/lib.rs`
> TypeScript: `napi/react-compiler/vite-plugin/`

---

### Gap 1: NAPI Bindings

**Upstream:** No direct upstream (replaces Babel plugin registration)
**Current state:** `napi/react-compiler/src/lib.rs` exists with basic scaffolding.
**What's needed:**

- `#[napi] pub fn transform_react_file(source: String, filename: String, options: Option<TransformOptions>) -> AsyncTask<TransformReactFileTask>`
- `TransformReactFileTask` implementing `napi::Task`:
  - `compute()`: parse with OXC, run `compile_program()`, return result
  - `resolve()`: convert to JS-accessible `TransformResult`
- `TransformResult` struct with `#[napi(object)]`:
  - `code: String`
  - `map: Option<String>` (source map JSON)
  - `transformed: bool`
  - `diagnostics: Vec<Diagnostic>` (warnings/errors)
- `TransformOptions` struct with `#[napi(object)]`:
  - Maps to `PluginOptions` on the Rust side
  - Nested `environment` config object
- Synchronous variant for testing: `#[napi] pub fn transform_react_file_sync(...)`
- Error handling: NAPI errors for parse failures, compiler panics caught

**Depends on:** Pipeline orchestration (pipeline.md Gap 2)

---

### Gap 2: Vite Plugin

**Upstream:** No direct upstream (replaces Babel plugin integration)
**Current state:** No TypeScript files exist yet for the Vite plugin.
**What's needed:**

Create `napi/react-compiler/vite-plugin/index.ts`:

```typescript
export function reactCompiler(options?: ReactCompilerOptions): Plugin {
    return {
        name: 'oxc-react-compiler',
        enforce: 'pre',
        transform(code, id) {
            if (!isReactFile(id)) return null;
            if (!mightContainReactCode(code)) return null;
            const result = transformReactFile(code, id, options);
            if (!result.transformed) return null;
            return { code: result.code, map: result.map };
        },
    };
}
```

- File filtering: `.tsx`, `.jsx`, `.ts`, `.js` extensions
- Quick check: skip files without component/hook patterns (regex-based fast path)
- Pass options through to NAPI
- Source map chaining with Vite's existing source maps
- HMR support: invalidate compiled modules on change
- `package.json` setup for publishing as `@oxc-react/vite` or similar

**Depends on:** Gap 1 (NAPI bindings)

---

### Gap 3: Configuration Parsing

**Upstream:** Upstream uses Zod schema validation for config
**Current state:** Nothing implemented.
**What's needed:**

- Parse user-provided configuration (JSON object from Vite plugin options) into `PluginOptions`
- Support all `EnvironmentConfig` flags with defaults matching upstream
- Validate configuration: unknown keys, invalid values
- Custom hooks configuration: `{ "useMyHook": { returnType: "ref" } }`
- Support Vite-style configuration (inline object in `vite.config.ts`)

**Depends on:** Environment types (environment.md Gap 1, Gap 2)

---

### Gap 4: Gating Support

**Upstream:** `src/Entrypoint/Options.ts` — GatingConfig
**Current state:** Nothing implemented.
**What's needed:**

Feature flag gating: wrap compiled output in a runtime check:

```javascript
if (enableReactCompiler) {
    // compiled version
} else {
    // original version
}
```

- `GatingConfig` struct: `import_source: String`, `import_specifier: String`
- Codegen must emit both branches
- Import the gating function

**Depends on:** Codegen (codegen.md Gap 1)

---

### Gap 5: Pin Upstream Commit

**Current state:** No `UPSTREAM_VERSION.md` exists.
**What's needed:**

- Create `UPSTREAM_VERSION.md` documenting the pinned React compiler commit
- Record which commit of `facebook/react` our implementation targets
- List any known divergences from upstream
- Update process: how to diff and merge upstream changes

**Depends on:** None (can be done anytime)

---

### Gap 6: Release Packaging

**Current state:** Nothing implemented.
**What's needed:**

- CI/CD pipeline for building platform-specific NAPI binaries
  - Linux x64, Linux ARM64, macOS x64, macOS ARM64, Windows x64
- npm package publishing workflow
- Pre-built binary distribution (like `@oxc-angular/napi-*`)
- Version management
- Changelog generation

**Depends on:** Gap 1, Gap 2 (functional NAPI and Vite plugin)
