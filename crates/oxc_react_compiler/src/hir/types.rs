//! HIR (High-level Intermediate Representation) type definitions.
//!
//! These types model the React Compiler's HIR, adapted for Rust/OXC.
//! They represent the compiler's internal representation of React components
//! and hooks after lowering from the AST.

use std::fmt;

use oxc_span::Span;

// ---------------------------------------------------------------------------
// SourceLocation
// ---------------------------------------------------------------------------

pub type SourceLocation = Span;

// ---------------------------------------------------------------------------
// ID Newtypes
// ---------------------------------------------------------------------------

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id!(BlockId);
define_id!(ScopeId);
define_id!(IdentifierId);
define_id!(DeclarationId);
define_id!(InstructionId);
define_id!(TypeId);

// ---------------------------------------------------------------------------
// MutableRange
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MutableRange {
    pub start: InstructionId,
    pub end: InstructionId,
}

// ---------------------------------------------------------------------------
// Effect
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
#[derive(Default)]
pub enum Effect {
    #[default]
    Unknown = 0,
    Freeze = 1,
    Read = 2,
    Capture = 3,
    ConditionallyMutateIterator = 4,
    ConditionallyMutate = 5,
    Mutate = 6,
    Store = 7,
}

// ---------------------------------------------------------------------------
// ValueKind / ValueReason / FreezeReason
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    MaybeFrozen,
    Frozen,
    Primitive,
    Global,
    Mutable,
    Context,
}

/// Reason why a value has a particular `ValueKind`.
///
/// Upstream: `ValueReason` in `HIR.ts`. Used by `Create` and `Freeze` effects
/// to explain provenance of a value's kind classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueReason {
    /// The value kind is known from the instruction itself (e.g. literals, object/array creation).
    KnownValue,
    /// Global variable access.
    Global,
    /// Value was captured by JSX (frozen as a prop/child).
    JsxCaptured,
    /// Value was captured by a hook call.
    HookCaptured,
    /// Value is a return from a hook call.
    HookReturn,
    /// Value is an effect callback argument.
    Effect,
    /// Value's kind is known from a built-in function's return signature.
    KnownReturnSignature,
    /// Value is a React context value.
    Context,
    /// Value is React state (from useState).
    State,
    /// Value is reducer state (from useReducer).
    ReducerState,
    /// Value is a parameter of a reactive function (component/hook).
    ReactiveFunctionArgument,
    /// Catch-all for other/unclassified reasons.
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FreezeReason {
    FrozenByBinding,
    FrozenByValue,
    Other,
}

/// Reason annotation for `Mutate` effects.
///
/// Upstream: `MutationReason` in `AliasingEffects.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MutationReason {
    /// The mutation is assigning to `.current` property (e.g. `ref.current = ...`).
    AssignCurrentProperty,
}

// ---------------------------------------------------------------------------
// Type / PrimitiveType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Type {
    Primitive(PrimitiveType),
    Object,
    Function,
    /// Return value of useRef() — accessing .current during render is invalid.
    Ref,
    /// A setState/dispatch function returned from useState/useReducer.
    SetState,
    /// Unknown / generic type.
    #[default]
    Poly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    String,
    Number,
    Boolean,
    Null,
    Undefined,
    BigInt,
    Symbol,
}

// ---------------------------------------------------------------------------
// Primitive (literal values)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Null,
    Undefined,
    Boolean(bool),
    Number(f64),
    String(String),
    BigInt(String),
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Exp,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    UnsignedShiftRight,
    EqEq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    In,
    InstanceOf,
    NullishCoalescing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Minus,
    Plus,
    Not,
    BitwiseNot,
    TypeOf,
    Void,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateOp {
    Increment,
    Decrement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalOp {
    And,
    Or,
    NullishCoalescing,
}

// ---------------------------------------------------------------------------
// InstructionKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstructionKind {
    Let,
    Const,
    Var,
    Reassign,
    HoistedConst,
    HoistedFunction,
}

// ---------------------------------------------------------------------------
// Identifier / Place
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Identifier {
    pub id: IdentifierId,
    /// SSA version number. All references to the same binding share the same
    /// `id`; the SSA pass distinguishes them via this version counter.
    pub ssa_version: u32,
    pub declaration_id: Option<DeclarationId>,
    pub name: Option<String>,
    pub mutable_range: MutableRange,
    /// The last instruction where this identifier is used as an operand.
    /// Populated by `annotate_last_use()` after `infer_mutation_aliasing_ranges`.
    /// Used by scope inference to decide if a call result escapes its definition site.
    pub last_use: InstructionId,
    pub scope: Option<Box<ReactiveScope>>,
    pub type_: Type,
    pub loc: SourceLocation,
}

impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Identifier {}

impl std::hash::Hash for Identifier {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct Place {
    pub identifier: Identifier,
    pub effect: Effect,
    pub reactive: bool,
    pub loc: SourceLocation,
}

// ---------------------------------------------------------------------------
// InstructionValue
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum InstructionValue {
    // Locals & context
    LoadLocal {
        place: Place,
    },
    StoreLocal {
        lvalue: Place,
        value: Place,
        type_: Option<InstructionKind>,
    },
    LoadContext {
        place: Place,
    },
    StoreContext {
        lvalue: Place,
        value: Place,
    },
    DeclareLocal {
        lvalue: Place,
        type_: InstructionKind,
    },
    DeclareContext {
        lvalue: Place,
    },
    Destructure {
        lvalue_pattern: DestructurePattern,
        value: Place,
    },

    // Literals
    Primitive {
        value: Primitive,
    },
    JSXText {
        value: String,
    },
    RegExpLiteral {
        pattern: String,
        flags: String,
    },
    TemplateLiteral {
        quasis: Vec<String>,
        subexpressions: Vec<Place>,
    },

    // Operators
    BinaryExpression {
        op: BinaryOp,
        left: Place,
        right: Place,
    },
    UnaryExpression {
        op: UnaryOp,
        value: Place,
    },
    PrefixUpdate {
        op: UpdateOp,
        lvalue: Place,
    },
    PostfixUpdate {
        op: UpdateOp,
        lvalue: Place,
    },

    // Calls
    CallExpression {
        callee: Place,
        args: Vec<Place>,
        optional: bool,
    },
    MethodCall {
        receiver: Place,
        property: String,
        args: Vec<Place>,
        /// Optional call: `x.method?.(args)` — the `?.` is on the call
        optional: bool,
        /// Optional member access: `x?.method(args)` — the `?.` is on the receiver
        optional_receiver: bool,
    },
    NewExpression {
        callee: Place,
        args: Vec<Place>,
    },

    // Property access
    PropertyLoad {
        object: Place,
        property: String,
        optional: bool,
    },
    PropertyStore {
        object: Place,
        property: String,
        value: Place,
    },
    ComputedLoad {
        object: Place,
        property: Place,
        optional: bool,
    },
    ComputedStore {
        object: Place,
        property: Place,
        value: Place,
    },
    PropertyDelete {
        object: Place,
        property: String,
    },
    ComputedDelete {
        object: Place,
        property: Place,
    },

    // Containers
    ObjectExpression {
        properties: Vec<ObjectProperty>,
    },
    ArrayExpression {
        elements: Vec<ArrayElement>,
    },

    // JSX
    JsxExpression {
        tag: Place,
        props: Vec<JsxAttribute>,
        children: Vec<Place>,
    },
    JsxFragment {
        children: Vec<Place>,
    },

    // Functions
    FunctionExpression {
        name: Option<String>,
        lowered_func: Box<HIRFunction>,
        expr_type: FunctionExprType,
    },
    ObjectMethod {
        lowered_func: Box<HIRFunction>,
    },

    // Globals
    LoadGlobal {
        binding: GlobalBinding,
    },
    StoreGlobal {
        name: String,
        value: Place,
    },

    // Async/Iterator
    Await {
        value: Place,
    },
    GetIterator {
        collection: Place,
    },
    IteratorNext {
        iterator: Place,
        loc: SourceLocation,
    },
    NextPropertyOf {
        value: Place,
    },

    // Type
    TypeCastExpression {
        value: Place,
        type_: String,
    },
    TaggedTemplateExpression {
        tag: Place,
        value: TemplateLiteralData,
    },

    // Manual memoization markers
    StartMemoize {
        manual_memo_id: u32,
        /// When true, the dependency array for this manual memo has invalid deps
        /// (detected by ValidateExhaustiveDependencies). This flag deduplicates
        /// errors with ValidatePreservedManualMemoization so it won't re-report
        /// the same dependency issue.
        has_invalid_deps: bool,
        /// Source dependencies extracted from the useMemo/useCallback dep array AST.
        /// `None` means no dep array was provided; `Some(vec)` means a dep array was
        /// present (possibly empty `[]`).
        source_deps: Option<Vec<ManualMemoDependency>>,
    },
    FinishMemoize {
        manual_memo_id: u32,
        decl: Place,
        deps: Vec<Place>,
        pruned: bool,
    },

    // Catch-all
    UnsupportedNode {
        node: String,
    },
}

// ---------------------------------------------------------------------------
// Supporting types for InstructionValue
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ObjectProperty {
    pub key: ObjectPropertyKey,
    pub value: Place,
    pub shorthand: bool,
}

#[derive(Debug, Clone)]
pub enum ObjectPropertyKey {
    Identifier(String),
    Computed(Place),
}

#[derive(Debug, Clone)]
pub enum ArrayElement {
    Hole,
    Spread(Place),
    Expression(Place),
}

#[derive(Debug, Clone)]
pub struct JsxAttribute {
    pub name: JsxAttributeName,
    pub value: Place,
}

#[derive(Debug, Clone)]
pub enum JsxAttributeName {
    Named(String),
    /// Spread attributes have no name.
    Spread,
}

#[derive(Debug, Clone)]
pub struct GlobalBinding {
    pub name: String,
    pub kind: GlobalBindingKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlobalBindingKind {
    Global,
    Module,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionExprType {
    FunctionExpression,
    ArrowFunction,
}

#[derive(Debug, Clone)]
pub struct TemplateLiteralData {
    pub quasis: Vec<String>,
    pub subexpressions: Vec<Place>,
}

#[derive(Debug, Clone)]
pub enum DestructurePattern {
    Object { properties: Vec<DestructureObjectProperty>, rest: Option<Place> },
    Array { items: Vec<DestructureArrayItem>, rest: Option<Place> },
}

#[derive(Debug, Clone)]
pub struct DestructureObjectProperty {
    pub key: String,
    pub value: DestructureTarget,
    pub shorthand: bool,
    /// Optional default value for this property. When present, the destructured
    /// value should be checked against `undefined` and replaced with this default
    /// if undefined. Corresponds to `{ key = defaultExpr }` syntax.
    pub default_value: Option<Place>,
}

#[derive(Debug, Clone)]
pub enum DestructureTarget {
    Place(Place),
    Pattern(Box<DestructurePattern>),
}

#[derive(Debug, Clone)]
pub enum DestructureArrayItem {
    Hole,
    Value(DestructureTarget),
    Spread(Place),
}

// ---------------------------------------------------------------------------
// Terminal
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Terminal {
    Goto {
        block: BlockId,
    },
    If {
        test: Place,
        consequent: BlockId,
        alternate: BlockId,
        fallthrough: BlockId,
    },
    Branch {
        test: Place,
        consequent: BlockId,
        alternate: BlockId,
    },
    Switch {
        test: Place,
        cases: Vec<SwitchCase>,
        fallthrough: BlockId,
    },
    Return {
        value: Place,
        /// Aliasing effects for the return statement, computed by inference.
        /// Upstream: `effects` field on `ReturnTerminal` in `HIR.ts`.
        effects: Option<Vec<AliasingEffect>>,
    },
    Throw {
        value: Place,
    },
    For {
        init: BlockId,
        test: BlockId,
        update: Option<BlockId>,
        body: BlockId,
        fallthrough: BlockId,
    },
    ForOf {
        init: BlockId,
        test: BlockId,
        body: BlockId,
        fallthrough: BlockId,
    },
    ForIn {
        init: BlockId,
        test: BlockId,
        body: BlockId,
        fallthrough: BlockId,
    },
    DoWhile {
        body: BlockId,
        test: BlockId,
        fallthrough: BlockId,
    },
    While {
        test: BlockId,
        body: BlockId,
        fallthrough: BlockId,
    },
    Logical {
        operator: LogicalOp,
        left: BlockId,
        right: BlockId,
        fallthrough: BlockId,
        /// The place that receives the logical expression result value.
        result: Option<Place>,
    },
    Ternary {
        test: Place,
        consequent: BlockId,
        alternate: BlockId,
        fallthrough: BlockId,
        /// The place that receives the ternary result value.
        /// Set during HIR building; never renamed by SSA (since it's never a def).
        /// Used by build_reactive_function to emit let/assign for the ternary result.
        result: Option<Place>,
    },
    Optional {
        test: Place,
        consequent: BlockId,
        fallthrough: BlockId,
    },
    Sequence {
        blocks: Vec<BlockId>,
        fallthrough: BlockId,
    },
    Label {
        block: BlockId,
        fallthrough: BlockId,
        label: u32,
    },
    MaybeThrow {
        continuation: BlockId,
        handler: BlockId,
        /// Aliasing effects for the maybe-throw terminal, computed by inference.
        /// Upstream: `effects` field on `MaybeThrowTerminal` in `HIR.ts`.
        effects: Option<Vec<AliasingEffect>>,
    },
    Try {
        block: BlockId,
        handler: BlockId,
        fallthrough: BlockId,
    },
    Scope {
        scope: ScopeId,
        block: BlockId,
        fallthrough: BlockId,
    },
    PrunedScope {
        scope: ScopeId,
        block: BlockId,
        fallthrough: BlockId,
    },
    Unreachable,
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub test: Option<Place>,
    pub block: BlockId,
}

// ---------------------------------------------------------------------------
// HIR Container types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReactFunctionType {
    Component,
    Hook,
    Other,
}

#[derive(Debug, Clone)]
pub enum Param {
    Identifier(Place),
    Spread(Place),
}

/// Upstream: `{ place: Place }` on `HIRFunction.returns`.
///
/// Wraps the return-value Place in a struct to match the upstream shape.
/// This allows future extension (e.g., adding return type annotations)
/// without changing the `HIRFunction` signature.
#[derive(Debug, Clone)]
pub struct FunctionReturns {
    pub place: Place,
}

#[derive(Debug, Clone)]
pub struct HIRFunction {
    pub loc: SourceLocation,
    pub id: Option<String>,
    pub fn_type: ReactFunctionType,
    pub params: Vec<Param>,
    pub returns: FunctionReturns,
    pub context: Vec<Place>,
    pub body: HIR,
    pub is_async: bool,
    pub is_generator: bool,
    pub directives: Vec<String>,
    /// Whether the original source was an arrow function expression.
    pub is_arrow: bool,
    /// Externally-visible aliasing effects of this function, computed by
    /// `InferMutationAliasingEffects` during `AnalyseFunctions`.
    ///
    /// Upstream: `aliasingEffects` field on `HIRFunction` in `HIR.ts`.
    /// `None` means effects have not been computed yet (pre-inference).
    /// `Some(vec)` contains the effects visible to callers.
    pub aliasing_effects: Option<Vec<AliasingEffect>>,
}

#[derive(Debug, Clone)]
pub struct HIR {
    pub entry: BlockId,
    /// Ordered mapping of block IDs to basic blocks.
    pub blocks: Vec<(BlockId, BasicBlock)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockKind {
    Block,
    Value,
    Loop,
    Sequence,
    Catch,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub kind: BlockKind,
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminal: Terminal,
    pub preds: Vec<BlockId>,
    pub phis: Vec<Phi>,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub id: InstructionId,
    pub lvalue: Place,
    pub value: InstructionValue,
    pub loc: SourceLocation,
    pub effects: Option<Vec<AliasingEffect>>,
}

#[derive(Debug, Clone)]
pub struct Phi {
    pub id: InstructionId,
    pub place: Place,
    pub operands: Vec<(BlockId, Place)>,
}

// ---------------------------------------------------------------------------
// AliasingEffect
// ---------------------------------------------------------------------------

/// Aliasing effects describe how an instruction affects the abstract heap model.
///
/// Upstream: `AliasingEffect` discriminated union in `Inference/AliasingEffects.ts`.
/// Each variant corresponds to a `kind` discriminant in the upstream TypeScript.
#[derive(Debug, Clone)]
pub enum AliasingEffect {
    // --- Value creation effects ---
    Create {
        into: Place,
        value: ValueKind,
        reason: ValueReason,
    },
    CreateFrom {
        from: Place,
        into: Place,
    },
    CreateFunction {
        captures: Vec<Place>,
        /// The place holding the function value (instruction lvalue).
        // DIVERGENCE: Upstream stores a reference to the FunctionExpression/ObjectMethod
        // instruction value node here. We store the lvalue Place instead, which is
        // sufficient for our current analysis passes.
        function: Place,
        into: Place,
    },

    // --- Data flow effects ---
    Assign {
        from: Place,
        into: Place,
    },
    Alias {
        from: Place,
        into: Place,
    },
    MaybeAlias {
        from: Place,
        into: Place,
    },
    Capture {
        from: Place,
        into: Place,
    },
    ImmutableCapture {
        from: Place,
        into: Place,
    },

    // --- Function call effects ---
    Apply {
        receiver: Place,
        function: Place,
        /// Whether calling this function may mutate the function value itself
        /// (e.g. a closure that captures and mutates its own scope).
        mutates_function: bool,
        args: Vec<Place>,
        into: Place,
        signature: Option<FunctionSignature>,
        loc: SourceLocation,
    },

    // --- State change effects ---
    Freeze {
        value: Place,
        reason: ValueReason,
    },
    Mutate {
        value: Place,
        reason: Option<MutationReason>,
    },
    MutateConditionally {
        value: Place,
    },
    MutateTransitive {
        value: Place,
    },
    MutateTransitiveConditionally {
        value: Place,
    },

    // --- JSX access ---
    Render {
        place: Place,
    },

    // --- Error effects ---
    MutateFrozen {
        place: Place,
        error: String,
    },
    MutateGlobal {
        place: Place,
        error: String,
    },
    Impure {
        place: Place,
        error: String,
    },
}

// ---------------------------------------------------------------------------
// FunctionSignature
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<ParamEffect>,
    pub return_effect: Effect,
    pub callee_effect: Effect,
    /// When true, the function only mutates operands if all arguments are
    /// already mutable. If all args are frozen/immutable, the call is
    /// treated as a pure read with an immutable return.
    ///
    /// Upstream: `mutableOnlyIfOperandsAreMutable` on `FunctionSignature`.
    pub mutable_only_if_operands_are_mutable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamEffect {
    pub effect: Effect,
    pub alias_to_return: bool,
}

// ---------------------------------------------------------------------------
// AliasingSignature
// ---------------------------------------------------------------------------

/// A resolved aliasing signature for a function call.
///
/// Upstream: `AliasingSignature` in `Inference/AliasingEffects.ts`.
///
/// This maps abstract parameter identifiers to concrete effects. When the
/// inference pass encounters an `Apply` effect with a known signature, it
/// substitutes the abstract identifiers with the actual arguments to produce
/// concrete effects.
#[derive(Debug, Clone)]
pub struct AliasingSignature {
    /// Abstract identifier for the receiver (`this` / first arg for methods).
    pub receiver: IdentifierId,
    /// Abstract identifiers for positional parameters.
    pub params: Vec<IdentifierId>,
    /// Abstract identifier for the rest parameter, if any.
    pub rest: Option<IdentifierId>,
    /// Abstract identifier for the return value.
    pub returns: IdentifierId,
    /// The effects expressed in terms of the abstract identifiers above.
    pub effects: Vec<AliasingEffect>,
    /// Temporary places used by the signature's effect computation.
    pub temporaries: Vec<Place>,
}

// ---------------------------------------------------------------------------
// AliasingSignatureConfig (string-based configuration format)
// ---------------------------------------------------------------------------

/// String-based configuration for aliasing signatures of built-in functions.
///
/// Upstream: `AliasingSignatureConfig` in `HIR/TypeSchema.ts`.
///
/// This is the serializable format used to define aliasing behavior for
/// built-in functions (Array.push, Object.assign, React hooks, etc.).
/// At initialization time, these configs are parsed into `AliasingSignature`
/// values by allocating fresh `IdentifierId`s for each named parameter.
#[derive(Debug, Clone)]
pub struct AliasingSignatureConfig {
    /// Name for the receiver parameter (e.g. "receiver").
    pub receiver: String,
    /// Names for positional parameters (e.g. `["arg0", "arg1"]`).
    pub params: Vec<String>,
    /// Name for the rest parameter, if any.
    pub rest: Option<String>,
    /// Name for the return value (e.g. "returns").
    pub returns: String,
    /// Effects expressed using the string parameter names above.
    pub effects: Vec<AliasingEffectConfig>,
    /// Names for temporary values used in the effect computation.
    pub temporaries: Vec<String>,
}

/// String-based configuration for a single aliasing effect.
///
/// Upstream: `AliasingEffectConfig` in `HIR/TypeSchema.ts`.
/// Each variant mirrors an `AliasingEffect` variant but uses string names
/// instead of `Place` / `IdentifierId` for referencing parameters.
#[derive(Debug, Clone)]
pub enum AliasingEffectConfig {
    Freeze {
        value: String,
        reason: ValueReason,
    },
    Create {
        into: String,
        value: ValueKind,
        reason: ValueReason,
    },
    CreateFrom {
        from: String,
        into: String,
    },
    Assign {
        from: String,
        into: String,
    },
    Alias {
        from: String,
        into: String,
    },
    Capture {
        from: String,
        into: String,
    },
    ImmutableCapture {
        from: String,
        into: String,
    },
    Impure {
        place: String,
    },
    Mutate {
        value: String,
    },
    MutateTransitiveConditionally {
        value: String,
    },
    Apply {
        receiver: String,
        function: String,
        mutates_function: bool,
        args: Vec<ApplyArgConfig>,
        into: String,
    },
}

/// Configuration for an argument in an `Apply` effect config.
///
/// Upstream: `ApplyArgConfig` in `HIR/TypeSchema.ts`.
#[derive(Debug, Clone)]
pub enum ApplyArgConfig {
    /// A named parameter reference.
    Place(String),
    /// A spread of a named parameter.
    Spread(String),
    /// A hole (skipped argument position).
    Hole,
}

// ---------------------------------------------------------------------------
// Reactive Function IR types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ReactiveFunction {
    pub loc: SourceLocation,
    pub id: Option<String>,
    pub params: Vec<Param>,
    pub body: ReactiveBlock,
    pub directives: Vec<String>,
    /// Whether the original source function was an arrow function expression.
    /// Used by codegen to preserve `() => {}` vs `function() {}` syntax.
    pub is_arrow: bool,
    /// Whether the original source function was async.
    /// Used by codegen to emit `async function` or `async () =>`.
    pub is_async: bool,
    /// Whether the original source function was a generator.
    /// Used by codegen to emit `function*`.
    pub is_generator: bool,
}

#[derive(Debug, Clone)]
pub struct ReactiveBlock {
    pub instructions: Vec<ReactiveInstruction>,
}

#[derive(Debug, Clone)]
pub enum ReactiveInstruction {
    Instruction(Instruction),
    Terminal(ReactiveTerminal),
    Scope(ReactiveScopeBlock),
}

#[derive(Debug, Clone)]
pub struct ReactiveScopeBlock {
    pub scope: ReactiveScope,
    pub instructions: ReactiveBlock,
}

#[derive(Debug, Clone)]
pub enum ReactiveTerminal {
    If {
        test: Place,
        consequent: ReactiveBlock,
        alternate: ReactiveBlock,
        id: BlockId,
    },
    Switch {
        test: Place,
        cases: Vec<(Option<Place>, ReactiveBlock)>,
        id: BlockId,
    },
    For {
        init: ReactiveBlock,
        test: ReactiveBlock,
        update: Option<ReactiveBlock>,
        body: ReactiveBlock,
        id: BlockId,
    },
    ForOf {
        init: ReactiveBlock,
        test: ReactiveBlock,
        body: ReactiveBlock,
        id: BlockId,
    },
    ForIn {
        init: ReactiveBlock,
        test: ReactiveBlock,
        body: ReactiveBlock,
        id: BlockId,
    },
    While {
        test: ReactiveBlock,
        /// The condition Place extracted from the Branch terminal.
        /// When present, codegen emits `while (<condition>)` instead of `while (true)`.
        condition: Option<Place>,
        body: ReactiveBlock,
        id: BlockId,
    },
    DoWhile {
        body: ReactiveBlock,
        test: ReactiveBlock,
        /// The condition Place extracted from the Branch terminal.
        /// When present, codegen emits `do { } while (<condition>)` instead of `while (true)`.
        condition: Option<Place>,
        id: BlockId,
    },
    Label {
        block: ReactiveBlock,
        id: BlockId,
        label: u32,
    },
    Try {
        block: ReactiveBlock,
        handler: ReactiveBlock,
        id: BlockId,
    },
    Return {
        value: Place,
        id: BlockId,
    },
    Throw {
        value: Place,
        id: BlockId,
    },
    /// Logical expression (&&, ||, ??).
    ///
    /// The left-side instructions are already emitted before this terminal
    /// (they always execute and store the left value into `result`).
    /// The `right` block is conditional: it executes and overwrites `result`
    /// only when the operator's short-circuit condition is not met.
    Logical {
        operator: LogicalOp,
        right: ReactiveBlock,
        /// The place holding the logical expression result. Before this
        /// terminal, it contains the left value; the right block may
        /// overwrite it.
        result: Option<Place>,
        id: BlockId,
    },
    /// Explicit `continue` statement inside a loop body.
    Continue {
        id: BlockId,
    },
    /// Explicit `break` statement inside a loop body.
    Break {
        id: BlockId,
    },
}

#[derive(Debug, Clone)]
pub struct ReactiveScope {
    pub id: ScopeId,
    pub range: MutableRange,
    pub dependencies: Vec<ReactiveScopeDependency>,
    pub declarations: Vec<(IdentifierId, ReactiveScopeDeclaration)>,
    pub reassignments: Vec<Identifier>,
    pub early_return_value: Option<EarlyReturnValue>,
    pub merged: Vec<ScopeId>,
    pub loc: SourceLocation,
    /// Scope was created for non-reactive allocating expressions (JSX, objects,
    /// arrays, etc.) that need sentinel-based caching. When true and
    /// `dependencies` is empty, codegen emits `Symbol.for("react.memo_cache_sentinel")`
    /// instead of dependency checks.
    pub is_allocating: bool,
}

#[derive(Debug, Clone)]
pub struct ReactiveScopeDependency {
    pub identifier: Identifier,
    pub reactive: bool,
    pub path: Vec<DependencyPathEntry>,
}

#[derive(Debug, Clone)]
pub struct ReactiveScopeDeclaration {
    pub identifier: Identifier,
    pub scope: ScopeId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyPathEntry {
    pub property: String,
    pub optional: bool,
}

/// A source dependency extracted from useMemo/useCallback dep array AST.
/// Mirrors upstream ManualMemoDependency.
#[derive(Debug, Clone)]
pub struct ManualMemoDependency {
    pub root: ManualMemoDependencyRoot,
    /// Property path from the root, e.g. `[{property: "y", optional: false}]`
    /// for `x.y`.
    pub path: Vec<DependencyPathEntry>,
}

/// Root of a manual memo dependency.
#[derive(Debug, Clone)]
pub enum ManualMemoDependencyRoot {
    /// A named local variable.
    NamedLocal { name: String },
    /// A global variable reference (e.g. `Math`, `Object`).
    Global { name: String },
}

#[derive(Debug, Clone)]
pub struct EarlyReturnValue {
    pub value: Place,
    pub loc: SourceLocation,
}

// ---------------------------------------------------------------------------
// ID Generator
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct IdGenerator {
    next_block: u32,
    next_scope: u32,
    next_identifier: u32,
    next_declaration: u32,
    next_instruction: u32,
    next_type: u32,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self {
            next_block: 0,
            next_scope: 0,
            next_identifier: 0,
            next_declaration: 0,
            next_instruction: 0,
            next_type: 0,
        }
    }

    pub fn next_block_id(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }

    pub fn next_scope_id(&mut self) -> ScopeId {
        let id = ScopeId(self.next_scope);
        self.next_scope += 1;
        id
    }

    pub fn next_identifier_id(&mut self) -> IdentifierId {
        let id = IdentifierId(self.next_identifier);
        self.next_identifier += 1;
        id
    }

    pub fn next_declaration_id(&mut self) -> DeclarationId {
        let id = DeclarationId(self.next_declaration);
        self.next_declaration += 1;
        id
    }

    pub fn next_instruction_id(&mut self) -> InstructionId {
        let id = InstructionId(self.next_instruction);
        self.next_instruction += 1;
        id
    }

    pub fn next_type_id(&mut self) -> TypeId {
        let id = TypeId(self.next_type);
        self.next_type += 1;
        id
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Compile-time size assertions – prevent accidental size regressions.
// Limits are set ~20% above current sizes, rounded to power-of-2 boundaries.
// If a variant is added that pushes the size past the limit, this will fail
// at compile time, signalling that the change should be reviewed for impact.
// ---------------------------------------------------------------------------
const _: () = assert!(std::mem::size_of::<InstructionValue>() <= 264);
const _: () = assert!(std::mem::size_of::<Terminal>() <= 192);
const _: () = assert!(std::mem::size_of::<Place>() <= 128);
const _: () = assert!(std::mem::size_of::<Instruction>() <= 512);
