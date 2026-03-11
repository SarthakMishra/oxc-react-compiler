# HIR Types

> All core data structures for the compiler's internal representation.
> Upstream: `src/HIR/HIR.ts`, `src/HIR/Types.ts`
> Rust module: `crates/oxc_react_compiler/src/hir/types.rs`

---

### Gap 1: Core ID Newtypes and Place Types

**Upstream:** `src/HIR/HIR.ts` (top-level type definitions)
**Current state:** `types.rs` is a stub with a single comment line.
**What's needed:**

- `BlockId(u32)` newtype with `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord` derives
- `ScopeId(u32)` newtype with same derives
- `IdentifierId(u32)` newtype with same derives
- `DeclarationId(u32)` newtype with same derives
- `InstructionId(u32)` newtype with same derives
- `TypeId(u32)` newtype with same derives
- `Place` struct: `identifier: Identifier`, `effect: Effect`, `reactive: bool`, `loc: SourceLocation`
- `Identifier` struct: `id: IdentifierId`, `declaration_id: DeclarationId`, `name: Option<String>`, `mutable_range: MutableRange`, `scope: Option<Box<ReactiveScope>>`, `type_: Type`, `loc: SourceLocation`
- `MutableRange` struct: `start: InstructionId`, `end: InstructionId`
- `SourceLocation` type (wrapping `oxc_span::Span` or equivalent)
- ID counter/generator for producing unique IDs during HIR construction
- `Display` implementations for all newtypes (useful for debugging and snapshot tests)

**Depends on:** None

---

### Gap 2: InstructionValue Enum

**Upstream:** `src/HIR/HIR.ts` — `InstructionValue` type (discriminated union, ~40 variants)
**Current state:** Nothing implemented.
**What's needed:**

All ~40 variants from REQUIREMENTS.md Section 6:

- **Locals & context:** `LoadLocal`, `StoreLocal`, `LoadContext`, `StoreContext`, `DeclareLocal`, `DeclareContext`, `Destructure`
- **Literals:** `Primitive`, `JSXText`, `RegExpLiteral`, `TemplateLiteral`
- **Operators:** `BinaryExpression`, `UnaryExpression`, `PrefixUpdate`, `PostfixUpdate`
- **Calls:** `CallExpression`, `MethodCall`, `NewExpression`
- **Property access:** `PropertyLoad`, `PropertyStore`, `ComputedLoad`, `ComputedStore`, `PropertyDelete`, `ComputedDelete`
- **Containers:** `ObjectExpression`, `ArrayExpression`
- **JSX:** `JsxExpression`, `JsxFragment`
- **Functions:** `FunctionExpression`, `ObjectMethod`
- **Globals:** `LoadGlobal`, `StoreGlobal`
- **Async/Iterator:** `Await`, `GetIterator`, `IteratorNext`, `NextPropertyOf`
- **Type:** `TypeCastExpression`, `TaggedTemplateExpression`
- **Manual memoization:** `StartMemoize`, `FinishMemoize`
- **Catch-all:** `UnsupportedNode`

Supporting types needed:
- `Primitive` enum (string, number, boolean, null, undefined, bigint)
- `BinaryOp`, `UnaryOp`, `UpdateOp` enums (map from `oxc_syntax::operator`)
- `InstructionKind` enum (Let, Const, Var, etc.)
- `DestructurePattern` enum (Object, Array with sub-patterns)
- `ObjectProperty` struct (key, value, shorthand)
- `ArrayElement` enum (Hole, Spread, Expression)
- `JsxAttribute` struct (name, value, spread)
- `GlobalBinding` struct (name, kind)
- `FunctionExprType` enum (FunctionExpression, ArrowFunction)
- `TemplateLiteralData` struct (quasis, subexpressions)
- `LogicalOp` enum (And, Or, NullishCoalescing)

**Depends on:** Gap 1 (newtypes, Place)

---

### Gap 3: Terminal Enum

**Upstream:** `src/HIR/HIR.ts` — `Terminal` type (discriminated union, ~20 variants)
**Current state:** Nothing implemented.
**What's needed:**

All ~20 variants from REQUIREMENTS.md Section 6:
- `Goto`, `If`, `Branch`, `Switch`, `Return`, `Throw`
- `For`, `ForOf`, `ForIn`, `DoWhile`, `While`
- `Logical`, `Ternary`, `Optional`, `Sequence`
- `Label`, `MaybeThrow`, `Try`, `Scope`, `PrunedScope`, `Unreachable`

Supporting types:
- `SwitchCase` struct (test: Option<Place>, block: BlockId)

**Depends on:** Gap 1 (BlockId, Place)

---

### Gap 4: HIR Container Types

**Upstream:** `src/HIR/HIR.ts` — `HIRFunction`, `HIR`, `BasicBlock`, `Phi`, `Instruction`
**Current state:** Nothing implemented.
**What's needed:**

- `HIRFunction` struct with all fields from REQUIREMENTS.md Section 6
  - `loc`, `id`, `fn_type: ReactFunctionType`, `env: Environment`, `params: Vec<Param>`, `returns: Place`, `context: Vec<Place>`, `body: HIR`, `is_async`, `is_generator`, `directives`
- `ReactFunctionType` enum: `Component`, `Hook`, `Other`
- `Param` enum: `Identifier(Place)`, `SpreadPattern(Place)`
- `HIR` struct: `entry: BlockId`, `blocks: IndexMap<BlockId, BasicBlock>`
- `BasicBlock` struct: `kind: BlockKind`, `id: BlockId`, `instructions: Vec<Instruction>`, `terminal: Terminal`, `preds: FxHashSet<BlockId>`, `phis: FxHashSet<Phi>`
- `BlockKind` enum: `Block`, `Value`, `Loop`, `Sequence`, `Catch`
- `Instruction` struct: `id: InstructionId`, `lvalue: Place`, `value: InstructionValue`, `loc: SourceLocation`, `effects: Option<Vec<AliasingEffect>>`
- `Phi` struct: `id: InstructionId`, `place: Place`, `operands: FxHashMap<BlockId, Place>`

**Depends on:** Gap 1, Gap 2, Gap 3

---

### Gap 5: Effect, ValueKind, and Supporting Enums

**Upstream:** `src/HIR/HIR.ts` — `Effect`, `ValueKind` types
**Current state:** Nothing implemented.
**What's needed:**

- `Effect` enum with ordering (for lattice operations): `Unknown`, `Freeze`, `Read`, `Capture`, `ConditionallyMutateIterator`, `ConditionallyMutate`, `Mutate`, `Store`
- `ValueKind` enum: `MaybeFrozen`, `Frozen`, `Primitive`, `Global`, `Mutable`, `Context`
- `ValueReason` enum (for Create effect diagnostics)
- `FreezeReason` enum (for Freeze effect diagnostics)
- Type lattice: `Type` enum with variants for Primitive, Object, Function, etc.
  - Needs to match upstream `src/HIR/Types.ts`
  - `TypeId`-based type table for efficient comparison
  - Subtyping relationships

**Depends on:** Gap 1

---

### Gap 6: ReactiveFunction IR Types

**Upstream:** `src/HIR/HIR.ts` — `ReactiveFunction`, `ReactiveBlock`, `ReactiveInstruction`, `ReactiveScope`, `ReactiveScopeBlock`, etc.
**Current state:** Nothing implemented.
**What's needed:**

The ReactiveFunction is a tree-shaped IR (as opposed to the CFG-shaped HIR). It is the output of `BuildReactiveFunction` and the input to codegen.

- `ReactiveFunction` struct: `loc`, `id`, `params`, `body: ReactiveBlock`, `env`, `directives`
- `ReactiveBlock` struct: `instructions: Vec<ReactiveInstruction>`
- `ReactiveInstruction` enum:
  - `Instruction(Instruction)` — a regular HIR instruction
  - `Terminal(ReactiveTerminal)` — control flow
  - `Scope(ReactiveScopeBlock)` — memoized scope block
- `ReactiveTerminal` enum: mirrors Terminal but tree-shaped (blocks contain ReactiveBlock instead of BlockId)
  - `If { test, consequent: ReactiveBlock, alternate: ReactiveBlock }`
  - `Switch { test, cases: Vec<(Option<Place>, ReactiveBlock)> }`
  - `For { init, test, update, body: ReactiveBlock }`
  - `While`, `DoWhile`, `ForOf`, `ForIn` — same pattern
  - `Label`, `Try`, `Return`, `Throw`
- `ReactiveScopeBlock` struct: `scope: ReactiveScope`, `instructions: ReactiveBlock`
- `ReactiveScope` struct (from REQUIREMENTS.md Section 9):
  - `id: ScopeId`, `range: MutableRange`
  - `dependencies: FxHashSet<ReactiveScopeDependency>`
  - `declarations: FxHashMap<IdentifierId, ReactiveScopeDeclaration>`
  - `reassignments: FxHashSet<Identifier>`
  - `early_return_value: Option<EarlyReturnValue>`
  - `merged: FxHashSet<ScopeId>`
  - `loc: SourceLocation`
- `ReactiveScopeDependency` struct: `identifier: Identifier`, `reactive: bool`, `path: Vec<DependencyPathEntry>`
- `ReactiveScopeDeclaration` struct: `identifier: Identifier`, `scope: ScopeId`
- `DependencyPathEntry` struct
- `EarlyReturnValue` struct

**Depends on:** Gap 1, Gap 2, Gap 3, Gap 4, Gap 5
