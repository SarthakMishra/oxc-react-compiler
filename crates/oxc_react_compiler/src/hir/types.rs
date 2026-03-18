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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueReason {
    KnownValue,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FreezeReason {
    FrozenByBinding,
    FrozenByValue,
    Other,
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
    },
    MethodCall {
        receiver: Place,
        property: String,
        args: Vec<Place>,
    },
    NewExpression {
        callee: Place,
        args: Vec<Place>,
    },

    // Property access
    PropertyLoad {
        object: Place,
        property: String,
    },
    PropertyStore {
        object: Place,
        property: String,
        value: Place,
    },
    ComputedLoad {
        object: Place,
        property: Place,
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

#[derive(Debug, Clone)]
pub struct HIRFunction {
    pub loc: SourceLocation,
    pub id: Option<String>,
    pub fn_type: ReactFunctionType,
    pub params: Vec<Param>,
    pub returns: Place,
    pub context: Vec<Place>,
    pub body: HIR,
    pub is_async: bool,
    pub is_generator: bool,
    pub directives: Vec<String>,
    /// Whether the original source was an arrow function expression.
    pub is_arrow: bool,
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

#[derive(Debug, Clone)]
pub enum AliasingEffect {
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
        function: Place,
        into: Place,
    },
    Apply {
        receiver: Place,
        function: Place,
        args: Vec<Place>,
        into: Place,
        signature: Option<FunctionSignature>,
    },
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
    Mutate {
        value: Place,
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
    Freeze {
        value: Place,
        reason: FreezeReason,
    },
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
    Render {
        place: Place,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamEffect {
    pub effect: Effect,
    pub alias_to_return: bool,
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
        body: ReactiveBlock,
        id: BlockId,
    },
    DoWhile {
        body: ReactiveBlock,
        test: ReactiveBlock,
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
