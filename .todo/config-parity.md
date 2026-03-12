# EnvironmentConfig Parity Gaps

> Tracking gaps between our `EnvironmentConfig` / `PluginOptions` and the upstream
> `babel-plugin-react-compiler` configuration surface. The upstream did a major flag
> cleanup in Feb 2026 (PR #35825), removing 19 experimental flags. This file tracks
> only flags that exist in upstream HEAD today.

Last updated: 2026-03-12

---

## Gap 1: `validateExhaustiveEffectDependencies` should be an enum

**Upstream:** `src/Entrypoint/Pipelines.ts`, `EnvironmentConfig.ts`
**Current state:** `environment.rs` has `validate_exhaustive_effect_dependencies: bool`. The upstream accepts `'off' | 'all' | 'missing-only' | 'extra-only'`.
**What's needed:**
- Create an enum `ExhaustiveDepsMode { Off, All, MissingOnly, ExtraOnly }` in `options.rs` or `environment.rs`
- Replace the `bool` field with the new enum
- Update `validate_exhaustive_dependencies.rs` to branch on the mode (skip missing-only checks when `ExtraOnly`, etc.)
- Update config parsing in `options.rs`
**Depends on:** None

---

## Gap 2: `enableEmitHookGuards` should accept `ExternalFunction` config

**Upstream:** `src/Entrypoint/Pipelines.ts`, `EnvironmentConfig.ts`
**Current state:** `environment.rs` has `enable_emit_hook_guards: bool`. Upstream accepts an `ExternalFunction` value (function name + import source) so codegen can emit `import { guardFn } from "source"; guardFn(hookName);` wrappers.
**What's needed:**
- Define an `ExternalFunction { function_name: String, import_source: String }` struct (may already exist as `GatingConfig` in `options.rs` -- evaluate reuse)
- Change `enable_emit_hook_guards` from `bool` to `Option<ExternalFunction>`
- Update codegen to emit the guard import and wrapping calls when the option is `Some`
- Update config parsing
**Depends on:** None

---

## Gap 3: `assertValidMutableRanges` should be config-gated

**Upstream:** `src/Entrypoint/Pipelines.ts`
**Current state:** `assert_valid_mutable_ranges.rs` runs unconditionally in the pipeline. Upstream gates it behind `config.assertValidMutableRanges` (default false, for internal debugging).
**What's needed:**
- Add `assert_valid_mutable_ranges: bool` to `EnvironmentConfig` (default `false`)
- Guard the call in `pipeline.rs` behind the flag
- This is a minor correctness risk -- running it unconditionally is strictly more conservative, but it adds unnecessary overhead and can bail out on edge cases upstream would accept
**Depends on:** None

---

## Gap 4: `enableNameAnonymousFunctions` config gate missing

**Upstream:** `src/Entrypoint/Pipelines.ts`
**Current state:** `name_anonymous_functions.rs` runs unconditionally. Upstream gates it behind `config.enableNameAnonymousFunctions` (default true).
**What's needed:**
- Add `enable_name_anonymous_functions: bool` to `EnvironmentConfig` (default `true`)
- Guard the call in `pipeline.rs`
**Depends on:** None

---

## Gap 5: `OutputMode::ClientNoMemo` variant missing

**Upstream:** `src/Entrypoint/Options.ts`
**Current state:** `OutputMode` enum has `Client | SSR | Lint`. Upstream also has `client-no-memo` which strips all memoization from output (useful for benchmarking and testing the raw transform).
**What's needed:**
- Add `ClientNoMemo` variant to `OutputMode` in `options.rs`
- Update config parsing to accept `"client-no-memo"`
- In codegen, skip `useMemoCache` emission and scope wrapping when this mode is active
- Essentially: run the full pipeline but emit the "unwrapped" code
**Depends on:** None

---

## Gap 6: `validateNoImpureFunctionsInRender` validation pass missing

**Upstream:** `src/Validation/ValidateNoImpureFunctionsInRender.ts`
**Current state:** No corresponding file exists.
**What's needed:**
- Create `validation/validate_no_impure_functions_in_render.rs`
- Check for calls to functions known to be impure (console.log, Math.random, Date.now, etc.) at the top level of render
- Add `validate_no_impure_functions_in_render: bool` to `EnvironmentConfig`
- Wire into pipeline
**Depends on:** None

---

## Gap 7: `validateBlocklistedImports` validation pass missing

**Upstream:** `src/Validation/ValidateBlocklistedImports.ts`
**Current state:** No corresponding file exists.
**What's needed:**
- Create `validation/validate_blocklisted_imports.rs`
- Accept a list of blocked import sources from config
- Add `blocklisted_imports: Vec<String>` to `EnvironmentConfig`
- Scan HIR for imports matching blocked sources, emit diagnostic
- Wire into pipeline
**Depends on:** None

---

## Gap 8: `validateNoVoidUseMemo` overlap check

**Upstream:** `src/Validation/ValidateNoVoidUseMemo.ts`
**Current state:** `validate_use_memo.rs` exists but may not cover the "void useMemo" case (useMemo with no return value).
**What's needed:**
- Audit `validate_use_memo.rs` against upstream `ValidateNoVoidUseMemo.ts`
- If the void check is missing, add it (useMemo callbacks that don't return a value are likely bugs)
- Add config flag if needed
**Depends on:** None

---

## Gap 9: `enableTreatSetIdentifiersAsStateSetters` heuristic

**Upstream:** `src/HIR/Environment.ts`
**Current state:** `validate_no_set_state_in_render.rs` and `validate_no_set_state_in_effects.rs` use a crude `starts_with("set") + uppercase` heuristic. Upstream has a richer mechanism that tracks `useState` return values through destructuring to identify the actual setter.
**What's needed:**
- Add `enable_treat_set_identifiers_as_state_setters: bool` to `EnvironmentConfig` (default `true`)
- When enabled, use a flow-sensitive approach: track `useState()` call return values, mark the second destructured element as a state setter
- Fall back to naming heuristic when the flag is `false`
- This improves both `validate_no_set_state_in_render` and `validate_no_set_state_in_effects`
**Depends on:** None, but benefits from type inference improvements

---

## Gap 10: `enableAllowSetStateFromRefsInEffects` nuance

**Upstream:** `src/Validation/ValidateNoSetStateInEffects.ts`
**Current state:** `validate_no_set_state_in_effects.rs` likely does not differentiate between setState calls that read from refs vs. other sources.
**What's needed:**
- Add `enable_allow_set_state_from_refs_in_effects: bool` to `EnvironmentConfig`
- When enabled, allow `setState(ref.current)` patterns inside effects (this is a valid pattern for syncing refs to state)
- Requires tracking data flow from ref reads to setState calls
**Depends on:** Gap 9 (setState identification)

---

## Gap 11: `enableVerboseNoSetStateInEffect` richer diagnostics

**Upstream:** `src/Validation/ValidateNoSetStateInEffects.ts`
**Current state:** `validate_no_set_state_in_effects.rs` emits a generic error message.
**What's needed:**
- Add `enable_verbose_no_set_state_in_effect: bool` to `EnvironmentConfig`
- When enabled, include the specific setState identifier name, the effect it was called in, and a suggestion for how to fix (e.g., "move to useLayoutEffect" or "guard with a condition")
**Depends on:** Gap 9 (setState identification)
