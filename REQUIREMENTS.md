# OXC React Compiler — Requirements & Architecture

> Native OXC port of [babel-plugin-react-compiler](https://github.com/facebook/react/tree/main/compiler/packages/babel-plugin-react-compiler) for the Rolldown/Vite pipeline, plus React 19 compiler-based lint rules for oxlint.

---

## Table of Contents

1. [Project Goals](#1-project-goals)
2. [Reference Implementations](#2-reference-implementations)
3. [Architecture Overview](#3-architecture-overview)
4. [Crate Structure](#4-crate-structure)
5. [Babel Compiler Analysis — What to Port](#5-babel-compiler-analysis--what-to-port)
6. [HIR & Core Data Structures](#6-hir--core-data-structures)
7. [Compilation Pipeline (62 Passes)](#7-compilation-pipeline-62-passes)
8. [Mutation/Aliasing Effect System](#8-mutationaliasing-effect-system)
9. [Reactive Scope & Memoization Strategy](#9-reactive-scope--memoization-strategy)
10. [Code Generation](#10-code-generation)
11. [Vite/Rolldown Plugin Integration](#11-viterolldown-plugin-integration)
12. [Oxlint Rules](#12-oxlint-rules)
13. [Configuration & Options](#13-configuration--options)
14. [Upstream Merge Strategy](#14-upstream-merge-strategy)
15. [Testing Strategy](#15-testing-strategy)
16. [Implementation Phases](#16-implementation-phases)

---

## 1. Project Goals

1. **Native OXC compiler plugin** — A Rust implementation of the React Compiler that operates on OXC's AST, replacing the Babel-based `babel-plugin-react-compiler`.
2. **Rolldown/Vite integration** — Ship as a Vite plugin (`@oxc-react/vite` or similar) using NAPI-RS bindings, following the pattern established by `@oxc-angular/vite`.
3. **Oxlint rules** — Implement the React 19 compiler-based lint rules natively in oxlint, replacing `eslint-plugin-react-compiler`.
4. **Performance** — Leverage Rust's performance for the 62-pass compilation pipeline; the compiler is CPU-intensive (abstract interpretation, fixpoint iteration, disjoint set operations).
5. **Upstream compatibility** — Maintain a clear mapping to the upstream TypeScript source so that upstream changes can be merged incrementally.

---

## 2. Reference Implementations

| Repository | Role |
|---|---|
| [facebook/react/compiler/packages/babel-plugin-react-compiler](https://github.com/facebook/react/tree/main/compiler/packages/babel-plugin-react-compiler) | Upstream compiler — all core logic, HIR, passes |
| [facebook/react/compiler/packages/eslint-plugin-react-compiler](https://github.com/facebook/react/tree/main/compiler/packages/eslint-plugin-react-compiler) | Upstream lint rules — runs compiler in `outputMode: 'lint'` |
| [facebook/react/compiler/packages/react-compiler-runtime](https://github.com/facebook/react/tree/main/compiler/packages/react-compiler-runtime) | Runtime (`useMemoCache`) — no port needed, consumed at runtime |
| [voidzero-dev/oxc-angular-compiler](https://github.com/voidzero-dev/oxc-angular-compiler) | Template for crate structure, NAPI bindings, Vite plugin pattern |

---

## 3. Architecture Overview

### Upstream Babel Compiler Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Babel-Specific Layer (thin)                   │
│  BabelPlugin.ts → Program.ts → BuildHIR.ts → CodegenReactive.ts│
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                 Compiler Core (Babel-agnostic)                  │
│  SSA → ConstProp → TypeInfer → MutationAnalysis → Reactivity   │
│  → ScopeInference → ScopeAlignment → ReactiveFunction → Prune  │
│  50+ passes operating purely on HIR/ReactiveFunction IR         │
└─────────────────────────────────────────────────────────────────┘
```

**Key insight**: The upstream compiler has a clean separation between:
- **Babel-specific layer** (~5 files): AST reading (BuildHIR), AST writing (Codegen), plugin registration, import management
- **Compiler core** (~55 files): All analysis, optimization, inference, and validation passes operate on internal HIR/ReactiveFunction data structures with **zero Babel dependency**

This means the core compiler logic can be ported 1:1 to Rust. Only the thin input/output layers need OXC-specific implementations.

### Proposed OXC Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                  OXC-Specific Layer (thin)                     │
│  VitePlugin (TS) → NAPI → OXC Parser → BuildHIR → CodegenOXC │
└──────────────────────────────┬────────────────────────────────┘
                               │
┌──────────────────────────────▼────────────────────────────────┐
│              Compiler Core (ported from TS → Rust)            │
│  Same 50+ passes, same algorithms, same data structures       │
│  HIR types, Effect system, ReactiveScope, DisjointSet, etc.   │
└───────────────────────────────────────────────────────────────┘
```

---

## 4. Crate Structure

Following the `oxc-angular-compiler` pattern:

```
oxc-react-compiler/
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── oxc_react_compiler/             # Core compiler crate
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── hir/                    # HIR types & CFG
│   │   │   │   ├── mod.rs
│   │   │   │   ├── types.rs            # Place, Identifier, InstructionValue, Terminal
│   │   │   │   ├── environment.rs      # Environment config & state
│   │   │   │   ├── object_shape.rs     # ShapeRegistry, FunctionSignature
│   │   │   │   ├── globals.rs          # Built-in shapes (Array, hooks, etc.)
│   │   │   │   └── build.rs            # OXC AST → HIR lowering (replaces BuildHIR.ts)
│   │   │   ├── ssa/                    # SSA conversion
│   │   │   │   ├── enter_ssa.rs
│   │   │   │   └── eliminate_redundant_phi.rs
│   │   │   ├── optimization/           # Optimization passes
│   │   │   │   ├── constant_propagation.rs
│   │   │   │   ├── dead_code_elimination.rs
│   │   │   │   ├── inline_iife.rs
│   │   │   │   └── ...
│   │   │   ├── inference/              # Type & mutation inference
│   │   │   │   ├── infer_types.rs
│   │   │   │   ├── infer_mutation_aliasing_effects.rs
│   │   │   │   ├── infer_mutation_aliasing_ranges.rs
│   │   │   │   ├── infer_reactive_places.rs
│   │   │   │   └── aliasing_effects.rs # AliasingEffect enum & composition rules
│   │   │   ├── reactive_scopes/        # Scope inference, alignment, codegen
│   │   │   │   ├── infer_reactive_scope_variables.rs
│   │   │   │   ├── align_scopes.rs
│   │   │   │   ├── merge_scopes.rs
│   │   │   │   ├── propagate_dependencies.rs
│   │   │   │   ├── build_reactive_function.rs
│   │   │   │   ├── prune_scopes.rs
│   │   │   │   └── codegen.rs          # ReactiveFunction → OXC AST (replaces CodegenReactiveFunction.ts)
│   │   │   ├── validation/             # Validation passes
│   │   │   │   ├── validate_hooks_usage.rs
│   │   │   │   ├── validate_no_ref_access_in_render.rs
│   │   │   │   ├── validate_no_set_state_in_render.rs
│   │   │   │   └── ... (one file per validation)
│   │   │   ├── entrypoint/             # Top-level orchestration
│   │   │   │   ├── pipeline.rs         # Pass pipeline ordering
│   │   │   │   ├── program.rs          # Function discovery & compilation loop
│   │   │   │   └── options.rs          # PluginOptions, EnvironmentConfig
│   │   │   ├── utils/                  # Shared utilities
│   │   │   │   ├── disjoint_set.rs
│   │   │   │   └── ordered_map.rs
│   │   │   └── error.rs               # CompilerError, ErrorCategory, diagnostics
│   │   ├── tests/
│   │   └── Cargo.toml
│   │
│   └── oxc_react_compiler_lint/        # Oxlint rule implementations
│       ├── src/
│       │   ├── lib.rs
│       │   ├── rules/
│       │   │   ├── no_ref_access_in_render.rs
│       │   │   ├── no_set_state_in_render.rs
│       │   │   ├── no_jsx_in_try.rs
│       │   │   ├── rules_of_hooks.rs
│       │   │   ├── no_set_state_in_effects.rs
│       │   │   ├── static_components.rs
│       │   │   ├── use_memo_validation.rs
│       │   │   ├── purity.rs
│       │   │   └── ...
│       │   └── utils/
│       │       └── hook_detection.rs   # Shared hook identification utilities
│       ├── tests/
│       └── Cargo.toml
│
├── napi/
│   └── react-compiler/                 # NAPI-RS Node.js bindings
│       ├── src/lib.rs                  # #[napi] function definitions
│       ├── build.rs
│       ├── build.ts
│       ├── vite-plugin/                # Vite/Rolldown plugin (TypeScript)
│       │   ├── index.ts                # Main plugin: reactCompiler()
│       │   └── options.ts
│       ├── package.json                # Published as @oxc-react/vite
│       ├── Cargo.toml
│       └── test/
│
├── justfile
├── pnpm-workspace.yaml
└── package.json
```

### Key Dependencies

| Crate | Purpose |
|---|---|
| `oxc_allocator` | Arena allocator for AST nodes |
| `oxc_ast` | OXC AST types (Expression, Statement, Program) |
| `oxc_parser` | TypeScript/JavaScript parser |
| `oxc_semantic` | Semantic analysis (scopes, symbols, references) |
| `oxc_span` | Source spans, Atom, SourceType |
| `oxc_diagnostics` | Error/warning reporting |
| `oxc_codegen` | AST → JavaScript string (for output) |
| `rustc-hash` | Fast hashing for internal maps |
| `indexmap` | Ordered maps (block ordering in CFG) |
| `napi` / `napi-derive` | NAPI-RS bindings |
| `mimalloc-safe` | Global allocator for NAPI crate |

---

## 5. Babel Compiler Analysis — What to Port

### Files that need OXC-specific reimplementation (5 files)

These are the Babel-coupled boundaries. They must be rewritten for OXC:

| Upstream File | Purpose | OXC Replacement |
|---|---|---|
| `src/Babel/BabelPlugin.ts` | Babel plugin registration | NAPI entry + Vite plugin |
| `src/Entrypoint/Program.ts` | Babel AST traversal to find functions, AST mutation | OXC AST visitor for function discovery |
| `src/HIR/BuildHIR.ts` | Babel `NodePath` → HIR lowering | `oxc_ast` → HIR lowering |
| `src/ReactiveScopes/CodegenReactiveFunction.ts` | ReactiveFunction → `@babel/types` AST | ReactiveFunction → `oxc_ast` / `oxc_codegen` |
| `src/Entrypoint/Imports.ts` | Babel import manipulation | OXC import insertion |

### Files that port 1:1 to Rust (~55 files)

The entire compiler core operates on internal IR with zero Babel dependency. These files map directly to Rust modules:

**SSA (2 files)**:
`EnterSSA.ts`, `EliminateRedundantPhi.ts`

**Optimization (5 files)**:
`ConstantPropagation.ts`, `DeadCodeElimination.ts`, `InlineImmediatelyInvokedFunctionExpressions.ts`, `MergeConsecutiveBlocks.ts`, `OptimizePropsMethodCalls.ts`

**Inference (6 files)**:
`InferTypes.ts`, `InferMutationAliasingEffects.ts`, `InferMutationAliasingRanges.ts`, `InferReactivePlaces.ts`, `InferReactiveScopeVariables.ts`, `AliasingEffects.ts`

**Reactive Scopes (15+ files)**:
`AlignMethodCallScopes.ts`, `AlignObjectMethodScopes.ts`, `AlignReactiveScopesToBlockScopesHIR.ts`, `BuildReactiveScopeTerminalsHIR.ts`, `BuildReactiveFunction.ts`, `FlattenReactiveLoopsHIR.ts`, `FlattenScopesWithHooksOrUseHIR.ts`, `MergeOverlappingReactiveScopesHIR.ts`, `MergeReactiveScopesThatInvalidateTogether.ts`, `PropagateScopeDependenciesHIR.ts`, `PruneAlwaysInvalidatingScopes.ts`, `PruneNonEscapingScopes.ts`, `PruneNonReactiveDependencies.ts`, `PruneUnusedScopes.ts`, `PropagateEarlyReturns.ts`, etc.

**Validation (14 files)**:
`ValidateHooksUsage.ts`, `ValidateNoRefAccessInRender.ts`, `ValidateNoSetStateInRender.ts`, `ValidateNoSetStateInEffects.ts`, `ValidateNoDerivedComputationsInEffects.ts`, `ValidateNoJSXInTryStatement.ts`, `ValidateNoCapitalizedCalls.ts`, `ValidateNoFreezingKnownMutableFunctions.ts`, `ValidateExhaustiveDependencies.ts`, `ValidatePreservedManualMemoization.ts`, `ValidateContextVariableLValues.ts`, `ValidateUseMemo.ts`, `ValidateLocalsNotReassignedAfterRender.ts`, `ValidateStaticComponents.ts`

**Core types & utilities (8+ files)**:
`HIR.ts` (types), `Environment.ts`, `ObjectShape.ts`, `Globals.ts`, `Types.ts`, `CompilerError.ts`, `DisjointSet.ts`, `Pipeline.ts`

---

## 6. HIR & Core Data Structures

These are the Rust types needed for the compiler's internal representation. The upstream TypeScript types map cleanly to Rust enums and structs.

### ID Newtypes

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopeId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdentifierId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeclarationId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstructionId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(pub u32);
```

### HIR (Control-Flow Graph)

```rust
pub struct HIRFunction {
    pub loc: SourceLocation,
    pub id: Option<String>,
    pub fn_type: ReactFunctionType,  // Component | Hook | Other
    pub env: Environment,
    pub params: Vec<Param>,          // Place | SpreadPattern
    pub returns: Place,
    pub context: Vec<Place>,         // captured variables
    pub body: HIR,
    pub is_async: bool,
    pub is_generator: bool,
    pub directives: Vec<String>,
}

pub struct HIR {
    pub entry: BlockId,
    pub blocks: IndexMap<BlockId, BasicBlock>,  // reverse-postorder
}

pub struct BasicBlock {
    pub kind: BlockKind,  // Block | Value | Loop | Sequence | Catch
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminal: Terminal,
    pub preds: FxHashSet<BlockId>,
    pub phis: FxHashSet<Phi>,
}

pub struct Instruction {
    pub id: InstructionId,
    pub lvalue: Place,
    pub value: InstructionValue,
    pub loc: SourceLocation,
    pub effects: Option<Vec<AliasingEffect>>,
}
```

### Place & Identifier

```rust
pub struct Place {
    pub identifier: Identifier,
    pub effect: Effect,
    pub reactive: bool,
    pub loc: SourceLocation,
}

pub struct Identifier {
    pub id: IdentifierId,
    pub declaration_id: DeclarationId,
    pub name: Option<String>,
    pub mutable_range: MutableRange,
    pub scope: Option<Box<ReactiveScope>>,
    pub type_: Type,
    pub loc: SourceLocation,
}

pub struct MutableRange {
    pub start: InstructionId,
    pub end: InstructionId,
}
```

### InstructionValue (~40 variants)

```rust
pub enum InstructionValue {
    // Locals & context
    LoadLocal { place: Place },
    StoreLocal { lvalue: Place, value: Place, type_: Option<InstructionKind> },
    LoadContext { place: Place },
    StoreContext { lvalue: Place, value: Place },
    DeclareLocal { lvalue: Place, type_: InstructionKind },
    DeclareContext { lvalue: Place },
    Destructure { lvalue_pattern: DestructurePattern, value: Place },

    // Literals
    Primitive { value: Primitive },
    JSXText { value: String },
    RegExpLiteral { pattern: String, flags: String },
    TemplateLiteral { quasis: Vec<String>, subexpressions: Vec<Place> },

    // Operators
    BinaryExpression { op: BinaryOp, left: Place, right: Place },
    UnaryExpression { op: UnaryOp, value: Place },
    PrefixUpdate { op: UpdateOp, lvalue: Place },
    PostfixUpdate { op: UpdateOp, lvalue: Place },

    // Calls
    CallExpression { callee: Place, args: Vec<Place> },
    MethodCall { receiver: Place, property: String, args: Vec<Place> },
    NewExpression { callee: Place, args: Vec<Place> },

    // Property access
    PropertyLoad { object: Place, property: String },
    PropertyStore { object: Place, property: String, value: Place },
    ComputedLoad { object: Place, property: Place },
    ComputedStore { object: Place, property: Place, value: Place },
    PropertyDelete { object: Place, property: String },
    ComputedDelete { object: Place, property: Place },

    // Containers
    ObjectExpression { properties: Vec<ObjectProperty> },
    ArrayExpression { elements: Vec<ArrayElement> },

    // JSX
    JsxExpression { tag: Place, props: Vec<JsxAttribute>, children: Vec<Place> },
    JsxFragment { children: Vec<Place> },

    // Functions
    FunctionExpression { name: Option<String>, lowered_func: Box<HIRFunction>, expr_type: FunctionExprType },
    ObjectMethod { lowered_func: Box<HIRFunction> },

    // Globals
    LoadGlobal { binding: GlobalBinding },
    StoreGlobal { name: String, value: Place },

    // Async/Iterator
    Await { value: Place },
    GetIterator { collection: Place },
    IteratorNext { iterator: Place, loc: SourceLocation },
    NextPropertyOf { value: Place },

    // Type
    TypeCastExpression { value: Place, type_: String },
    TaggedTemplateExpression { tag: Place, value: TemplateLiteralData },

    // Manual memoization markers
    StartMemoize { manualMemoId: u32 },
    FinishMemoize { manualMemoId: u32, decl: Place, deps: Vec<Place>, pruned: bool },

    // Catch-all
    UnsupportedNode { node: String },
}
```

### Terminal (~20 variants)

```rust
pub enum Terminal {
    Goto { block: BlockId },
    If { test: Place, consequent: BlockId, alternate: BlockId, fallthrough: BlockId },
    Branch { test: Place, consequent: BlockId, alternate: BlockId },
    Switch { test: Place, cases: Vec<SwitchCase>, fallthrough: BlockId },
    Return { value: Place },
    Throw { value: Place },
    For { init: BlockId, test: BlockId, update: Option<BlockId>, body: BlockId, fallthrough: BlockId },
    ForOf { init: BlockId, test: BlockId, body: BlockId, fallthrough: BlockId },
    ForIn { init: BlockId, test: BlockId, body: BlockId, fallthrough: BlockId },
    DoWhile { body: BlockId, test: BlockId, fallthrough: BlockId },
    While { test: BlockId, body: BlockId, fallthrough: BlockId },
    Logical { operator: LogicalOp, left: BlockId, right: BlockId, fallthrough: BlockId },
    Ternary { test: Place, consequent: BlockId, alternate: BlockId, fallthrough: BlockId },
    Optional { test: Place, consequent: BlockId, fallthrough: BlockId },
    Sequence { blocks: Vec<BlockId>, fallthrough: BlockId },
    Label { block: BlockId, fallthrough: BlockId, label: u32 },
    MaybeThrow { continuation: BlockId, handler: BlockId },
    Try { block: BlockId, handler: BlockId, fallthrough: BlockId },
    Scope { scope: ScopeId, block: BlockId, fallthrough: BlockId },
    PrunedScope { scope: ScopeId, block: BlockId, fallthrough: BlockId },
    Unreachable,
}
```

### Effect & ValueKind

```rust
pub enum Effect {
    Unknown,
    Freeze,
    Read,
    Capture,
    ConditionallyMutateIterator,
    ConditionallyMutate,
    Mutate,
    Store,
}

pub enum ValueKind {
    MaybeFrozen,
    Frozen,
    Primitive,
    Global,
    Mutable,
    Context,
}
```

---

## 7. Compilation Pipeline (62 Passes)

The passes execute in this exact order. Each pass maps to a Rust function operating on `&mut HIR` or `&mut ReactiveFunction`.

### Phase 1: HIR Construction & Early Cleanup
| # | Pass | Input → Output |
|---|---|---|
| 1 | `lower` | OXC AST → HIR CFG |
| 2 | `prune_maybe_throws` | HIR → HIR |
| 3 | `validate_context_variable_lvalues` | HIR (validation) |
| 4 | `validate_use_memo` | HIR (validation) |
| 5 | `drop_manual_memoization` | HIR → HIR (conditional) |
| 6 | `inline_iife` | HIR → HIR |
| 7 | `merge_consecutive_blocks` | HIR → HIR |

### Phase 2: SSA
| # | Pass | Input → Output |
|---|---|---|
| 8 | `enter_ssa` | HIR → SSA HIR (phi nodes) |
| 9 | `eliminate_redundant_phi` | SSA HIR → SSA HIR |

### Phase 3: Optimization & Type Inference
| # | Pass | Input → Output |
|---|---|---|
| 10 | `constant_propagation` | SSA HIR → SSA HIR |
| 11 | `infer_types` | SSA HIR (annotates types) |

### Phase 4: Validation (Hooks)
| # | Pass | Notes |
|---|---|---|
| 12 | `validate_hooks_usage` | Conditional on config |
| 13 | `validate_no_capitalized_calls` | Conditional on config |

### Phase 5: Mutation/Aliasing Analysis (CORE)
| # | Pass | Input → Output |
|---|---|---|
| 14 | `optimize_props_method_calls` | HIR → HIR |
| 15 | `analyse_functions` | Recursively analyze nested functions |
| 16 | `infer_mutation_aliasing_effects` | HIR → HIR (annotates effects) |
| 17 | `optimize_for_ssr` | HIR → HIR (SSR only) |
| 18 | `dead_code_elimination` | HIR → HIR |
| 19 | `prune_maybe_throws` | HIR → HIR (2nd pass) |
| 20 | `infer_mutation_aliasing_ranges` | HIR → HIR (annotates MutableRange) |

### Phase 6: Validation Battery
| # | Pass | Notes |
|---|---|---|
| 21 | `validate_locals_not_reassigned_after_render` | Always |
| 22 | `assert_valid_mutable_ranges` | Optional |
| 23 | `validate_no_ref_access_in_render` | Configurable |
| 24 | `validate_no_set_state_in_render` | Configurable |
| 25 | `validate_no_derived_computations_in_effects` | Configurable |
| 26 | `validate_no_set_state_in_effects` | Lint mode only |
| 27 | `validate_no_jsx_in_try_statement` | Lint mode only |
| 28 | `validate_no_freezing_known_mutable_functions` | Always |

### Phase 7: Reactivity Inference
| # | Pass | Input → Output |
|---|---|---|
| 29 | `infer_reactive_places` | HIR → HIR (marks reactive) |
| 30 | `validate_exhaustive_dependencies` | Configurable |
| 31 | `rewrite_instruction_kinds_based_on_reassignment` | HIR → HIR |
| 32 | `validate_static_components` | Lint mode only |

### Phase 8: Reactive Scope Construction
| # | Pass | Input → Output |
|---|---|---|
| 33 | `infer_reactive_scope_variables` | HIR → HIR (creates scopes) |
| 34 | `memoize_fbt_and_macro_operands_in_same_scope` | HIR → HIR |
| 35 | `outline_jsx` | Optional |
| 36 | `name_anonymous_functions` | Optional |
| 37 | `outline_functions` | Optional |
| 38 | `align_method_call_scopes` | HIR → HIR |
| 39 | `align_object_method_scopes` | HIR → HIR |
| 40 | `prune_unused_labels_hir` | HIR → HIR |
| 41 | `align_reactive_scopes_to_block_scopes_hir` | HIR → HIR |
| 42 | `merge_overlapping_reactive_scopes_hir` | HIR → HIR |
| 43 | `build_reactive_scope_terminals_hir` | HIR → HIR (scope terminals) |
| 44 | `flatten_reactive_loops_hir` | HIR → HIR |
| 45 | `flatten_scopes_with_hooks_or_use_hir` | HIR → HIR |
| 46 | `propagate_scope_dependencies_hir` | HIR → HIR |

### Phase 9: HIR → ReactiveFunction
| # | Pass | Input → Output |
|---|---|---|
| 47 | `build_reactive_function` | HIR CFG → ReactiveFunction tree |

### Phase 10: ReactiveFunction Optimization
| # | Pass | Input → Output |
|---|---|---|
| 48 | `prune_unused_labels` | RF → RF |
| 49 | `prune_non_escaping_scopes` | RF → RF |
| 50 | `prune_non_reactive_dependencies` | RF → RF |
| 51 | `prune_unused_scopes` | RF → RF |
| 52 | `merge_reactive_scopes_that_invalidate_together` | RF → RF |
| 53 | `prune_always_invalidating_scopes` | RF → RF |
| 54 | `propagate_early_returns` | RF → RF |
| 55 | `prune_unused_lvalues` | RF → RF |
| 56 | `promote_used_temporaries` | RF → RF |
| 57 | `extract_scope_declarations_from_destructuring` | RF → RF |
| 58 | `stabilize_block_ids` | RF → RF |
| 59 | `rename_variables` | RF → RF |
| 60 | `prune_hoisted_contexts` | RF → RF |

### Phase 11: Validation & Code Generation
| # | Pass | Input → Output |
|---|---|---|
| 61 | `validate_preserved_manual_memoization` | RF (validation, conditional) |
| 62 | `codegen_function` | ReactiveFunction → OXC AST |

---

## 8. Mutation/Aliasing Effect System

The effect system is the **core algorithmic engine** of the compiler. It determines which values are mutable, frozen, aliased, and captured — driving all memoization decisions.

### AliasingEffect Enum (17 variants)

```rust
pub enum AliasingEffect {
    // Creation
    Create { into: Place, value: ValueKind, reason: ValueReason },
    CreateFrom { from: Place, into: Place },
    CreateFunction { captures: Vec<Place>, function: Place, into: Place },
    Apply { receiver: Place, function: Place, args: Vec<Place>, into: Place, signature: Option<FunctionSignature> },

    // Data flow
    Assign { from: Place, into: Place },
    Alias { from: Place, into: Place },
    MaybeAlias { from: Place, into: Place },
    Capture { from: Place, into: Place },
    ImmutableCapture { from: Place, into: Place },

    // Mutation
    Mutate { value: Place },
    MutateConditionally { value: Place },
    MutateTransitive { value: Place },
    MutateTransitiveConditionally { value: Place },
    Freeze { value: Place, reason: FreezeReason },

    // Errors (always reported)
    MutateFrozen { place: Place, error: String },
    MutateGlobal { place: Place, error: String },
    Impure { place: Place, error: String },
    Render { place: Place },
}
```

### Transitivity Rules

Effects compose along data-flow edges:
- **Assign/Alias/CreateFrom**: Direct edges — local mutation flows through
- **Capture**: Indirect edge — local mutation does NOT flow, transitive mutation DOES
- **MaybeAlias**: Downgrades mutation to conditional
- **Freeze**: Freezes the reference, not the underlying value

### Abstract Interpretation (`infer_mutation_aliasing_effects`)

This is the most computationally intensive pass:
1. For each instruction, compute candidate effects based on instruction kind and operand types
2. Build an abstract heap model (pointer graph with `ValueKind` per abstract value)
3. Fixpoint iteration: propagate effects through the pointer graph until stable
4. Record final effects on each instruction

### Mutable Range Computation (`infer_mutation_aliasing_ranges`)

Uses effects to compute `MutableRange` for each identifier:
- `start`: instruction that creates the value
- `end`: last instruction that mutates the value (transitively through aliases)
- Values with overlapping mutable ranges that share at least one reactive operand are grouped into the same `ReactiveScope`

---

## 9. Reactive Scope & Memoization Strategy

### Scope Inference Algorithm

Uses a `DisjointSet<IdentifierId>` (union-find):
1. For each instruction, if the lvalue has a mutable range > 1 or the instruction allocates:
   - Union the lvalue with all mutable operands
   - If any is reactive, the whole set becomes a reactive scope
2. For phi nodes whose values are mutated after creation, union all operands
3. Each disjoint set becomes a `ReactiveScope` with a merged `MutableRange`

### ReactiveScope Structure

```rust
pub struct ReactiveScope {
    pub id: ScopeId,
    pub range: MutableRange,
    pub dependencies: FxHashSet<ReactiveScopeDependency>,
    pub declarations: FxHashMap<IdentifierId, ReactiveScopeDeclaration>,
    pub reassignments: FxHashSet<Identifier>,
    pub early_return_value: Option<EarlyReturnValue>,
    pub merged: FxHashSet<ScopeId>,
    pub loc: SourceLocation,
}

pub struct ReactiveScopeDependency {
    pub identifier: Identifier,
    pub reactive: bool,
    pub path: Vec<DependencyPathEntry>,
}
```

### Runtime Cache Layout

The compiler uses `useMemoCache(N)` which returns a flat `Array<any>` of size N. Cache slots are allocated per-scope:

```
Scope 1: [dep0, dep1, ..., output0, output1, ...]
Scope 2: [dep0, ..., output0, ...]
...
```

Generated pattern:
```javascript
const $ = _c(N);
if ($[0] !== dep0 || $[1] !== dep1) {
    // recompute
    $[0] = dep0;
    $[1] = dep1;
    $[2] = computedValue;
}
const result = $[2];
```

For constant scopes (no deps): uses `$[slot] === Symbol.for('react.memo_cache_sentinel')` check.

---

## 10. Code Generation

### Approach: Edit-Based (following oxc-angular-compiler pattern)

Rather than rebuilding the entire AST, use surgical text edits:

1. **Parse** with `oxc_parser` to get AST
2. **Discover** compilable functions via AST traversal
3. **Lower** each function to HIR (reading from OXC AST)
4. **Run** the full 62-pass pipeline
5. **Generate** the compiled function body as a JavaScript string
6. **Apply edits**: Replace the original function body with the compiled output
7. **Insert** `import { c as _c } from 'react/compiler-runtime'` at the top

This approach:
- Preserves formatting, comments, and non-compiled code
- Avoids needing full OXC codegen for the entire file
- Matches the oxc-angular-compiler's proven pattern
- Generates source maps via edit tracking

### Alternative: Full OXC AST Generation

For higher fidelity, codegen could produce `oxc_ast` nodes directly:
- More complex to implement
- Better source map quality
- Required if we want to integrate with other OXC transformer passes

**Recommendation**: Start with edit-based approach, migrate to full AST generation later if needed.

---

## 11. Vite/Rolldown Plugin Integration

### Plugin Structure (TypeScript)

```typescript
// vite-plugin/index.ts
import { transformReactFile } from '#binding';

export function reactCompiler(options?: ReactCompilerOptions): Plugin {
    return {
        name: 'oxc-react-compiler',
        enforce: 'pre',  // Run before other transforms

        transform(code: string, id: string) {
            // Filter: only .tsx, .jsx, .ts, .js files
            if (!isReactFile(id)) return null;

            // Quick check: skip if no component/hook patterns
            if (!mightContainReactCode(code)) return null;

            // Call Rust via NAPI
            const result = await transformReactFile(code, id, options);

            if (!result.transformed) return null;
            return { code: result.code, map: result.map };
        },
    };
}
```

### NAPI Bridge (Rust)

```rust
#[napi]
pub fn transform_react_file(
    source: String,
    filename: String,
    options: Option<TransformOptions>,
) -> AsyncTask<TransformReactFileTask> {
    AsyncTask::new(TransformReactFileTask { source, filename, options })
}

impl Task for TransformReactFileTask {
    type JsValue = TransformResult;
    type Output = TransformResult;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(&self.filename).unwrap_or_default();
        let parser_ret = Parser::new(&allocator, &self.source, source_type).parse();

        let result = compile_program(&allocator, &parser_ret.program, &self.options);

        Ok(TransformResult {
            code: result.code,
            map: result.source_map,
            transformed: result.transformed,
        })
    }
}
```

---

## 12. Oxlint Rules

The upstream `eslint-plugin-react-compiler` runs the **entire compiler** in lint mode. For oxlint, we take a hybrid approach:

### Tier 1: Standalone AST Rules (no compiler dependency)

These can be implemented as standard oxlint rules using `oxc_ast` visitors and `oxc_semantic`:

| Rule | Upstream Category | Complexity | Description |
|---|---|---|---|
| `react-compiler/no-jsx-in-try` | ErrorBoundaries | Easy | JSX inside try/catch blocks |
| `react-compiler/use-memo-validation` | UseMemo + VoidUseMemo | Easy | useMemo callback must return, no params, not async |
| `react-compiler/no-capitalized-calls` | CapitalizedCalls | Easy | Calling PascalCase functions as regular calls |
| `react-compiler/purity` | Purity | Easy | Known impure functions (Math.random, Date.now) in render |
| `react-compiler/incompatible-library` | IncompatibleLibrary | Easy | Imports from blocklisted libraries |
| `react-compiler/static-components` | StaticComponents | Medium | Components defined inline during render |
| `react-compiler/no-set-state-in-render` | RenderSetState | Medium | Unconditional setState calls in render |
| `react-compiler/no-set-state-in-effects` | EffectSetState | Medium | Synchronous setState in effect bodies |
| `react-compiler/no-ref-access-in-render` | Refs | Medium | ref.current access during render (simplified) |
| `react-compiler/no-deriving-state-in-effects` | EffectDerivationsOfState | Medium | useEffect(() => setState(f(dep)), [dep]) |
| `react-compiler/globals` | Globals | Medium | Mutating module-scope variables in render |

### Tier 2: Compiler-Dependent Rules (need HIR analysis)

These rules require the compiler's mutation/aliasing analysis or reactive scope inference. They should use the compiler core as a library:

| Rule | Upstream Category | Complexity | Description |
|---|---|---|---|
| `react-compiler/hooks` | Hooks | Hard | Full Rules of Hooks (conditional calls, first-class hooks) |
| `react-compiler/immutability` | Immutability | Hard | Mutation of frozen values (props, state, hook returns) |
| `react-compiler/preserve-manual-memoization` | PreserveManualMemo | Hard | Manual useMemo/useCallback would be preserved |
| `react-compiler/memo-dependencies` | MemoDependencies | Hard | Exhaustive useMemo/useCallback deps (with autofix) |
| `react-compiler/exhaustive-effect-deps` | EffectExhaustiveDependencies | Hard | Exhaustive useEffect deps (with autofix) |

### Tier 3: Not Applicable to Oxlint

| Rule | Reason |
|---|---|
| `react-compiler/config` | Compiler configuration, not linting |
| `react-compiler/gating` | Compiler code generation feature |
| `react-compiler/syntax` | Handled by parser |
| `react-compiler/todo` | Internal compiler tracking |
| `react-compiler/invariant` | Internal compiler tracking |
| `react-compiler/fbt` | Meta-internal |

### Shared Utilities for Lint Rules

```rust
// utils/hook_detection.rs
pub fn is_hook_call(name: &str) -> bool {
    name.starts_with("use") && name.chars().nth(3).map_or(false, |c| c.is_uppercase())
}

pub fn is_component_name(name: &str) -> bool {
    name.chars().next().map_or(false, |c| c.is_uppercase())
        && !name.chars().all(|c| c.is_uppercase() || c == '_')
}

pub fn is_effect_hook(name: &str) -> bool {
    matches!(name, "useEffect" | "useLayoutEffect" | "useInsertionEffect")
}
```

---

## 13. Configuration & Options

### Plugin Options

```rust
pub struct PluginOptions {
    /// How to discover compilable functions
    pub compilation_mode: CompilationMode,  // Infer | Syntax | Annotation | All

    /// Output mode
    pub output_mode: OutputMode,  // Client | SSR | Lint

    /// React version target (determines runtime import)
    pub target: ReactTarget,  // React17 | React18 | React19

    /// Feature flag gating
    pub gating: Option<GatingConfig>,

    /// Error handling threshold
    pub panic_threshold: PanicThreshold,  // AllErrors | CriticalErrors | None

    /// File filter
    pub sources: Option<SourceFilter>,
}
```

### Environment Config (~50 flags)

The full `EnvironmentConfig` should be ported from the upstream Zod schema. Key flags:

```rust
pub struct EnvironmentConfig {
    // Memoization
    pub enable_preserve_existing_memoization_guarantees: bool,
    pub validate_preserve_existing_memoization_guarantees: bool,

    // Outlining
    pub enable_function_outlining: bool,
    pub enable_jsx_outlining: bool,

    // Validation toggles
    pub validate_hooks_usage: bool,
    pub validate_ref_access_during_render: bool,
    pub validate_no_set_state_in_render: bool,
    pub validate_no_set_state_in_effects: bool,
    pub validate_no_derived_computations_in_effects: bool,
    pub validate_no_jsx_in_try_statements: bool,
    pub validate_no_capitalized_calls: bool,
    pub validate_exhaustive_memo_dependencies: bool,
    pub validate_exhaustive_effect_dependencies: bool,

    // Analysis
    pub enable_assume_hooks_follow_rules_of_react: bool,
    pub enable_transitively_freeze_function_expressions: bool,
    pub enable_optional_dependencies: bool,
    pub enable_treat_ref_like_identifiers_as_refs: bool,

    // Extensibility
    pub custom_hooks: FxHashMap<String, CustomHookConfig>,
    pub custom_macros: Option<Vec<String>>,

    // Dev/HMR
    pub enable_reset_cache_on_source_file_changes: bool,
    pub enable_emit_hook_guards: Option<HookGuardConfig>,
}
```

---

## 14. Upstream Merge Strategy

The React Compiler is under active development. Upstream changes must be mergeable. Our strategy:

### Structural Mapping

Maintain a **1:1 file mapping** between upstream TypeScript and our Rust:

| Upstream TS | Our Rust |
|---|---|
| `src/HIR/HIR.ts` | `crates/oxc_react_compiler/src/hir/types.rs` |
| `src/Inference/InferTypes.ts` | `crates/oxc_react_compiler/src/inference/infer_types.rs` |
| `src/Validation/ValidateHooksUsage.ts` | `crates/oxc_react_compiler/src/validation/validate_hooks_usage.rs` |
| ... | ... |

### Naming Conventions

- **Function names**: snake_case versions of upstream camelCase (e.g., `inferReactivePlaces` → `infer_reactive_places`)
- **Type names**: Same PascalCase names (e.g., `InstructionValue`, `ReactiveScope`)
- **Enum variants**: Same PascalCase names (e.g., `Effect::Mutate`, `ValueKind::Frozen`)
- **Constants**: SCREAMING_SNAKE_CASE versions of upstream

### Merge Process

1. **Track upstream**: Pin to a specific React compiler commit. Document it in `UPSTREAM_VERSION.md`.
2. **Diff-based merge**: When upstream updates, diff the TypeScript changes and apply equivalent Rust changes.
3. **Core passes are 1:1**: The 50+ core passes should be algorithmically identical to upstream. Changes in upstream pass logic should translate directly.
4. **Boundary layers are independent**: Changes to `BuildHIR.ts` (Babel AST reading) don't affect us — our OXC lowering is independent. Similarly for codegen.
5. **Test fixtures**: Upstream fixture tests can be reused. The compiler's input→output behavior should be identical.
6. **New passes**: New passes added upstream can be inserted at the correct pipeline position.
7. **HIR type changes**: Changes to `InstructionValue`, `Terminal`, etc. require corresponding Rust enum updates.

### Risk Mitigation

- Keep the boundary layers (BuildHIR, Codegen) as thin as possible
- Avoid refactoring core pass logic — keep it 1:1 even when Rust idioms suggest otherwise
- Document any intentional divergences with `// DIVERGENCE:` comments
- Run upstream test fixtures to verify behavioral equivalence

---

## 15. Testing Strategy

### Unit Tests (Rust)

- **Per-pass tests**: Each pass gets unit tests with small HIR fixtures
- **Snapshot tests** via `insta`: Dump HIR/ReactiveFunction at each pipeline stage

### Integration Tests

- **Fixture tests**: Port upstream `compiler/packages/babel-plugin-react-compiler/src/__tests__/fixtures/`
  - Input: React component source code
  - Expected: Compiled output JavaScript
  - Run through full pipeline and compare
- **Conformance suite**: Separate crate (like `angular_conformance`) that runs all upstream fixtures

### E2E Tests

- **Vite integration**: Build sample React apps with the plugin, verify runtime behavior
- **Comparison tests**: Run same input through both Babel plugin and OXC plugin, diff outputs

### Lint Rule Tests

- **Per-rule tests**: Standard oxlint rule test format
- **Upstream test cases**: Port diagnostic test cases from `eslint-plugin-react-compiler`

---

## 16. Implementation Phases

### Phase 0: Project Scaffolding
- [ ] Initialize Cargo workspace, crate structure
- [ ] Set up NAPI bindings skeleton
- [ ] Set up Vite plugin skeleton
- [ ] Set up CI (cargo check/test/fmt + NAPI build)
- [ ] Pin upstream React compiler commit

### Phase 1: HIR Foundation
- [ ] Implement all HIR types (`types.rs`: ~40 InstructionValue variants, ~20 Terminal variants)
- [ ] Implement `Environment`, `EnvironmentConfig`, `PluginOptions`
- [ ] Implement `ObjectShape`, `ShapeRegistry`, `FunctionSignature`
- [ ] Implement `Globals` (built-in shapes for Array, hooks, etc.)
- [ ] Implement `CompilerError`, `ErrorCategory`
- [ ] Implement utility types: `DisjointSet`, `OrderedMap`

### Phase 2: BuildHIR (OXC AST → HIR)
- [ ] Implement `lower()` — the OXC AST → HIR lowering pass
  - Walk OXC `Statement`/`Expression` nodes
  - Flatten nested expressions into temporaries
  - Convert control flow to explicit block edges
  - Handle JSX, hooks, destructuring, closures
- [ ] Implement function discovery (find compilable components/hooks)
- [ ] Test with simple components

### Phase 3: SSA & Early Optimization
- [ ] `enter_ssa` — phi node insertion, identifier renaming
- [ ] `eliminate_redundant_phi`
- [ ] `constant_propagation`
- [ ] `dead_code_elimination`
- [ ] `inline_iife`
- [ ] `merge_consecutive_blocks`

### Phase 4: Type Inference & Mutation Analysis
- [ ] `infer_types` — constraint-based type inference with shape system
- [ ] `AliasingEffect` enum and composition rules
- [ ] `infer_mutation_aliasing_effects` — abstract interpretation (most complex pass)
- [ ] `infer_mutation_aliasing_ranges` — mutable range computation
- [ ] `analyse_functions` — recursive nested function analysis

### Phase 5: Reactivity & Scope Inference
- [ ] `infer_reactive_places` — fixpoint iteration with post-dominator frontier
- [ ] `infer_reactive_scope_variables` — DisjointSet-based scope grouping
- [ ] All scope alignment passes (align to blocks, merge overlapping, etc.)
- [ ] `propagate_scope_dependencies_hir`
- [ ] `build_reactive_scope_terminals_hir`

### Phase 6: ReactiveFunction & Codegen
- [ ] `build_reactive_function` — HIR CFG → tree-shaped ReactiveFunction
- [ ] All ReactiveFunction pruning/optimization passes
- [ ] `codegen_function` — ReactiveFunction → JavaScript output
- [ ] Source map generation
- [ ] Import insertion (`react/compiler-runtime`)

### Phase 7: Validation Passes
- [ ] All 14 validation passes
- [ ] Error accumulation and reporting
- [ ] Lint mode support (collect errors without codegen)

### Phase 8: Vite Plugin & NAPI
- [ ] Complete NAPI bindings with async task pattern
- [ ] Vite plugin with `transform` hook
- [ ] Configuration parsing
- [ ] HMR support
- [ ] Gating support

### Phase 9: Oxlint Rules (Tier 1)
- [ ] Implement all 11 standalone AST-based lint rules
- [ ] Port upstream test cases
- [ ] Autofix support where applicable

### Phase 10: Oxlint Rules (Tier 2)
- [ ] Wire compiler core as library for lint rules
- [ ] Implement 5 compiler-dependent lint rules
- [ ] Caching (avoid re-running compiler per rule)

### Phase 11: Polish & Conformance
- [ ] Run full upstream fixture suite
- [ ] Performance benchmarking
- [ ] Documentation
- [ ] Release packaging (platform-specific NAPI binaries)

---

## Appendix A: Upstream File → Rust Module Mapping

| Upstream Path | Rust Module |
|---|---|
| `src/HIR/HIR.ts` | `hir/types.rs` |
| `src/HIR/Environment.ts` | `hir/environment.rs` |
| `src/HIR/ObjectShape.ts` | `hir/object_shape.rs` |
| `src/HIR/Globals.ts` | `hir/globals.rs` |
| `src/HIR/Types.ts` | `hir/type_system.rs` |
| `src/HIR/BuildHIR.ts` | `hir/build.rs` |
| `src/Entrypoint/Pipeline.ts` | `entrypoint/pipeline.rs` |
| `src/Entrypoint/Program.ts` | `entrypoint/program.rs` |
| `src/Entrypoint/Options.ts` | `entrypoint/options.rs` |
| `src/SSA/EnterSSA.ts` | `ssa/enter_ssa.rs` |
| `src/SSA/EliminateRedundantPhi.ts` | `ssa/eliminate_redundant_phi.rs` |
| `src/Optimization/ConstantPropagation.ts` | `optimization/constant_propagation.rs` |
| `src/Optimization/DeadCodeElimination.ts` | `optimization/dead_code_elimination.rs` |
| `src/Optimization/InlineImmediatelyInvokedFunctionExpressions.ts` | `optimization/inline_iife.rs` |
| `src/Inference/InferTypes.ts` | `inference/infer_types.rs` |
| `src/Inference/InferMutationAliasingEffects.ts` | `inference/infer_mutation_aliasing_effects.rs` |
| `src/Inference/InferMutationAliasingRanges.ts` | `inference/infer_mutation_aliasing_ranges.rs` |
| `src/Inference/InferReactivePlaces.ts` | `inference/infer_reactive_places.rs` |
| `src/Inference/InferReactiveScopeVariables.ts` | `reactive_scopes/infer_reactive_scope_variables.rs` |
| `src/Inference/AliasingEffects.ts` | `inference/aliasing_effects.rs` |
| `src/ReactiveScopes/BuildReactiveFunction.ts` | `reactive_scopes/build_reactive_function.rs` |
| `src/ReactiveScopes/CodegenReactiveFunction.ts` | `reactive_scopes/codegen.rs` |
| `src/ReactiveScopes/PropagateScopeDependenciesHIR.ts` | `reactive_scopes/propagate_dependencies.rs` |
| `src/ReactiveScopes/AlignReactiveScopesToBlockScopesHIR.ts` | `reactive_scopes/align_scopes.rs` |
| `src/ReactiveScopes/MergeOverlappingReactiveScopesHIR.ts` | `reactive_scopes/merge_scopes.rs` |
| `src/ReactiveScopes/PruneNonEscapingScopes.ts` | `reactive_scopes/prune_scopes.rs` |
| `src/Validation/Validate*.ts` | `validation/validate_*.rs` (1:1) |
| `src/CompilerError.ts` | `error.rs` |
| `src/Utils/DisjointSet.ts` | `utils/disjoint_set.rs` |

## Appendix B: OXC API Surface Used

| OXC Crate | APIs Used |
|---|---|
| `oxc_allocator` | `Allocator`, arena allocation for AST nodes |
| `oxc_parser` | `Parser::new(&allocator, source, source_type).parse()` |
| `oxc_ast` | `Program`, `Statement`, `Expression`, `JSXElement`, all AST node types for reading in BuildHIR |
| `oxc_semantic` | `SemanticBuilder`, `ScopeTree`, `SymbolTable` — for function discovery and scope analysis |
| `oxc_span` | `Span`, `Atom`, `SourceType` — source locations and string interning |
| `oxc_diagnostics` | `OxcDiagnostic` — error/warning reporting |
| `oxc_codegen` | `Codegen` — optional, for generating output JS from AST nodes |
| `oxc_sourcemap` | Source map generation and composition |

## Appendix C: Key Algorithmic Complexity

| Algorithm | Location | Complexity | Notes |
|---|---|---|---|
| Abstract interpretation | `infer_mutation_aliasing_effects` | O(n × k) fixpoint | k = iterations to convergence, typically 2-5 |
| Union-Find | `infer_reactive_scope_variables` | O(n α(n)) | Near-linear with path compression |
| Post-dominator frontier | `infer_reactive_places` | O(n²) worst case | Fixpoint over CFG |
| SSA construction | `enter_ssa` | O(n) | Standard algorithm with dominance frontiers |
| Scope alignment | `align_reactive_scopes_to_block_scopes_hir` | O(n) | Single CFG traversal |
| Dependency propagation | `propagate_scope_dependencies_hir` | O(n × s) | n = instructions, s = scopes |
