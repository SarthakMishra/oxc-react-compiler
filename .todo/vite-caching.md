# Incremental Caching in Vite Plugin

> Add file-level caching to the Vite plugin so unchanged files are not recompiled on every rebuild.

**Priority:** P3 (Build When Ready) -- performance optimization, not a correctness issue. Every file is correctly compiled today; this just avoids redundant work.

---

## Current State

The Vite plugin at `napi/react-compiler/vite-plugin/index.ts` (107 lines) processes every file on every rebuild:

- `transform(code, id)` hook is called by Vite for every module in the dependency graph
- `mightContainReactCode(code)` heuristic already skips files without React patterns (functions + JSX/return)
- `isReactFile(id)` skips non-JS/TS files and `node_modules`
- No caching of previous compilation results
- `handleHotUpdate()` invalidates modules on file change, forcing re-transform
- Plugin options are passed to `binding.transformReactFile()` on every call

The NAPI binding (`napi/react-compiler/`) exposes `transformReactFile(code, filename, options)` which runs the full 62-pass pipeline on every invocation.

---

## Gap 1: In-Memory Content-Hash Cache

**Upstream:** N/A (upstream Babel plugin relies on Webpack/Vite's own caching; no built-in cache)
**Current state:** No caching whatsoever
**What's needed:**
- Add a `Map<string, CacheEntry>` to the plugin closure, keyed on file ID
- `CacheEntry` contains:
  - `contentHash: string` -- hash of the source code (use a fast hash like xxhash or Node's built-in `crypto.createHash('md5')`)
  - `optionsHash: string` -- hash of the plugin options (invalidate if config changes)
  - `result: { code: string; map: object | null }` -- cached transform output
- On `transform(code, id)`:
  1. Compute content hash
  2. If cache hit (same content hash + options hash), return cached result
  3. Otherwise, compile, cache, and return
- On `handleHotUpdate()`: evict the cache entry for the changed file (the next `transform()` call will recompute)
- On `buildStart()`: clear the entire cache if options changed since last build

**Implementation notes:**
- Use `node:crypto` for hashing (available in all Node.js versions Vite supports)
- The options hash only needs to be computed once per build (in `buildStart` or `configResolved`)
- Memory usage is bounded by the number of React files in the project (typically hundreds, not thousands)
- No need for LRU eviction -- the cache is per-build and cleared on restart

**Files involved:**
- `napi/react-compiler/vite-plugin/index.ts` (primary change)
- `napi/react-compiler/vite-plugin/options.ts` (may add `sourceMap` to options interface if not present)

**Depends on:** None

---

## Gap 2: Config Change Invalidation

**Current state:** Plugin options are read once in `configResolved` but there is no mechanism to detect config changes between builds
**What's needed:**
- Compute a stable hash of the serialized plugin options during `configResolved` or `buildStart`
- Store as `currentOptionsHash` in the plugin closure
- If `currentOptionsHash` differs from the previous build's hash, clear the entire cache
- This handles the case where a user changes `compilationMode`, `outputMode`, or `gating` options between dev server restarts (or in Vite's config HMR if applicable)

**Files involved:**
- `napi/react-compiler/vite-plugin/index.ts`

**Depends on:** Gap 1

---

## Gap 3: Optional Disk Cache for Large Projects

**Current state:** No disk persistence
**What's needed:**
- For projects with many React files, allow persisting the cache to disk between Vite restarts
- Implementation:
  - Add `cacheDir?: string` option to `ReactCompilerOptions` (default: disabled)
  - When enabled, on `buildStart`: load cache from disk (JSON or binary format)
  - On `buildEnd` or `closeBundle`: write cache to disk
  - Cache file format: `{ version: number, optionsHash: string, entries: Record<string, { contentHash, code, map }> }`
  - Version number allows invalidating the entire cache when the compiler version changes
- This is a **nice-to-have** -- the in-memory cache (Gap 1) handles the common dev workflow where the Vite server stays running

**Files involved:**
- `napi/react-compiler/vite-plugin/index.ts`
- `napi/react-compiler/vite-plugin/options.ts` (add `cacheDir` option)

**Depends on:** Gap 1, Gap 2

---

## Gap 4: Performance Measurement

**Current state:** No benchmarking of Vite plugin transform times
**What's needed:**
- Create a simple benchmark that measures:
  1. Cold start: time to compile all React files in a representative project (no cache)
  2. Warm rebuild: time to re-transform after a single file change (cache hit for all other files)
  3. Cache miss rate: percentage of files that actually need recompilation on a typical change
- Can use the existing `benchmarks/` infrastructure or a separate script
- Measure on a representative project (e.g., the e2e test app or a standalone fixture project)
- Report: "With caching, rebuild time is X% of cold start time" (expected: <5% for single-file edits)

**Files involved:**
- New benchmark script (location TBD, likely `benchmarks/scripts/vite-cache-bench.mjs` or similar)
- The e2e test app if it exists, or a synthetic project

**Depends on:** Gap 1

---

## Acceptance Criteria

1. Unchanged files are not recompiled (cache hit returns previous result)
2. Changed files are recompiled (cache miss triggers full pipeline)
3. Config changes invalidate the entire cache
4. HMR still works correctly (changed files are invalidated and recompiled)
5. No memory leaks (cache is bounded by project size)
6. Performance improvement is measurable on a representative project
