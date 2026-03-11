# Environment, Config, and Shape System

> Configuration, environment state, object shapes, and globals.
> Upstream: `src/HIR/Environment.ts`, `src/HIR/ObjectShape.ts`, `src/HIR/Globals.ts`, `src/Entrypoint/Options.ts`

---

### Gap 1: Environment Struct

**Upstream:** `src/HIR/Environment.ts`
**Current state:** `hir/environment.rs` is a stub.
**What's needed:**

- `Environment` struct holding:
  - `config: EnvironmentConfig` — all ~50 configuration flags
  - `shape_registry: ShapeRegistry` — type shapes for built-in and custom types
  - `next_identifier_id: Counter` — unique ID generation
  - `next_block_id: Counter`
  - `next_instruction_id: Counter`
  - `next_scope_id: Counter`
  - `type_table: TypeTable` — maps `TypeId` to `Type` for efficient type operations
  - `custom_hooks: FxHashMap<String, CustomHookConfig>`
- `EnvironmentConfig` struct with all flags from REQUIREMENTS.md Section 13:
  - Memoization flags: `enable_preserve_existing_memoization_guarantees`, `validate_preserve_existing_memoization_guarantees`
  - Outlining flags: `enable_function_outlining`, `enable_jsx_outlining`
  - Validation toggles: `validate_hooks_usage`, `validate_ref_access_during_render`, `validate_no_set_state_in_render`, `validate_no_set_state_in_effects`, `validate_no_derived_computations_in_effects`, `validate_no_jsx_in_try_statements`, `validate_no_capitalized_calls`, `validate_exhaustive_memo_dependencies`, `validate_exhaustive_effect_dependencies`
  - Analysis flags: `enable_assume_hooks_follow_rules_of_react`, `enable_transitively_freeze_function_expressions`, `enable_optional_dependencies`, `enable_treat_ref_like_identifiers_as_refs`
  - Dev/HMR: `enable_reset_cache_on_source_file_changes`, `enable_emit_hook_guards`
  - Extensibility: `custom_macros`
- `CustomHookConfig` struct: return type info, effect behavior
- Default implementations for `EnvironmentConfig` matching upstream defaults

**Depends on:** None (but will be used by all HIR types)

---

### Gap 2: PluginOptions and CompilationMode

**Upstream:** `src/Entrypoint/Options.ts`
**Current state:** `entrypoint/options.rs` is a stub.
**What's needed:**

- `PluginOptions` struct:
  - `compilation_mode: CompilationMode`
  - `output_mode: OutputMode`
  - `target: ReactTarget`
  - `gating: Option<GatingConfig>`
  - `panic_threshold: PanicThreshold`
  - `sources: Option<SourceFilter>`
- `CompilationMode` enum: `Infer`, `Syntax`, `Annotation`, `All`
- `OutputMode` enum: `Client`, `SSR`, `Lint`
- `ReactTarget` enum: `React17`, `React18`, `React19`
- `PanicThreshold` enum: `AllErrors`, `CriticalErrors`, `None`
- `GatingConfig` struct: import source, function name for wrapping
- `SourceFilter` struct: include/exclude patterns

**Depends on:** None

---

### Gap 3: ObjectShape, ShapeRegistry, FunctionSignature

**Upstream:** `src/HIR/ObjectShape.ts`
**Current state:** `hir/object_shape.rs` is a stub.
**What's needed:**

- `ObjectShape` struct: describes the "shape" of a type (what properties it has, what methods return)
  - `properties: FxHashMap<String, PropertyShape>`
  - `call_signature: Option<FunctionSignature>` — if the shape is callable
  - `construct_signature: Option<FunctionSignature>` — if the shape is newable
- `PropertyShape` struct:
  - `value_shape: ShapeId`
  - `writable: bool`
- `FunctionSignature` struct:
  - `params: Vec<ParamEffect>` — effect on each parameter
  - `return_type: ShapeId`
  - `call_kind: CallKind` (Hook, Impure, etc.)
  - `no_alias: bool` — arguments are not aliased
- `ParamEffect` enum: `Read`, `Mutate`, `Freeze`, `Capture`, etc.
- `CallKind` enum: `Normal`, `Hook`, `Impure`, `Pure`
- `ShapeRegistry` struct:
  - `shapes: Vec<ObjectShape>` (indexed by `ShapeId`)
  - Methods: `register_shape()`, `get_shape()`, `get_property_shape()`
- `ShapeId(u32)` newtype

**Depends on:** None

---

### Gap 4: Globals Registry

**Upstream:** `src/HIR/Globals.ts`
**Current state:** `hir/globals.rs` is a stub.
**What's needed:**

The globals registry defines shapes for all built-in JavaScript objects and React APIs. This is critical for the effect system to know how built-in calls behave.

- Built-in object shapes: `Array`, `Object`, `Map`, `Set`, `String`, `Number`, `Math`, `JSON`, `Promise`, `RegExp`, `Date`, `console`, `Symbol`
  - For each: property shapes, method signatures (e.g., `Array.prototype.push` mutates receiver, `Array.prototype.map` reads receiver and captures callback)
- React API shapes:
  - `useState` — returns `[state, setter]`, setter is stable
  - `useReducer` — returns `[state, dispatch]`, dispatch is stable
  - `useRef` — returns mutable ref object
  - `useEffect`, `useLayoutEffect`, `useInsertionEffect` — effect hooks, capture callback
  - `useMemo`, `useCallback` — memoization hooks
  - `useContext` — reads context
  - `useTransition`, `useDeferredValue`, `useId`, `useSyncExternalStore`
  - `React.createElement`, `React.cloneElement`, `React.Children.*`
  - `forwardRef`, `memo`, `lazy`, `createContext`
- DOM globals: `document`, `window`, `navigator`
- `register_globals(registry: &mut ShapeRegistry)` function that populates all built-ins
- Must match upstream `Globals.ts` function signatures precisely for correct effect inference

**Depends on:** Gap 3 (ObjectShape, ShapeRegistry)

---

### Gap 5: CompilerError Expansion

**Upstream:** `src/CompilerError.ts`
**Current state:** `error.rs` has basic `CompilerError` with category, span, message.
**What's needed:**

- Error severity levels: `InvalidReact`, `InvalidJS`, `Todo`, `InvariantViolation` (already have category, but need severity mapping)
- `CompilerErrorDetailOptions` struct for rich error context
- Error accumulator pattern: `ErrorCollector` that gathers errors during a pass and decides whether to bail
- `PanicThreshold` integration: bail on critical errors vs collect all
- `todo!()` wrapper that creates `Todo` category errors (for unimplemented features during incremental port)
- Multiple diagnostic support: one `CompilerError` can produce multiple `OxcDiagnostic` labels
- Error codes matching upstream error categories for lint mode

**Depends on:** None
