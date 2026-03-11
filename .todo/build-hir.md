# BuildHIR (OXC AST to HIR Lowering)

> The OXC-specific boundary layer. Replaces upstream `BuildHIR.ts` which reads Babel AST.
> This is one of the ~5 files that must be reimplemented for OXC rather than ported 1:1.
> Upstream: `src/HIR/BuildHIR.ts`
> Rust module: `crates/oxc_react_compiler/src/hir/build.rs`

---

### Gap 1: Statement Lowering

**Upstream:** `src/HIR/BuildHIR.ts` — `lowerStatement()` and related functions
**Current state:** `hir/build.rs` is a stub.
**What's needed:**

Convert OXC `Statement` nodes into HIR instructions and basic blocks:

- `VariableDeclaration` -> `DeclareLocal` + optional `StoreLocal`
  - Handle `let`, `const`, `var` with correct `InstructionKind`
  - Handle multiple declarators in a single declaration
- `ExpressionStatement` -> lower the expression, discard the result
- `ReturnStatement` -> lower expression, emit `Return` terminal
- `ThrowStatement` -> lower expression, emit `Throw` terminal
- `BlockStatement` -> create new basic block scope
- `EmptyStatement` -> no-op
- `DebuggerStatement` -> `UnsupportedNode` or skip
- `LabeledStatement` -> `Label` terminal with label ID tracking
- `BreakStatement` / `ContinueStatement` -> `Goto` to appropriate target block
- `WithStatement` -> `UnsupportedNode`
- `ImportDeclaration` / `ExportDeclaration` -> handle at program level, not inside function bodies

Key patterns:
- Every expression must be flattened into a temporary: `a + b * c` becomes `t0 = b * c; t1 = a + t0`
- Each temporary gets a fresh `IdentifierId` from the `Environment`
- Source locations must be preserved on every `Place` and `Instruction`

**Depends on:** HIR types (all gaps), Environment

---

### Gap 2: Expression Lowering

**Upstream:** `src/HIR/BuildHIR.ts` — `lowerExpression()` and related functions
**Current state:** Nothing implemented.
**What's needed:**

Convert OXC `Expression` nodes into sequences of HIR instructions, returning the `Place` holding the result:

- `Identifier` -> `LoadLocal` (or `LoadContext` if captured) or `LoadGlobal`
- `NumericLiteral`, `StringLiteral`, `BooleanLiteral`, `NullLiteral`, `BigIntLiteral` -> `Primitive`
- `BinaryExpression` -> lower left, lower right, emit `BinaryExpression`
- `UnaryExpression` -> lower operand, emit `UnaryExpression`
- `UpdateExpression` -> `PrefixUpdate` or `PostfixUpdate`
- `AssignmentExpression` -> lower RHS, emit `StoreLocal`/`PropertyStore`/`ComputedStore` based on LHS pattern
  - Handle compound assignment (`+=`, `-=`, etc.) by decomposing into read + op + write
- `CallExpression` -> lower callee, lower args, emit `CallExpression` or `MethodCall`
  - Distinguish `.call()` on member expressions -> `MethodCall`
- `NewExpression` -> lower callee, lower args, emit `NewExpression`
- `MemberExpression` -> `PropertyLoad` or `ComputedLoad`
- `ConditionalExpression` -> `Ternary` terminal with consequent/alternate blocks
- `LogicalExpression` -> `Logical` terminal (short-circuit evaluation creates CFG branches)
- `SequenceExpression` -> `Sequence` terminal
- `TemplateLiteral` -> lower quasis and expressions, emit `TemplateLiteral`
- `TaggedTemplateExpression` -> emit `TaggedTemplateExpression`
- `AwaitExpression` -> lower value, emit `Await`
- `YieldExpression` -> handle generator yield
- `TypeCastExpression` / `TSAsExpression` -> `TypeCastExpression` (strip type, keep value)
- `SpreadElement` -> handle in array/call contexts
- `ParenthesizedExpression` -> unwrap and lower inner
- `ThisExpression` -> `LoadLocal` for implicit `this` binding
- `ClassExpression` -> `UnsupportedNode` or lower if supported
- `MetaProperty` (`import.meta`, `new.target`) -> `LoadGlobal` or `UnsupportedNode`

**Depends on:** Gap 1 (statement lowering shares infrastructure)

---

### Gap 3: Control Flow Lowering

**Upstream:** `src/HIR/BuildHIR.ts` — control flow handling
**Current state:** Nothing implemented.
**What's needed:**

Convert OXC control flow into explicit CFG edges with Terminal variants:

- `IfStatement` -> `If` terminal with consequent block, alternate block, fallthrough block
- `SwitchStatement` -> `Switch` terminal with case blocks
  - Handle fall-through between cases
  - Handle `default` case
- `ForStatement` -> `For` terminal with init, test, update, body, fallthrough blocks
- `ForInStatement` -> `ForIn` terminal
- `ForOfStatement` -> `ForOf` terminal with `GetIterator` + `IteratorNext` instructions
- `WhileStatement` -> `While` terminal
- `DoWhileStatement` -> `DoWhile` terminal
- `TryStatement` -> `Try` terminal with handler block
  - `MaybeThrow` terminals at call sites within try blocks
  - `Catch` clause parameter binding
  - `Finally` block handling
- `OptionalExpression` (`?.`) -> `Optional` terminal
- Break/continue with labels -> `Goto` to correct block, track label mapping
- Exception edges -> `MaybeThrow` terminals for calls that might throw

Block linking:
- Maintain predecessor sets (`preds`) on each block
- Ensure blocks are in reverse-postorder in the `IndexMap`

**Depends on:** Gap 1, Gap 2

---

### Gap 4: JSX Lowering

**Upstream:** `src/HIR/BuildHIR.ts` — JSX handling
**Current state:** Nothing implemented.
**What's needed:**

- `JSXElement` -> `JsxExpression` instruction
  - Lower tag: identifier for components, string for intrinsic elements
  - Lower each attribute to `JsxAttribute` (name-value or spread)
  - Lower children recursively
  - Handle `key` prop specially (separate from other props in some contexts)
- `JSXFragment` -> `JsxFragment` instruction with children
- `JSXText` -> `JSXText` instruction (trimming whitespace per React JSX rules)
- `JSXExpressionContainer` -> lower the contained expression
- `JSXSpreadChild` -> spread in children array
- `JSXSpreadAttribute` -> spread in props
- `JSXMemberExpression` -> `PropertyLoad` chain (e.g., `Foo.Bar.Baz`)
- `JSXNamespacedName` -> string concatenation of namespace:name

**Depends on:** Gap 2 (expression lowering)

---

### Gap 5: Function Lowering

**Upstream:** `src/HIR/BuildHIR.ts` — function expression/declaration handling
**Current state:** Nothing implemented.
**What's needed:**

- `FunctionDeclaration` -> `DeclareLocal` + `FunctionExpression` instruction
- `FunctionExpression` -> `FunctionExpression` instruction with `lowered_func: Box<HIRFunction>`
  - Recursively lower the function body into a new `HIRFunction`
  - Capture analysis: determine which outer variables are referenced (these become `context` entries)
  - Handle `this` binding differences between arrow and regular functions
- `ArrowFunctionExpression` -> `FunctionExpression` with `FunctionExprType::ArrowFunction`
  - Arrow functions inherit `this` from enclosing scope
  - Arrow functions cannot be generators
  - Concise body (`=> expr`) vs block body (`=> { ... }`)
- `ObjectMethod` -> `ObjectMethod` instruction
- `async` modifier -> `is_async: true` on `HIRFunction`
- `generator` modifier -> `is_generator: true` on `HIRFunction`
- Default parameter values -> lower as conditional assignment in function body
- Rest parameters -> `SpreadPattern`

**Depends on:** Gap 1, Gap 2, Gap 3

---

### Gap 6: Destructuring Lowering

**Upstream:** `src/HIR/BuildHIR.ts` — destructuring pattern handling
**Current state:** Nothing implemented.
**What's needed:**

Convert destructuring patterns into `Destructure` instructions with `DestructurePattern`:

- Object destructuring: `{ a, b: c, ...rest }` -> `Destructure` with `ObjectPattern`
  - Simple property: `{ a }` -> read property `a`
  - Renamed property: `{ a: b }` -> read property `a`, bind to `b`
  - Computed property: `{ [expr]: name }` -> computed read
  - Default value: `{ a = default }` -> conditional assignment
  - Rest element: `{ ...rest }` -> collect remaining
- Array destructuring: `[a, , b, ...rest]` -> `Destructure` with `ArrayPattern`
  - Holes: `[, , x]` -> skip indices
  - Default values: `[a = default]`
  - Rest element: `[...rest]`
  - Nested: `[{ a }, [b]]` -> recursive pattern
- Nested destructuring: patterns within patterns
- Function parameter destructuring -> same patterns applied to params
- Variable declaration destructuring: `const { a, b } = obj`
- Assignment destructuring: `({ a, b } = obj)` (LHS patterns)

**Depends on:** Gap 2 (expression lowering for default values)

---

### Gap 7: Pattern and Iterator Lowering

**Upstream:** `src/HIR/BuildHIR.ts` — for-of/for-in, spread, computed properties
**Current state:** Nothing implemented.
**What's needed:**

- `for...of` lowering:
  - `GetIterator` instruction on the iterable
  - `IteratorNext` instruction in the loop test
  - Destructuring of iterator result into loop variable
- `for...in` lowering:
  - `NextPropertyOf` instruction pattern
- Spread in arrays: `[...arr]` -> iterate and collect
- Spread in objects: `{...obj}` -> copy properties
- Spread in calls: `fn(...args)` -> spread args
- Computed property names: `{ [expr]: value }` -> lower expr, use as key
- Object methods with computed names

**Depends on:** Gap 2, Gap 6

---

### Gap 8: Function Discovery

**Upstream:** `src/Entrypoint/Program.ts` — function discovery and compilation gating
**Current state:** `entrypoint/program.rs` is a stub.
**What's needed:**

Walk the OXC AST to find functions that should be compiled:

- Identify React components:
  - Named function declarations/expressions starting with uppercase
  - Arrow functions assigned to uppercase-named variables
  - Functions returned from `React.forwardRef()`, `React.memo()`
  - Default exports of component-shaped functions
- Identify React hooks:
  - Functions starting with `use` followed by uppercase letter
- Apply `CompilationMode` gating:
  - `Infer`: compile functions that look like components/hooks
  - `Syntax`: compile functions with `"use memo"` or `"use no memo"` directive
  - `Annotation`: compile functions with specific annotations
  - `All`: compile everything
- Apply `SourceFilter` from `PluginOptions`
- Skip functions with `"use no memo"` directive
- Handle nested function expressions (may be compiled as part of parent)
- Return list of `(function_node, ReactFunctionType)` pairs for compilation

**Depends on:** PluginOptions (environment.md Gap 2)

---

### Gap 9: Context Variable Handling

**Upstream:** `src/HIR/BuildHIR.ts` — closure variable handling
**Current state:** Nothing implemented.
**What's needed:**

When lowering a function that references variables from an outer scope:

- Use `oxc_semantic` scope tree to determine which references are to outer variables
- Create `LoadContext` / `StoreContext` instructions for outer variable access
- Populate `HIRFunction.context` with the list of captured places
- Handle `this` as an implicit context variable for non-arrow functions
- Handle `arguments` as a context variable
- Mutable context variables (let/var from outer scope that might be reassigned) need special treatment
- Const context variables can be treated as immutable

**Depends on:** Gap 1, Gap 2, Gap 5

---

### Gap 10: Manual Memoization Markers

**Upstream:** `src/HIR/BuildHIR.ts` — useMemo/useCallback detection
**Current state:** Nothing implemented.
**What's needed:**

When the user has existing `useMemo()` or `useCallback()` calls, the compiler inserts markers to track them:

- Detect `useMemo(() => expr, [deps])` calls
- Emit `StartMemoize { manualMemoId }` before the call
- Emit `FinishMemoize { manualMemoId, decl, deps, pruned: false }` after
- Same for `useCallback(fn, [deps])`
- The `manualMemoId` links the start/finish pair
- These markers are used by `ValidatePreservedManualMemoization` to ensure the compiler's memoization is at least as good as the manual one
- `DropManualMemoization` pass may remove these markers if the config says not to preserve them

**Depends on:** Gap 2 (expression lowering for detecting useMemo/useCallback calls)
