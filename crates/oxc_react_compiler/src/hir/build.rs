//! OXC AST → HIR lowering (replaces upstream BuildHIR.ts)
//!
//! Converts OXC AST nodes into HIR instructions and basic blocks.
//! Every expression is flattened into temporaries, and control flow
//! is lowered into basic blocks with terminals.

#![allow(dead_code)]

use oxc_ast::ast::{
    self as ast, Argument, ArrayExpressionElement, AssignmentTarget, BindingPattern, Expression,
    ForStatementInit, ForStatementLeft, JSXAttributeItem, JSXAttributeName, JSXAttributeValue,
    JSXChild, JSXElementName, JSXExpression, JSXMemberExpressionObject, ObjectPropertyKind,
    PropertyKey, SimpleAssignmentTarget, Statement, VariableDeclarationKind,
};
use oxc_span::Span;
use oxc_syntax::operator::{
    AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator, UpdateOperator,
};
use rustc_hash::{FxHashMap, FxHashSet};

use super::environment::{Environment, EnvironmentConfig};
use super::types::*;

// ---------------------------------------------------------------------------
// Operator mapping helpers
// ---------------------------------------------------------------------------

fn map_binary_op(op: BinaryOperator) -> BinaryOp {
    match op {
        BinaryOperator::Addition => BinaryOp::Add,
        BinaryOperator::Subtraction => BinaryOp::Sub,
        BinaryOperator::Multiplication => BinaryOp::Mul,
        BinaryOperator::Division => BinaryOp::Div,
        BinaryOperator::Remainder => BinaryOp::Mod,
        BinaryOperator::Exponential => BinaryOp::Exp,
        BinaryOperator::BitwiseAnd => BinaryOp::BitwiseAnd,
        BinaryOperator::BitwiseOR => BinaryOp::BitwiseOr,
        BinaryOperator::BitwiseXOR => BinaryOp::BitwiseXor,
        BinaryOperator::ShiftLeft => BinaryOp::ShiftLeft,
        BinaryOperator::ShiftRight => BinaryOp::ShiftRight,
        BinaryOperator::ShiftRightZeroFill => BinaryOp::UnsignedShiftRight,
        BinaryOperator::Equality => BinaryOp::EqEq,
        BinaryOperator::Inequality => BinaryOp::NotEq,
        BinaryOperator::StrictEquality => BinaryOp::StrictEq,
        BinaryOperator::StrictInequality => BinaryOp::StrictNotEq,
        BinaryOperator::LessThan => BinaryOp::Lt,
        BinaryOperator::LessEqualThan => BinaryOp::LtEq,
        BinaryOperator::GreaterThan => BinaryOp::Gt,
        BinaryOperator::GreaterEqualThan => BinaryOp::GtEq,
        BinaryOperator::In => BinaryOp::In,
        BinaryOperator::Instanceof => BinaryOp::InstanceOf,
    }
}

fn map_unary_op(op: UnaryOperator) -> UnaryOp {
    match op {
        UnaryOperator::UnaryNegation => UnaryOp::Minus,
        UnaryOperator::UnaryPlus => UnaryOp::Plus,
        UnaryOperator::LogicalNot => UnaryOp::Not,
        UnaryOperator::BitwiseNot => UnaryOp::BitwiseNot,
        UnaryOperator::Typeof => UnaryOp::TypeOf,
        UnaryOperator::Void => UnaryOp::Void,
        UnaryOperator::Delete => UnaryOp::Delete,
    }
}

fn map_update_op(op: UpdateOperator) -> UpdateOp {
    match op {
        UpdateOperator::Increment => UpdateOp::Increment,
        UpdateOperator::Decrement => UpdateOp::Decrement,
    }
}

fn map_logical_op(op: LogicalOperator) -> LogicalOp {
    match op {
        LogicalOperator::And => LogicalOp::And,
        LogicalOperator::Or => LogicalOp::Or,
        LogicalOperator::Coalesce => LogicalOp::NullishCoalescing,
    }
}

/// Map a compound assignment operator (e.g. `+=`) to its corresponding binary op.
/// Returns `None` for plain `=`.
fn compound_assignment_to_binary(op: AssignmentOperator) -> Option<BinaryOp> {
    match op {
        AssignmentOperator::Assign => None,
        AssignmentOperator::Addition => Some(BinaryOp::Add),
        AssignmentOperator::Subtraction => Some(BinaryOp::Sub),
        AssignmentOperator::Multiplication => Some(BinaryOp::Mul),
        AssignmentOperator::Division => Some(BinaryOp::Div),
        AssignmentOperator::Remainder => Some(BinaryOp::Mod),
        AssignmentOperator::Exponential => Some(BinaryOp::Exp),
        AssignmentOperator::ShiftLeft => Some(BinaryOp::ShiftLeft),
        AssignmentOperator::ShiftRight => Some(BinaryOp::ShiftRight),
        AssignmentOperator::ShiftRightZeroFill => Some(BinaryOp::UnsignedShiftRight),
        AssignmentOperator::BitwiseOR => Some(BinaryOp::BitwiseOr),
        AssignmentOperator::BitwiseXOR => Some(BinaryOp::BitwiseXor),
        AssignmentOperator::BitwiseAnd => Some(BinaryOp::BitwiseAnd),
        AssignmentOperator::LogicalOr => None, // handled as logical
        AssignmentOperator::LogicalAnd => None, // handled as logical
        AssignmentOperator::LogicalNullish => None, // handled as logical
    }
}

fn map_var_kind(kind: VariableDeclarationKind) -> InstructionKind {
    match kind {
        VariableDeclarationKind::Var => InstructionKind::Var,
        VariableDeclarationKind::Let => InstructionKind::Let,
        VariableDeclarationKind::Const => InstructionKind::Const,
        VariableDeclarationKind::Using | VariableDeclarationKind::AwaitUsing => {
            InstructionKind::Const
        }
    }
}

// ---------------------------------------------------------------------------
// HIRBuilder
// ---------------------------------------------------------------------------

/// The main builder that lowers OXC AST into HIR blocks and instructions.
pub struct HIRBuilder {
    pub env: Environment,

    /// All basic blocks produced during lowering, in order.
    blocks: Vec<(BlockId, BasicBlock)>,

    /// The block we are currently emitting instructions into.
    current_block: BlockId,

    /// Stack of break targets (innermost last).
    break_targets: Vec<BlockId>,

    /// Stack of continue targets (innermost last).
    continue_targets: Vec<BlockId>,

    /// Label name → (break_target, continue_target).
    label_map: FxHashMap<String, (BlockId, BlockId)>,

    /// Monotonically increasing label counter for the `Label` terminal.
    next_label: u32,

    /// Set of variable names that refer to context (captured from outer scope).
    /// When an identifier in this set is loaded, we emit `LoadContext` instead of `LoadLocal`.
    context_vars: FxHashSet<String>,

    /// Monotonically increasing ID for manual memoization markers (useMemo/useCallback).
    next_memo_id: u32,
}

impl HIRBuilder {
    // ------------------------------------------------------------------
    // Construction
    // ------------------------------------------------------------------

    pub fn new(config: EnvironmentConfig) -> Self {
        let mut env = Environment::new(config);
        let entry_id = env.id_generator.next_block_id();

        let entry_block = BasicBlock {
            kind: BlockKind::Block,
            id: entry_id,
            instructions: Vec::new(),
            terminal: Terminal::Unreachable,
            preds: Vec::new(),
            phis: Vec::new(),
        };

        Self {
            env,
            blocks: vec![(entry_id, entry_block)],
            current_block: entry_id,
            break_targets: Vec::new(),
            continue_targets: Vec::new(),
            label_map: FxHashMap::default(),
            next_label: 0,
            context_vars: FxHashSet::default(),
            next_memo_id: 0,
        }
    }

    /// Track context variables (captured from outer scope).
    /// Call this when building a nested function to set up context tracking.
    fn setup_context_variables(&mut self, outer_scope_vars: &[String]) {
        self.context_vars = outer_scope_vars.iter().cloned().collect();
    }

    // ------------------------------------------------------------------
    // Block management
    // ------------------------------------------------------------------

    /// Create a new empty basic block and return its ID.
    fn new_block(&mut self, kind: BlockKind) -> BlockId {
        let id = self.env.id_generator.next_block_id();
        let block = BasicBlock {
            kind,
            id,
            instructions: Vec::new(),
            terminal: Terminal::Unreachable,
            preds: Vec::new(),
            phis: Vec::new(),
        };
        self.blocks.push((id, block));
        id
    }

    /// Switch the current emission target to a different block.
    fn switch_block(&mut self, block_id: BlockId) {
        self.current_block = block_id;
    }

    /// Get a mutable reference to the current basic block.
    fn current_block_mut(&mut self) -> &mut BasicBlock {
        let id = self.current_block;
        self.blocks
            .iter_mut()
            .find(|(bid, _)| *bid == id)
            .map(|(_, b)| b)
            .expect("current block must exist")
    }

    // ------------------------------------------------------------------
    // Identifier / Place helpers
    // ------------------------------------------------------------------

    /// Create a fresh temporary place (unnamed identifier).
    fn make_temp(&mut self, loc: Span) -> Place {
        let id = self.env.id_generator.next_identifier_id();
        Place {
            identifier: Identifier {
                id,
                declaration_id: None,
                name: None,
                mutable_range: MutableRange {
                    start: InstructionId(0),
                    end: InstructionId(0),
                },
                scope: None,
                type_: Type::default(),
                loc,
            },
            effect: Effect::Unknown,
            reactive: false,
            loc,
        }
    }

    /// Create a named place for a local binding.
    fn make_named_place(&mut self, name: &str, loc: Span) -> Place {
        let id = self.env.id_generator.next_identifier_id();
        let decl_id = self.env.id_generator.next_declaration_id();
        Place {
            identifier: Identifier {
                id,
                declaration_id: Some(decl_id),
                name: Some(name.to_string()),
                mutable_range: MutableRange {
                    start: InstructionId(0),
                    end: InstructionId(0),
                },
                scope: None,
                type_: Type::default(),
                loc,
            },
            effect: Effect::Unknown,
            reactive: false,
            loc,
        }
    }

    // ------------------------------------------------------------------
    // Emit helpers
    // ------------------------------------------------------------------

    /// Emit an instruction into the current block and return its lvalue place.
    fn emit(&mut self, value: InstructionValue, loc: Span) -> Place {
        let instr_id = self.env.id_generator.next_instruction_id();
        let lvalue = self.make_temp(loc);
        let instr = Instruction {
            id: instr_id,
            lvalue: lvalue.clone(),
            value,
            loc,
            effects: None,
        };
        self.current_block_mut().instructions.push(instr);
        lvalue
    }

    /// Set the terminal of the current block.
    fn emit_terminal(&mut self, terminal: Terminal) {
        self.current_block_mut().terminal = terminal;
    }

    // ------------------------------------------------------------------
    // Entry point: build a function
    // ------------------------------------------------------------------

    /// Lower an OXC `Function` AST node into an `HIRFunction`.
    pub fn build_function(
        mut self,
        func: &ast::Function<'_>,
        fn_type: ReactFunctionType,
    ) -> HIRFunction {
        let loc = func.span;
        let id = func.id.as_ref().map(|id| id.name.to_string());
        let is_async = func.r#async;
        let is_generator = func.generator;

        // Lower parameters
        let params = self.lower_formal_params(&func.params);

        // Collect directives
        let directives = func
            .body
            .as_ref()
            .map(|body| {
                body.directives
                    .iter()
                    .map(|d| d.directive.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Lower body statements
        if let Some(body) = &func.body {
            for stmt in body.statements.iter() {
                self.lower_statement(stmt);
            }
        }

        // Ensure the last block has a return terminal if it's still unreachable.
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            let undef = self.emit(
                InstructionValue::Primitive {
                    value: Primitive::Undefined,
                },
                loc,
            );
            self.emit_terminal(Terminal::Return { value: undef });
        }

        let returns = self.make_temp(loc);
        let entry = self.blocks.first().map(|(id, _)| *id).unwrap();

        HIRFunction {
            loc,
            id,
            fn_type,
            params,
            returns,
            context: Vec::new(),
            body: HIR {
                entry,
                blocks: self.blocks,
            },
            is_async,
            is_generator,
            directives,
        }
    }

    /// Lower a top-level arrow function expression into an `HIRFunction` for compilation.
    ///
    /// Unlike `build_arrow` (which handles nested arrows within a function body),
    /// this consumes the builder and produces a standalone HIRFunction suitable for
    /// the compilation pipeline.
    pub fn build_arrow_function(
        mut self,
        arrow: &ast::ArrowFunctionExpression<'_>,
        id: Option<String>,
        fn_type: ReactFunctionType,
    ) -> HIRFunction {
        let loc = arrow.span;
        let params = self.lower_formal_params(&arrow.params);

        let directives = arrow
            .body
            .directives
            .iter()
            .map(|d| d.directive.to_string())
            .collect::<Vec<_>>();

        if arrow.expression {
            if let Some(stmt) = arrow.body.statements.first() {
                if let Statement::ExpressionStatement(expr_stmt) = stmt {
                    let val = self.lower_expression(&expr_stmt.expression);
                    self.emit_terminal(Terminal::Return { value: val });
                } else {
                    self.lower_statement(stmt);
                }
            }
        } else {
            for stmt in arrow.body.statements.iter() {
                self.lower_statement(stmt);
            }
        }

        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            let undef = self.emit(
                InstructionValue::Primitive {
                    value: Primitive::Undefined,
                },
                loc,
            );
            self.emit_terminal(Terminal::Return { value: undef });
        }

        let returns = self.make_temp(loc);
        let entry = self.blocks.first().map(|(id, _)| *id).unwrap();

        HIRFunction {
            loc,
            id,
            fn_type,
            params,
            returns,
            context: Vec::new(),
            body: HIR {
                entry,
                blocks: self.blocks,
            },
            is_async: arrow.r#async,
            is_generator: false,
            directives,
        }
    }

    /// Lower an arrow function expression into an `HIRFunction`.
    fn build_arrow(&mut self, arrow: &ast::ArrowFunctionExpression<'_>) -> HIRFunction {
        let mut inner = HIRBuilder::new(EnvironmentConfig::default());
        let loc = arrow.span;

        let params = inner.lower_formal_params(&arrow.params);

        let directives = arrow
            .body
            .directives
            .iter()
            .map(|d| d.directive.to_string())
            .collect::<Vec<_>>();

        if arrow.expression {
            // Arrow with expression body: `() => expr`
            // The body will have a single expression statement or a return.
            // OXC always wraps in FunctionBody, but if `expression` is true,
            // the single statement is actually the return value.
            if let Some(stmt) = arrow.body.statements.first() {
                if let Statement::ExpressionStatement(expr_stmt) = stmt {
                    let val = inner.lower_expression(&expr_stmt.expression);
                    inner.emit_terminal(Terminal::Return { value: val });
                } else {
                    inner.lower_statement(stmt);
                }
            }
        } else {
            for stmt in arrow.body.statements.iter() {
                inner.lower_statement(stmt);
            }
        }

        // Implicit undefined return if needed
        if matches!(inner.current_block_mut().terminal, Terminal::Unreachable) {
            let undef = inner.emit(
                InstructionValue::Primitive {
                    value: Primitive::Undefined,
                },
                loc,
            );
            inner.emit_terminal(Terminal::Return { value: undef });
        }

        let returns = inner.make_temp(loc);
        let entry = inner.blocks.first().map(|(id, _)| *id).unwrap();

        HIRFunction {
            loc,
            id: None,
            fn_type: ReactFunctionType::Other,
            params,
            returns,
            context: Vec::new(),
            body: HIR {
                entry,
                blocks: inner.blocks,
            },
            is_async: arrow.r#async,
            is_generator: false,
            directives,
        }
    }

    // ------------------------------------------------------------------
    // Parameter lowering
    // ------------------------------------------------------------------

    fn lower_formal_params(&mut self, params: &ast::FormalParameters<'_>) -> Vec<Param> {
        let mut result = Vec::new();
        for param in params.items.iter() {
            match &param.pattern {
                BindingPattern::BindingIdentifier(id) => {
                    let place = self.make_named_place(&id.name, id.span);
                    result.push(Param::Identifier(place));
                }
                _ => {
                    // For destructured params, create a temp and emit destructure later.
                    let place = self.make_temp(param.span);
                    result.push(Param::Identifier(place));
                }
            }
        }
        if let Some(rest) = &params.rest {
            match &rest.rest.argument {
                BindingPattern::BindingIdentifier(id) => {
                    let place = self.make_named_place(&id.name, id.span);
                    result.push(Param::Spread(place));
                }
                _ => {
                    let place = self.make_temp(rest.span);
                    result.push(Param::Spread(place));
                }
            }
        }
        result
    }

    // ------------------------------------------------------------------
    // Statement lowering
    // ------------------------------------------------------------------

    fn lower_statement(&mut self, stmt: &Statement<'_>) {
        match stmt {
            Statement::BlockStatement(block) => {
                for s in block.body.iter() {
                    self.lower_statement(s);
                }
            }

            Statement::EmptyStatement(_) => {
                // no-op
            }

            Statement::ExpressionStatement(expr_stmt) => {
                let _ = self.lower_expression(&expr_stmt.expression);
            }

            Statement::VariableDeclaration(decl) => {
                self.lower_variable_declaration(decl);
            }

            Statement::ReturnStatement(ret) => {
                let value = if let Some(arg) = &ret.argument {
                    self.lower_expression(arg)
                } else {
                    self.emit(
                        InstructionValue::Primitive {
                            value: Primitive::Undefined,
                        },
                        ret.span,
                    )
                };
                self.emit_terminal(Terminal::Return { value });
                // Create a new block for any unreachable code after return.
                let dead = self.new_block(BlockKind::Block);
                self.switch_block(dead);
            }

            Statement::ThrowStatement(throw) => {
                let value = self.lower_expression(&throw.argument);
                self.emit_terminal(Terminal::Throw { value });
                let dead = self.new_block(BlockKind::Block);
                self.switch_block(dead);
            }

            Statement::IfStatement(if_stmt) => {
                self.lower_if_statement(if_stmt);
            }

            Statement::SwitchStatement(switch) => {
                self.lower_switch_statement(switch);
            }

            Statement::ForStatement(for_stmt) => {
                self.lower_for_statement(for_stmt);
            }

            Statement::ForInStatement(for_in) => {
                self.lower_for_in_statement(for_in);
            }

            Statement::ForOfStatement(for_of) => {
                self.lower_for_of_statement(for_of);
            }

            Statement::WhileStatement(while_stmt) => {
                self.lower_while_statement(while_stmt);
            }

            Statement::DoWhileStatement(do_while) => {
                self.lower_do_while_statement(do_while);
            }

            Statement::TryStatement(try_stmt) => {
                self.lower_try_statement(try_stmt);
            }

            Statement::LabeledStatement(labeled) => {
                self.lower_labeled_statement(labeled);
            }

            Statement::BreakStatement(brk) => {
                self.lower_break_statement(brk);
            }

            Statement::ContinueStatement(cont) => {
                self.lower_continue_statement(cont);
            }

            Statement::FunctionDeclaration(func) => {
                self.lower_function_declaration(func);
            }

            Statement::DebuggerStatement(_) => {
                // Emit as unsupported node; debugger has no semantic effect for memoization.
                self.emit(
                    InstructionValue::UnsupportedNode {
                        node: "DebuggerStatement".to_string(),
                    },
                    stmt.span(),
                );
            }

            // Class declarations, TS declarations, module declarations, etc.
            _ => {
                self.emit(
                    InstructionValue::UnsupportedNode {
                        node: format!("Statement::{}", stmt_kind_name(stmt)),
                    },
                    stmt.span(),
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // Variable declarations
    // ------------------------------------------------------------------

    fn lower_variable_declaration(&mut self, decl: &ast::VariableDeclaration<'_>) {
        let kind = map_var_kind(decl.kind);
        for declarator in decl.declarations.iter() {
            self.lower_variable_declarator(declarator, kind);
        }
    }

    fn lower_variable_declarator(
        &mut self,
        decl: &ast::VariableDeclarator<'_>,
        kind: InstructionKind,
    ) {
        match &decl.id {
            BindingPattern::BindingIdentifier(id) => {
                let lvalue = self.make_named_place(&id.name, id.span);
                // Emit DeclareLocal
                self.emit(
                    InstructionValue::DeclareLocal {
                        lvalue: lvalue.clone(),
                        type_: kind,
                    },
                    id.span,
                );
                // If there's an initializer, lower it and store
                if let Some(init) = &decl.init {
                    let value = self.lower_expression(init);
                    self.emit(
                        InstructionValue::StoreLocal {
                            lvalue,
                            value,
                            type_: Some(kind),
                        },
                        decl.span,
                    );
                }
            }
            BindingPattern::ObjectPattern(obj_pat) => {
                // Lower initializer (required for destructuring)
                let value = if let Some(init) = &decl.init {
                    self.lower_expression(init)
                } else {
                    self.emit(
                        InstructionValue::Primitive {
                            value: Primitive::Undefined,
                        },
                        decl.span,
                    )
                };
                let pattern = self.lower_object_binding_pattern(obj_pat, kind);
                self.emit(
                    InstructionValue::Destructure {
                        lvalue_pattern: pattern,
                        value,
                    },
                    decl.span,
                );
            }
            BindingPattern::ArrayPattern(arr_pat) => {
                let value = if let Some(init) = &decl.init {
                    self.lower_expression(init)
                } else {
                    self.emit(
                        InstructionValue::Primitive {
                            value: Primitive::Undefined,
                        },
                        decl.span,
                    )
                };
                let pattern = self.lower_array_binding_pattern(arr_pat, kind);
                self.emit(
                    InstructionValue::Destructure {
                        lvalue_pattern: pattern,
                        value,
                    },
                    decl.span,
                );
            }
            BindingPattern::AssignmentPattern(assign_pat) => {
                // `let x = default_val` with destructure default -- unusual at top level
                // but handle gracefully
                let value = if let Some(init) = &decl.init {
                    self.lower_expression(init)
                } else {
                    self.lower_expression(&assign_pat.right)
                };
                self.lower_binding_pattern_assign(&assign_pat.left, value, kind, decl.span);
            }
        }
    }

    // ------------------------------------------------------------------
    // Destructuring patterns (binding)
    // ------------------------------------------------------------------

    fn lower_object_binding_pattern(
        &mut self,
        pat: &ast::ObjectPattern<'_>,
        kind: InstructionKind,
    ) -> DestructurePattern {
        let mut properties = Vec::new();
        for prop in pat.properties.iter() {
            let key = self.property_key_to_string(&prop.key);
            let target = self.lower_binding_pattern_to_target(&prop.value, kind);
            properties.push(DestructureObjectProperty {
                key,
                value: target,
                shorthand: prop.shorthand,
            });
        }
        let rest = pat.rest.as_ref().map(|r| match &r.argument {
            BindingPattern::BindingIdentifier(id) => {
                let place = self.make_named_place(&id.name, id.span);
                self.emit(
                    InstructionValue::DeclareLocal {
                        lvalue: place.clone(),
                        type_: kind,
                    },
                    id.span,
                );
                place
            }
            _ => self.make_temp(r.span),
        });
        DestructurePattern::Object { properties, rest }
    }

    fn lower_array_binding_pattern(
        &mut self,
        pat: &ast::ArrayPattern<'_>,
        kind: InstructionKind,
    ) -> DestructurePattern {
        let mut items = Vec::new();
        for elem in pat.elements.iter() {
            match elem {
                Some(binding) => {
                    let target = self.lower_binding_pattern_to_target(binding, kind);
                    items.push(DestructureArrayItem::Value(target));
                }
                None => {
                    items.push(DestructureArrayItem::Hole);
                }
            }
        }
        let rest = pat.rest.as_ref().map(|r| match &r.argument {
            BindingPattern::BindingIdentifier(id) => {
                let place = self.make_named_place(&id.name, id.span);
                self.emit(
                    InstructionValue::DeclareLocal {
                        lvalue: place.clone(),
                        type_: kind,
                    },
                    id.span,
                );
                place
            }
            _ => self.make_temp(r.span),
        });
        DestructurePattern::Array { items, rest }
    }

    fn lower_binding_pattern_to_target(
        &mut self,
        pat: &BindingPattern<'_>,
        kind: InstructionKind,
    ) -> DestructureTarget {
        match pat {
            BindingPattern::BindingIdentifier(id) => {
                let place = self.make_named_place(&id.name, id.span);
                self.emit(
                    InstructionValue::DeclareLocal {
                        lvalue: place.clone(),
                        type_: kind,
                    },
                    id.span,
                );
                DestructureTarget::Place(place)
            }
            BindingPattern::ObjectPattern(obj) => {
                let inner = self.lower_object_binding_pattern(obj, kind);
                DestructureTarget::Pattern(Box::new(inner))
            }
            BindingPattern::ArrayPattern(arr) => {
                let inner = self.lower_array_binding_pattern(arr, kind);
                DestructureTarget::Pattern(Box::new(inner))
            }
            BindingPattern::AssignmentPattern(assign) => {
                // Pattern with default: `{ x = 5 } = obj`
                // For now, treat as the inner pattern.
                self.lower_binding_pattern_to_target(&assign.left, kind)
            }
        }
    }

    fn lower_binding_pattern_assign(
        &mut self,
        pat: &BindingPattern<'_>,
        value: Place,
        kind: InstructionKind,
        loc: Span,
    ) {
        match pat {
            BindingPattern::BindingIdentifier(id) => {
                let lvalue = self.make_named_place(&id.name, id.span);
                self.emit(
                    InstructionValue::DeclareLocal {
                        lvalue: lvalue.clone(),
                        type_: kind,
                    },
                    id.span,
                );
                self.emit(
                    InstructionValue::StoreLocal {
                        lvalue,
                        value,
                        type_: Some(kind),
                    },
                    loc,
                );
            }
            _ => {
                self.emit(
                    InstructionValue::UnsupportedNode {
                        node: "ComplexAssignmentPattern".to_string(),
                    },
                    loc,
                );
            }
        }
    }

    fn property_key_to_string(&self, key: &PropertyKey<'_>) -> String {
        match key {
            PropertyKey::StaticIdentifier(id) => id.name.to_string(),
            PropertyKey::StringLiteral(s) => s.value.to_string(),
            PropertyKey::NumericLiteral(n) => n.value.to_string(),
            _ => "<computed>".to_string(),
        }
    }

    // ------------------------------------------------------------------
    // Control-flow statement lowering
    // ------------------------------------------------------------------

    fn lower_if_statement(&mut self, if_stmt: &ast::IfStatement<'_>) {
        let test = self.lower_expression(&if_stmt.test);

        let consequent_block = self.new_block(BlockKind::Block);
        let alternate_block = self.new_block(BlockKind::Block);
        let fallthrough = self.new_block(BlockKind::Block);

        self.emit_terminal(Terminal::If {
            test,
            consequent: consequent_block,
            alternate: alternate_block,
            fallthrough,
        });

        // Consequent
        self.switch_block(consequent_block);
        self.lower_statement(&if_stmt.consequent);
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            self.emit_terminal(Terminal::Goto { block: fallthrough });
        }

        // Alternate
        self.switch_block(alternate_block);
        if let Some(alt) = &if_stmt.alternate {
            self.lower_statement(alt);
        }
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            self.emit_terminal(Terminal::Goto { block: fallthrough });
        }

        self.switch_block(fallthrough);
    }

    fn lower_switch_statement(&mut self, switch: &ast::SwitchStatement<'_>) {
        let test = self.lower_expression(&switch.discriminant);
        let fallthrough = self.new_block(BlockKind::Block);

        let mut cases = Vec::new();
        let mut case_blocks = Vec::new();

        for case in switch.cases.iter() {
            let block_id = self.new_block(BlockKind::Block);
            let test_place = case.test.as_ref().map(|t| {
                self.switch_block(self.current_block);
                self.lower_expression(t)
            });
            cases.push(SwitchCase {
                test: test_place,
                block: block_id,
            });
            case_blocks.push((block_id, &case.consequent));
        }

        self.emit_terminal(Terminal::Switch {
            test,
            cases,
            fallthrough,
        });

        // Push break target
        self.break_targets.push(fallthrough);

        for (i, (block_id, stmts)) in case_blocks.iter().enumerate() {
            self.switch_block(*block_id);
            for s in stmts.iter() {
                self.lower_statement(s);
            }
            // Fall through to next case block if no explicit break/return
            if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
                let next = if i + 1 < case_blocks.len() {
                    case_blocks[i + 1].0
                } else {
                    fallthrough
                };
                self.emit_terminal(Terminal::Goto { block: next });
            }
        }

        self.break_targets.pop();
        self.switch_block(fallthrough);
    }

    fn lower_for_statement(&mut self, for_stmt: &ast::ForStatement<'_>) {
        let init_block = self.new_block(BlockKind::Block);
        let test_block = self.new_block(BlockKind::Value);
        let body_block = self.new_block(BlockKind::Loop);
        let update_block = self.new_block(BlockKind::Block);
        let fallthrough = self.new_block(BlockKind::Block);

        self.emit_terminal(Terminal::Goto { block: init_block });

        // Init
        self.switch_block(init_block);
        if let Some(init) = &for_stmt.init {
            match init {
                ForStatementInit::VariableDeclaration(decl) => {
                    self.lower_variable_declaration(decl);
                }
                _ => {
                    // Expression init
                    if let Some(expr) = for_init_as_expression(init) {
                        let _ = self.lower_expression(expr);
                    }
                }
            }
        }
        self.emit_terminal(Terminal::Goto { block: test_block });

        // Test
        self.switch_block(test_block);
        if let Some(test_expr) = &for_stmt.test {
            let test = self.lower_expression(test_expr);
            self.emit_terminal(Terminal::Branch {
                test,
                consequent: body_block,
                alternate: fallthrough,
            });
        } else {
            // Infinite loop (no test)
            self.emit_terminal(Terminal::Goto { block: body_block });
        }

        // Body
        self.break_targets.push(fallthrough);
        self.continue_targets.push(update_block);

        self.switch_block(body_block);
        self.lower_statement(&for_stmt.body);
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            self.emit_terminal(Terminal::Goto {
                block: update_block,
            });
        }

        self.continue_targets.pop();
        self.break_targets.pop();

        // Update
        self.switch_block(update_block);
        if let Some(update) = &for_stmt.update {
            let _ = self.lower_expression(update);
        }
        self.emit_terminal(Terminal::Goto { block: test_block });

        self.switch_block(fallthrough);
    }

    fn lower_for_in_statement(&mut self, for_in: &ast::ForInStatement<'_>) {
        let init_block = self.new_block(BlockKind::Block);
        let test_block = self.new_block(BlockKind::Value);
        let body_block = self.new_block(BlockKind::Loop);
        let fallthrough = self.new_block(BlockKind::Block);

        // Lower the collection expression first
        let collection = self.lower_expression(&for_in.right);
        self.emit_terminal(Terminal::Goto { block: init_block });

        // Init: emit NextPropertyOf
        self.switch_block(init_block);
        let next_prop = self.emit(
            InstructionValue::NextPropertyOf { value: collection },
            for_in.span,
        );
        self.lower_for_left(&for_in.left, next_prop, for_in.span);
        self.emit_terminal(Terminal::Goto { block: test_block });

        // Test
        self.switch_block(test_block);
        self.emit_terminal(Terminal::Goto { block: body_block });

        // Body
        self.break_targets.push(fallthrough);
        self.continue_targets.push(init_block);

        self.switch_block(body_block);
        self.lower_statement(&for_in.body);
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            self.emit_terminal(Terminal::Goto { block: init_block });
        }

        self.continue_targets.pop();
        self.break_targets.pop();

        self.switch_block(fallthrough);
    }

    fn lower_for_of_statement(&mut self, for_of: &ast::ForOfStatement<'_>) {
        let init_block = self.new_block(BlockKind::Block);
        let test_block = self.new_block(BlockKind::Value);
        let body_block = self.new_block(BlockKind::Loop);
        let fallthrough = self.new_block(BlockKind::Block);

        // Lower the collection
        let collection = self.lower_expression(&for_of.right);
        let iterator = self.emit(InstructionValue::GetIterator { collection }, for_of.span);
        self.emit_terminal(Terminal::Goto { block: init_block });

        // Init: get next value
        self.switch_block(init_block);
        let next_val = self.emit(
            InstructionValue::IteratorNext {
                iterator: iterator.clone(),
                loc: for_of.span,
            },
            for_of.span,
        );
        self.lower_for_left(&for_of.left, next_val, for_of.span);
        self.emit_terminal(Terminal::Goto { block: test_block });

        // Test
        self.switch_block(test_block);
        self.emit_terminal(Terminal::Goto { block: body_block });

        // Body
        self.break_targets.push(fallthrough);
        self.continue_targets.push(init_block);

        self.switch_block(body_block);
        self.lower_statement(&for_of.body);
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            self.emit_terminal(Terminal::Goto { block: init_block });
        }

        self.continue_targets.pop();
        self.break_targets.pop();

        self.switch_block(fallthrough);
    }

    /// Lower the `left` part of a for-in/for-of into appropriate declarations or stores.
    fn lower_for_left(&mut self, left: &ForStatementLeft<'_>, value: Place, loc: Span) {
        match left {
            ForStatementLeft::VariableDeclaration(decl) => {
                // Typically `for (let x of ...)` — single declarator
                if let Some(declarator) = decl.declarations.first() {
                    let kind = map_var_kind(decl.kind);
                    match &declarator.id {
                        BindingPattern::BindingIdentifier(id) => {
                            let lvalue = self.make_named_place(&id.name, id.span);
                            self.emit(
                                InstructionValue::DeclareLocal {
                                    lvalue: lvalue.clone(),
                                    type_: kind,
                                },
                                id.span,
                            );
                            self.emit(
                                InstructionValue::StoreLocal {
                                    lvalue,
                                    value,
                                    type_: Some(kind),
                                },
                                loc,
                            );
                        }
                        _ => {
                            self.emit(
                                InstructionValue::UnsupportedNode {
                                    node: "ForLeftDestructure".to_string(),
                                },
                                loc,
                            );
                        }
                    }
                }
            }
            ForStatementLeft::AssignmentTargetIdentifier(id) => {
                let lvalue = self.make_named_place(&id.name, id.span);
                self.emit(
                    InstructionValue::StoreLocal {
                        lvalue,
                        value,
                        type_: None,
                    },
                    loc,
                );
            }
            _ => {
                self.emit(
                    InstructionValue::UnsupportedNode {
                        node: "ForLeftComplex".to_string(),
                    },
                    loc,
                );
            }
        }
    }

    fn lower_while_statement(&mut self, while_stmt: &ast::WhileStatement<'_>) {
        let test_block = self.new_block(BlockKind::Value);
        let body_block = self.new_block(BlockKind::Loop);
        let fallthrough = self.new_block(BlockKind::Block);

        self.emit_terminal(Terminal::Goto { block: test_block });

        // Test
        self.switch_block(test_block);
        let test = self.lower_expression(&while_stmt.test);
        self.emit_terminal(Terminal::Branch {
            test,
            consequent: body_block,
            alternate: fallthrough,
        });

        // Body
        self.break_targets.push(fallthrough);
        self.continue_targets.push(test_block);

        self.switch_block(body_block);
        self.lower_statement(&while_stmt.body);
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            self.emit_terminal(Terminal::Goto { block: test_block });
        }

        self.continue_targets.pop();
        self.break_targets.pop();

        self.switch_block(fallthrough);
    }

    fn lower_do_while_statement(&mut self, do_while: &ast::DoWhileStatement<'_>) {
        let body_block = self.new_block(BlockKind::Loop);
        let test_block = self.new_block(BlockKind::Value);
        let fallthrough = self.new_block(BlockKind::Block);

        self.emit_terminal(Terminal::Goto { block: body_block });

        // Body
        self.break_targets.push(fallthrough);
        self.continue_targets.push(test_block);

        self.switch_block(body_block);
        self.lower_statement(&do_while.body);
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            self.emit_terminal(Terminal::Goto { block: test_block });
        }

        self.continue_targets.pop();
        self.break_targets.pop();

        // Test
        self.switch_block(test_block);
        let test = self.lower_expression(&do_while.test);
        self.emit_terminal(Terminal::Branch {
            test,
            consequent: body_block,
            alternate: fallthrough,
        });

        self.switch_block(fallthrough);
    }

    fn lower_try_statement(&mut self, try_stmt: &ast::TryStatement<'_>) {
        let try_block = self.new_block(BlockKind::Block);
        let handler_block = self.new_block(BlockKind::Catch);
        let fallthrough = self.new_block(BlockKind::Block);

        self.emit_terminal(Terminal::Try {
            block: try_block,
            handler: handler_block,
            fallthrough,
        });

        // Try body
        self.switch_block(try_block);
        for stmt in try_stmt.block.body.iter() {
            self.lower_statement(stmt);
        }
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            self.emit_terminal(Terminal::Goto { block: fallthrough });
        }

        // Handler
        self.switch_block(handler_block);
        if let Some(handler) = &try_stmt.handler {
            // Declare catch param if present
            if let Some(param) = &handler.param {
                if let BindingPattern::BindingIdentifier(id) = &param.pattern {
                    let lvalue = self.make_named_place(&id.name, id.span);
                    self.emit(
                        InstructionValue::DeclareLocal {
                            lvalue,
                            type_: InstructionKind::Let,
                        },
                        id.span,
                    );
                }
            }
            for stmt in handler.body.body.iter() {
                self.lower_statement(stmt);
            }
        }
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            self.emit_terminal(Terminal::Goto { block: fallthrough });
        }

        // Finalizer (lowered inline after the fallthrough for simplicity)
        self.switch_block(fallthrough);
        if let Some(finalizer) = &try_stmt.finalizer {
            for stmt in finalizer.body.iter() {
                self.lower_statement(stmt);
            }
        }
    }

    fn lower_labeled_statement(&mut self, labeled: &ast::LabeledStatement<'_>) {
        let label_name = labeled.label.name.to_string();
        let body_block = self.new_block(BlockKind::Block);
        let fallthrough = self.new_block(BlockKind::Block);

        let label_id = self.next_label;
        self.next_label += 1;

        self.emit_terminal(Terminal::Label {
            block: body_block,
            fallthrough,
            label: label_id,
        });

        // Register label for break/continue
        self.label_map
            .insert(label_name.clone(), (fallthrough, body_block));
        self.break_targets.push(fallthrough);

        self.switch_block(body_block);
        self.lower_statement(&labeled.body);
        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            self.emit_terminal(Terminal::Goto { block: fallthrough });
        }

        self.break_targets.pop();
        self.label_map.remove(&label_name);

        self.switch_block(fallthrough);
    }

    fn lower_break_statement(&mut self, brk: &ast::BreakStatement<'_>) {
        let target = if let Some(label) = &brk.label {
            let name = label.name.to_string();
            self.label_map
                .get(&name)
                .map(|(break_target, _)| *break_target)
        } else {
            self.break_targets.last().copied()
        };

        if let Some(target) = target {
            self.emit_terminal(Terminal::Goto { block: target });
        }

        let dead = self.new_block(BlockKind::Block);
        self.switch_block(dead);
    }

    fn lower_continue_statement(&mut self, cont: &ast::ContinueStatement<'_>) {
        let target = if let Some(label) = &cont.label {
            let name = label.name.to_string();
            self.label_map
                .get(&name)
                .map(|(_, cont_target)| *cont_target)
        } else {
            self.continue_targets.last().copied()
        };

        if let Some(target) = target {
            self.emit_terminal(Terminal::Goto { block: target });
        }

        let dead = self.new_block(BlockKind::Block);
        self.switch_block(dead);
    }

    fn lower_function_declaration(&mut self, func: &ast::Function<'_>) {
        let name = func.id.as_ref().map(|id| id.name.to_string());
        let loc = func.span;

        // Build inner function HIR
        let mut inner_builder = HIRBuilder::new(EnvironmentConfig::default());
        let inner_hir = inner_builder.build_function_inner(func);

        let lvalue = if let Some(ref n) = name {
            self.make_named_place(n, loc)
        } else {
            self.make_temp(loc)
        };

        self.emit(
            InstructionValue::DeclareLocal {
                lvalue: lvalue.clone(),
                type_: InstructionKind::HoistedFunction,
            },
            loc,
        );

        self.emit(
            InstructionValue::FunctionExpression {
                name,
                lowered_func: Box::new(inner_hir),
                expr_type: FunctionExprType::FunctionExpression,
            },
            loc,
        );
    }

    /// Build the inner HIR for a function (used by both declarations and expressions).
    fn build_function_inner(&mut self, func: &ast::Function<'_>) -> HIRFunction {
        let loc = func.span;
        let id = func.id.as_ref().map(|id| id.name.to_string());

        let params = self.lower_formal_params(&func.params);

        let directives = func
            .body
            .as_ref()
            .map(|body| {
                body.directives
                    .iter()
                    .map(|d| d.directive.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if let Some(body) = &func.body {
            for stmt in body.statements.iter() {
                self.lower_statement(stmt);
            }
        }

        if matches!(self.current_block_mut().terminal, Terminal::Unreachable) {
            let undef = self.emit(
                InstructionValue::Primitive {
                    value: Primitive::Undefined,
                },
                loc,
            );
            self.emit_terminal(Terminal::Return { value: undef });
        }

        let returns = self.make_temp(loc);
        let entry = self.blocks.first().map(|(id, _)| *id).unwrap();

        HIRFunction {
            loc,
            id,
            fn_type: ReactFunctionType::Other,
            params,
            returns,
            context: Vec::new(),
            body: HIR {
                entry,
                blocks: std::mem::take(&mut self.blocks),
            },
            is_async: func.r#async,
            is_generator: func.generator,
            directives,
        }
    }

    // ------------------------------------------------------------------
    // Expression lowering
    // ------------------------------------------------------------------

    /// Lower an expression, returning the Place that holds the result.
    fn lower_expression(&mut self, expr: &Expression<'_>) -> Place {
        let expr = expr.without_parentheses();
        let loc = expr.span();

        match expr {
            // Identifiers
            Expression::Identifier(ident) => {
                let name = ident.name.to_string();
                if is_global_name(&name) {
                    self.emit(
                        InstructionValue::LoadGlobal {
                            binding: GlobalBinding {
                                name,
                                kind: GlobalBindingKind::Global,
                            },
                        },
                        loc,
                    )
                } else if self.context_vars.contains(&name) {
                    // Variable captured from an outer scope — use context ops
                    let place = self.make_named_place(&name, loc);
                    self.emit(InstructionValue::LoadContext { place }, loc)
                } else {
                    let place = self.make_named_place(&name, loc);
                    self.emit(InstructionValue::LoadLocal { place }, loc)
                }
            }

            Expression::ThisExpression(_) => self.emit(
                InstructionValue::LoadGlobal {
                    binding: GlobalBinding {
                        name: "this".to_string(),
                        kind: GlobalBindingKind::Global,
                    },
                },
                loc,
            ),

            // Literals
            Expression::BooleanLiteral(lit) => self.emit(
                InstructionValue::Primitive {
                    value: Primitive::Boolean(lit.value),
                },
                loc,
            ),
            Expression::NullLiteral(_) => self.emit(
                InstructionValue::Primitive {
                    value: Primitive::Null,
                },
                loc,
            ),
            Expression::NumericLiteral(lit) => self.emit(
                InstructionValue::Primitive {
                    value: Primitive::Number(lit.value),
                },
                loc,
            ),
            Expression::StringLiteral(lit) => self.emit(
                InstructionValue::Primitive {
                    value: Primitive::String(lit.value.to_string()),
                },
                loc,
            ),
            Expression::BigIntLiteral(lit) => self.emit(
                InstructionValue::Primitive {
                    value: Primitive::BigInt(
                        lit.raw.as_ref().map(|r| r.to_string()).unwrap_or_default(),
                    ),
                },
                loc,
            ),
            Expression::RegExpLiteral(lit) => self.emit(
                InstructionValue::RegExpLiteral {
                    pattern: lit.regex.pattern.text.to_string(),
                    flags: format!("{:?}", lit.regex.flags),
                },
                loc,
            ),

            // Template literals
            Expression::TemplateLiteral(tpl) => {
                let quasis = tpl
                    .quasis
                    .iter()
                    .map(|q| {
                        q.value
                            .cooked
                            .as_ref()
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| q.value.raw.to_string())
                    })
                    .collect();
                let subexpressions = tpl
                    .expressions
                    .iter()
                    .map(|e| self.lower_expression(e))
                    .collect();
                self.emit(
                    InstructionValue::TemplateLiteral {
                        quasis,
                        subexpressions,
                    },
                    loc,
                )
            }

            Expression::TaggedTemplateExpression(tagged) => {
                let tag = self.lower_expression(&tagged.tag);
                let quasis = tagged
                    .quasi
                    .quasis
                    .iter()
                    .map(|q| {
                        q.value
                            .cooked
                            .as_ref()
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| q.value.raw.to_string())
                    })
                    .collect();
                let subexpressions = tagged
                    .quasi
                    .expressions
                    .iter()
                    .map(|e| self.lower_expression(e))
                    .collect();
                self.emit(
                    InstructionValue::TaggedTemplateExpression {
                        tag,
                        value: TemplateLiteralData {
                            quasis,
                            subexpressions,
                        },
                    },
                    loc,
                )
            }

            // Binary
            Expression::BinaryExpression(bin) => {
                let left = self.lower_expression(&bin.left);
                let right = self.lower_expression(&bin.right);
                self.emit(
                    InstructionValue::BinaryExpression {
                        op: map_binary_op(bin.operator),
                        left,
                        right,
                    },
                    loc,
                )
            }

            // Unary
            Expression::UnaryExpression(unary) => {
                let value = self.lower_expression(&unary.argument);
                self.emit(
                    InstructionValue::UnaryExpression {
                        op: map_unary_op(unary.operator),
                        value,
                    },
                    loc,
                )
            }

            // Update (++/--)
            Expression::UpdateExpression(update) => {
                let lvalue = self.lower_simple_assignment_target_as_place(&update.argument, loc);
                let op = map_update_op(update.operator);
                if update.prefix {
                    self.emit(InstructionValue::PrefixUpdate { op, lvalue }, loc)
                } else {
                    self.emit(InstructionValue::PostfixUpdate { op, lvalue }, loc)
                }
            }

            // Call expression
            Expression::CallExpression(call) => self.lower_call_expression(call, loc),

            // New expression
            Expression::NewExpression(new_expr) => {
                let callee = self.lower_expression(&new_expr.callee);
                let args = self.lower_arguments(&new_expr.arguments);
                self.emit(InstructionValue::NewExpression { callee, args }, loc)
            }

            // Member expression (static/computed)
            Expression::StaticMemberExpression(member) => {
                let object = self.lower_expression(&member.object);
                self.emit(
                    InstructionValue::PropertyLoad {
                        object,
                        property: member.property.name.to_string(),
                    },
                    loc,
                )
            }
            Expression::ComputedMemberExpression(member) => {
                let object = self.lower_expression(&member.object);
                let property = self.lower_expression(&member.expression);
                self.emit(InstructionValue::ComputedLoad { object, property }, loc)
            }
            Expression::PrivateFieldExpression(member) => {
                let object = self.lower_expression(&member.object);
                self.emit(
                    InstructionValue::PropertyLoad {
                        object,
                        property: format!("#{}", member.field.name),
                    },
                    loc,
                )
            }

            // Assignment
            Expression::AssignmentExpression(assign) => {
                self.lower_assignment_expression(assign, loc)
            }

            // Conditional (ternary)
            Expression::ConditionalExpression(cond) => self.lower_conditional_expression(cond, loc),

            // Logical
            Expression::LogicalExpression(logical) => self.lower_logical_expression(logical, loc),

            // Sequence
            Expression::SequenceExpression(seq) => {
                let mut last = self.make_temp(loc);
                for expr in seq.expressions.iter() {
                    last = self.lower_expression(expr);
                }
                last
            }

            // Object literal
            Expression::ObjectExpression(obj) => self.lower_object_expression(obj, loc),

            // Array literal
            Expression::ArrayExpression(arr) => self.lower_array_expression(arr, loc),

            // Arrow function
            Expression::ArrowFunctionExpression(arrow) => {
                let hir_func = self.build_arrow(arrow);
                self.emit(
                    InstructionValue::FunctionExpression {
                        name: None,
                        lowered_func: Box::new(hir_func),
                        expr_type: FunctionExprType::ArrowFunction,
                    },
                    loc,
                )
            }

            // Function expression
            Expression::FunctionExpression(func) => {
                let name = func.id.as_ref().map(|id| id.name.to_string());
                let mut inner_builder = HIRBuilder::new(EnvironmentConfig::default());
                let hir_func = inner_builder.build_function_inner(func);
                self.emit(
                    InstructionValue::FunctionExpression {
                        name,
                        lowered_func: Box::new(hir_func),
                        expr_type: FunctionExprType::FunctionExpression,
                    },
                    loc,
                )
            }

            // Await
            Expression::AwaitExpression(await_expr) => {
                let value = self.lower_expression(&await_expr.argument);
                self.emit(InstructionValue::Await { value }, loc)
            }

            // Yield
            Expression::YieldExpression(yield_expr) => {
                let _value = if let Some(arg) = &yield_expr.argument {
                    self.lower_expression(arg)
                } else {
                    self.emit(
                        InstructionValue::Primitive {
                            value: Primitive::Undefined,
                        },
                        loc,
                    )
                };
                // Model yield as an unsupported node for now (could add proper generator support)
                self.emit(
                    InstructionValue::UnsupportedNode {
                        node: "YieldExpression".to_string(),
                    },
                    loc,
                )
            }

            // JSX Element
            Expression::JSXElement(jsx) => self.lower_jsx_element(jsx, loc),

            // JSX Fragment
            Expression::JSXFragment(frag) => self.lower_jsx_fragment(frag, loc),

            // Import expression
            Expression::ImportExpression(import) => {
                let source = self.lower_expression(&import.source);
                let callee = self.emit(
                    InstructionValue::LoadGlobal {
                        binding: GlobalBinding {
                            name: "import".to_string(),
                            kind: GlobalBindingKind::Global,
                        },
                    },
                    loc,
                );
                self.emit(
                    InstructionValue::CallExpression {
                        callee,
                        args: vec![source],
                    },
                    loc,
                )
            }

            // Chain expression (optional chaining)
            Expression::ChainExpression(chain) => self.lower_chain_expression(chain, loc),

            // Super
            Expression::Super(_) => self.emit(
                InstructionValue::LoadGlobal {
                    binding: GlobalBinding {
                        name: "super".to_string(),
                        kind: GlobalBindingKind::Global,
                    },
                },
                loc,
            ),

            // MetaProperty (import.meta, new.target)
            Expression::MetaProperty(meta) => {
                let name = format!("{}.{}", meta.meta.name, meta.property.name);
                self.emit(
                    InstructionValue::LoadGlobal {
                        binding: GlobalBinding {
                            name,
                            kind: GlobalBindingKind::Global,
                        },
                    },
                    loc,
                )
            }

            // TS type assertions — unwrap and lower inner expression
            Expression::TSAsExpression(ts) => self.lower_expression(&ts.expression),
            Expression::TSSatisfiesExpression(ts) => self.lower_expression(&ts.expression),
            Expression::TSTypeAssertion(ts) => self.lower_expression(&ts.expression),
            Expression::TSNonNullExpression(ts) => self.lower_expression(&ts.expression),
            Expression::TSInstantiationExpression(ts) => self.lower_expression(&ts.expression),

            // Class expression
            Expression::ClassExpression(_) => self.emit(
                InstructionValue::UnsupportedNode {
                    node: "ClassExpression".to_string(),
                },
                loc,
            ),

            // PrivateInExpression (#field in obj)
            Expression::PrivateInExpression(_) => self.emit(
                InstructionValue::UnsupportedNode {
                    node: "PrivateInExpression".to_string(),
                },
                loc,
            ),

            // Parenthesized — should have been unwrapped by without_parentheses()
            Expression::ParenthesizedExpression(paren) => self.lower_expression(&paren.expression),

            // V8 intrinsic
            Expression::V8IntrinsicExpression(_) => self.emit(
                InstructionValue::UnsupportedNode {
                    node: "V8IntrinsicExpression".to_string(),
                },
                loc,
            ),
        }
    }

    // ------------------------------------------------------------------
    // Call expressions
    // ------------------------------------------------------------------

    fn lower_call_expression(&mut self, call: &ast::CallExpression<'_>, loc: Span) -> Place {
        // Detect useMemo / useCallback for manual memoization markers
        if let Some(callee_name) = extract_callee_name(&call.callee) {
            if callee_name == "useMemo" || callee_name == "useCallback" {
                let memo_id = self.next_memo_id;
                self.next_memo_id += 1;
                self.emit(
                    InstructionValue::StartMemoize {
                        manual_memo_id: memo_id,
                    },
                    loc,
                );
                // Lower the call normally
                let callee = self.lower_expression(&call.callee);
                let args = self.lower_arguments(&call.arguments);
                let result = self.emit(
                    InstructionValue::CallExpression {
                        callee,
                        args: args.clone(),
                    },
                    loc,
                );
                // The deps array is the second argument, if present
                let deps = if args.len() > 1 {
                    vec![args[1].clone()]
                } else {
                    Vec::new()
                };
                self.emit(
                    InstructionValue::FinishMemoize {
                        manual_memo_id: memo_id,
                        decl: result.clone(),
                        deps,
                        pruned: false,
                    },
                    loc,
                );
                return result;
            }
        }

        // Check if callee is a member expression → MethodCall
        match &call.callee {
            Expression::StaticMemberExpression(member) => {
                let receiver = self.lower_expression(&member.object);
                let property = member.property.name.to_string();
                let args = self.lower_arguments(&call.arguments);
                self.emit(
                    InstructionValue::MethodCall {
                        receiver,
                        property,
                        args,
                    },
                    loc,
                )
            }
            Expression::ComputedMemberExpression(member) => {
                // Computed method call: obj[prop](args)
                let object = self.lower_expression(&member.object);
                let property = self.lower_expression(&member.expression);
                let computed_access = self.emit(
                    InstructionValue::ComputedLoad {
                        object: object.clone(),
                        property,
                    },
                    loc,
                );
                let args = self.lower_arguments(&call.arguments);
                self.emit(
                    InstructionValue::CallExpression {
                        callee: computed_access,
                        args,
                    },
                    loc,
                )
            }
            _ => {
                let callee = self.lower_expression(&call.callee);
                let args = self.lower_arguments(&call.arguments);
                self.emit(InstructionValue::CallExpression { callee, args }, loc)
            }
        }
    }

    fn lower_arguments(&mut self, args: &[Argument<'_>]) -> Vec<Place> {
        args.iter()
            .map(|arg| match arg {
                Argument::SpreadElement(spread) => self.lower_expression(&spread.argument),
                _ => {
                    // All other Argument variants inherit from Expression
                    if let Some(expr) = arg_as_expression(arg) {
                        self.lower_expression(expr)
                    } else {
                        self.make_temp(arg.span())
                    }
                }
            })
            .collect()
    }

    // ------------------------------------------------------------------
    // Assignment expression
    // ------------------------------------------------------------------

    fn lower_assignment_expression(
        &mut self,
        assign: &ast::AssignmentExpression<'_>,
        loc: Span,
    ) -> Place {
        let rhs = self.lower_expression(&assign.right);

        // For compound assignment (+=, etc.), compute the new value
        let value = if let Some(bin_op) = compound_assignment_to_binary(assign.operator) {
            let lhs_val = self.lower_assignment_target_load(&assign.left, loc);
            self.emit(
                InstructionValue::BinaryExpression {
                    op: bin_op,
                    left: lhs_val,
                    right: rhs,
                },
                loc,
            )
        } else if matches!(
            assign.operator,
            AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalNullish
        ) {
            // Logical assignment: a &&= b, a ||= b, a ??= b
            // For simplicity, treat as regular assignment
            rhs
        } else {
            rhs
        };

        self.lower_assignment_target_store(&assign.left, value, loc)
    }

    /// Load the current value from an assignment target.
    fn lower_assignment_target_load(&mut self, target: &AssignmentTarget<'_>, loc: Span) -> Place {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                let name = id.name.to_string();
                let place = self.make_named_place(&name, id.span);
                self.emit(InstructionValue::LoadLocal { place }, loc)
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                let object = self.lower_expression(&member.object);
                self.emit(
                    InstructionValue::PropertyLoad {
                        object,
                        property: member.property.name.to_string(),
                    },
                    loc,
                )
            }
            AssignmentTarget::ComputedMemberExpression(member) => {
                let object = self.lower_expression(&member.object);
                let property = self.lower_expression(&member.expression);
                self.emit(InstructionValue::ComputedLoad { object, property }, loc)
            }
            _ => self.emit(
                InstructionValue::UnsupportedNode {
                    node: "AssignmentTargetLoad".to_string(),
                },
                loc,
            ),
        }
    }

    /// Store a value into an assignment target, returning the stored value place.
    fn lower_assignment_target_store(
        &mut self,
        target: &AssignmentTarget<'_>,
        value: Place,
        loc: Span,
    ) -> Place {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                let name = id.name.to_string();
                if is_global_name(&name) {
                    self.emit(
                        InstructionValue::StoreGlobal {
                            name,
                            value: value.clone(),
                        },
                        loc,
                    );
                } else {
                    let lvalue = self.make_named_place(&name, id.span);
                    self.emit(
                        InstructionValue::StoreLocal {
                            lvalue,
                            value: value.clone(),
                            type_: Some(InstructionKind::Reassign),
                        },
                        loc,
                    );
                }
                value
            }
            AssignmentTarget::StaticMemberExpression(member) => {
                let object = self.lower_expression(&member.object);
                self.emit(
                    InstructionValue::PropertyStore {
                        object,
                        property: member.property.name.to_string(),
                        value: value.clone(),
                    },
                    loc,
                );
                value
            }
            AssignmentTarget::ComputedMemberExpression(member) => {
                let object = self.lower_expression(&member.object);
                let property = self.lower_expression(&member.expression);
                self.emit(
                    InstructionValue::ComputedStore {
                        object,
                        property,
                        value: value.clone(),
                    },
                    loc,
                );
                value
            }
            AssignmentTarget::ArrayAssignmentTarget(_)
            | AssignmentTarget::ObjectAssignmentTarget(_) => {
                // Destructuring assignment
                self.emit(
                    InstructionValue::UnsupportedNode {
                        node: "DestructuringAssignment".to_string(),
                    },
                    loc,
                )
            }
            _ => self.emit(
                InstructionValue::UnsupportedNode {
                    node: "AssignmentTargetStore".to_string(),
                },
                loc,
            ),
        }
    }

    fn lower_simple_assignment_target_as_place(
        &mut self,
        target: &SimpleAssignmentTarget<'_>,
        loc: Span,
    ) -> Place {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                self.make_named_place(&id.name, id.span)
            }
            SimpleAssignmentTarget::StaticMemberExpression(member) => {
                let object = self.lower_expression(&member.object);
                self.emit(
                    InstructionValue::PropertyLoad {
                        object,
                        property: member.property.name.to_string(),
                    },
                    loc,
                )
            }
            SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                let object = self.lower_expression(&member.object);
                let property = self.lower_expression(&member.expression);
                self.emit(InstructionValue::ComputedLoad { object, property }, loc)
            }
            _ => self.make_temp(loc),
        }
    }

    // ------------------------------------------------------------------
    // Conditional (ternary)
    // ------------------------------------------------------------------

    fn lower_conditional_expression(
        &mut self,
        cond: &ast::ConditionalExpression<'_>,
        loc: Span,
    ) -> Place {
        let test = self.lower_expression(&cond.test);
        let consequent_block = self.new_block(BlockKind::Value);
        let alternate_block = self.new_block(BlockKind::Value);
        let fallthrough = self.new_block(BlockKind::Block);

        self.emit_terminal(Terminal::Ternary {
            test,
            consequent: consequent_block,
            alternate: alternate_block,
            fallthrough,
        });

        // Consequent
        self.switch_block(consequent_block);
        let _cons_val = self.lower_expression(&cond.consequent);
        self.emit_terminal(Terminal::Goto { block: fallthrough });

        // Alternate
        self.switch_block(alternate_block);
        let _alt_val = self.lower_expression(&cond.alternate);
        self.emit_terminal(Terminal::Goto { block: fallthrough });

        self.switch_block(fallthrough);

        // The result is a phi between cons_val and alt_val.
        // For now, return a temp — phi insertion happens in a later pass.
        self.make_temp(loc)
    }

    // ------------------------------------------------------------------
    // Logical expressions
    // ------------------------------------------------------------------

    fn lower_logical_expression(
        &mut self,
        logical: &ast::LogicalExpression<'_>,
        loc: Span,
    ) -> Place {
        let left_block = self.new_block(BlockKind::Value);
        let right_block = self.new_block(BlockKind::Value);
        let fallthrough = self.new_block(BlockKind::Block);

        let op = map_logical_op(logical.operator);

        // Emit left into left_block
        self.emit_terminal(Terminal::Goto { block: left_block });
        self.switch_block(left_block);
        let _left_val = self.lower_expression(&logical.left);

        self.emit_terminal(Terminal::Logical {
            operator: op,
            left: left_block,
            right: right_block,
            fallthrough,
        });

        // Right
        self.switch_block(right_block);
        let _right_val = self.lower_expression(&logical.right);
        self.emit_terminal(Terminal::Goto { block: fallthrough });

        self.switch_block(fallthrough);
        self.make_temp(loc)
    }

    // ------------------------------------------------------------------
    // Object / Array literals
    // ------------------------------------------------------------------

    fn lower_object_expression(&mut self, obj: &ast::ObjectExpression<'_>, loc: Span) -> Place {
        let mut properties = Vec::new();
        for prop_kind in obj.properties.iter() {
            match prop_kind {
                ObjectPropertyKind::ObjectProperty(prop) => {
                    if prop.method {
                        // Object method shorthand
                        let value = self.lower_expression(&prop.value);
                        let key = self.lower_obj_property_key(&prop.key);
                        properties.push(ObjectProperty {
                            key,
                            value,
                            shorthand: false,
                        });
                    } else {
                        let value = self.lower_expression(&prop.value);
                        let key = self.lower_obj_property_key(&prop.key);
                        properties.push(ObjectProperty {
                            key,
                            value,
                            shorthand: prop.shorthand,
                        });
                    }
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    let value = self.lower_expression(&spread.argument);
                    properties.push(ObjectProperty {
                        key: ObjectPropertyKey::Identifier("...".to_string()),
                        value,
                        shorthand: false,
                    });
                }
            }
        }
        self.emit(InstructionValue::ObjectExpression { properties }, loc)
    }

    fn lower_obj_property_key(&mut self, key: &PropertyKey<'_>) -> ObjectPropertyKey {
        match key {
            PropertyKey::StaticIdentifier(id) => ObjectPropertyKey::Identifier(id.name.to_string()),
            PropertyKey::StringLiteral(s) => ObjectPropertyKey::Identifier(s.value.to_string()),
            PropertyKey::NumericLiteral(n) => ObjectPropertyKey::Identifier(n.value.to_string()),
            _ => {
                // Computed property key — lower the expression
                if let Some(expr) = property_key_as_expression(key) {
                    let place = self.lower_expression(expr);
                    ObjectPropertyKey::Computed(place)
                } else {
                    ObjectPropertyKey::Identifier("<unknown>".to_string())
                }
            }
        }
    }

    fn lower_array_expression(&mut self, arr: &ast::ArrayExpression<'_>, loc: Span) -> Place {
        let mut elements = Vec::new();
        for elem in arr.elements.iter() {
            match elem {
                ArrayExpressionElement::SpreadElement(spread) => {
                    let val = self.lower_expression(&spread.argument);
                    elements.push(ArrayElement::Spread(val));
                }
                ArrayExpressionElement::Elision(_) => {
                    elements.push(ArrayElement::Hole);
                }
                _ => {
                    if let Some(expr) = array_elem_as_expression(elem) {
                        let val = self.lower_expression(expr);
                        elements.push(ArrayElement::Expression(val));
                    } else {
                        elements.push(ArrayElement::Hole);
                    }
                }
            }
        }
        self.emit(InstructionValue::ArrayExpression { elements }, loc)
    }

    // ------------------------------------------------------------------
    // JSX
    // ------------------------------------------------------------------

    fn lower_jsx_element(&mut self, jsx: &ast::JSXElement<'_>, loc: Span) -> Place {
        let tag = self.lower_jsx_element_name(&jsx.opening_element.name, loc);
        let props = self.lower_jsx_attributes(&jsx.opening_element.attributes);
        let children = self.lower_jsx_children(&jsx.children);

        self.emit(
            InstructionValue::JsxExpression {
                tag,
                props,
                children,
            },
            loc,
        )
    }

    fn lower_jsx_fragment(&mut self, frag: &ast::JSXFragment<'_>, loc: Span) -> Place {
        let children = self.lower_jsx_children(&frag.children);
        self.emit(InstructionValue::JsxFragment { children }, loc)
    }

    fn lower_jsx_element_name(&mut self, name: &JSXElementName<'_>, loc: Span) -> Place {
        match name {
            JSXElementName::Identifier(id) => {
                // Lowercase identifiers are intrinsic elements (div, span, etc.)
                self.emit(
                    InstructionValue::Primitive {
                        value: Primitive::String(id.name.to_string()),
                    },
                    loc,
                )
            }
            JSXElementName::IdentifierReference(id) => {
                // Component reference
                let name = id.name.to_string();
                let place = self.make_named_place(&name, id.span);
                self.emit(InstructionValue::LoadLocal { place }, loc)
            }
            JSXElementName::MemberExpression(member) => {
                self.lower_jsx_member_expression(member, loc)
            }
            JSXElementName::NamespacedName(ns) => self.emit(
                InstructionValue::Primitive {
                    value: Primitive::String(format!("{}:{}", ns.namespace.name, ns.name.name)),
                },
                loc,
            ),
            JSXElementName::ThisExpression(_) => self.emit(
                InstructionValue::LoadGlobal {
                    binding: GlobalBinding {
                        name: "this".to_string(),
                        kind: GlobalBindingKind::Global,
                    },
                },
                loc,
            ),
        }
    }

    fn lower_jsx_member_expression(
        &mut self,
        member: &ast::JSXMemberExpression<'_>,
        loc: Span,
    ) -> Place {
        let object = match &member.object {
            JSXMemberExpressionObject::IdentifierReference(id) => {
                let name = id.name.to_string();
                let place = self.make_named_place(&name, id.span);
                self.emit(InstructionValue::LoadLocal { place }, loc)
            }
            JSXMemberExpressionObject::MemberExpression(inner) => {
                self.lower_jsx_member_expression(inner, loc)
            }
            JSXMemberExpressionObject::ThisExpression(_) => self.emit(
                InstructionValue::LoadGlobal {
                    binding: GlobalBinding {
                        name: "this".to_string(),
                        kind: GlobalBindingKind::Global,
                    },
                },
                loc,
            ),
        };
        self.emit(
            InstructionValue::PropertyLoad {
                object,
                property: member.property.name.to_string(),
            },
            loc,
        )
    }

    fn lower_jsx_attributes(&mut self, attrs: &[JSXAttributeItem<'_>]) -> Vec<JsxAttribute> {
        let mut result = Vec::new();
        for attr in attrs {
            match attr {
                JSXAttributeItem::Attribute(a) => {
                    let name = match &a.name {
                        JSXAttributeName::Identifier(id) => {
                            JsxAttributeName::Named(id.name.to_string())
                        }
                        JSXAttributeName::NamespacedName(ns) => JsxAttributeName::Named(format!(
                            "{}:{}",
                            ns.namespace.name, ns.name.name
                        )),
                    };
                    let value = if let Some(val) = &a.value {
                        match val {
                            JSXAttributeValue::StringLiteral(s) => self.emit(
                                InstructionValue::Primitive {
                                    value: Primitive::String(s.value.to_string()),
                                },
                                a.span,
                            ),
                            JSXAttributeValue::ExpressionContainer(container) => {
                                match &container.expression {
                                    JSXExpression::EmptyExpression(_) => self.emit(
                                        InstructionValue::Primitive {
                                            value: Primitive::Boolean(true),
                                        },
                                        a.span,
                                    ),
                                    _ => {
                                        if let Some(expr) =
                                            jsx_expression_as_expression(&container.expression)
                                        {
                                            self.lower_expression(expr)
                                        } else {
                                            self.emit(
                                                InstructionValue::Primitive {
                                                    value: Primitive::Boolean(true),
                                                },
                                                a.span,
                                            )
                                        }
                                    }
                                }
                            }
                            JSXAttributeValue::Element(el) => self.lower_jsx_element(el, a.span),
                            JSXAttributeValue::Fragment(frag) => {
                                self.lower_jsx_fragment(frag, a.span)
                            }
                        }
                    } else {
                        // Boolean attribute: `<div disabled />`
                        self.emit(
                            InstructionValue::Primitive {
                                value: Primitive::Boolean(true),
                            },
                            a.span,
                        )
                    };
                    result.push(JsxAttribute { name, value });
                }
                JSXAttributeItem::SpreadAttribute(spread) => {
                    let value = self.lower_expression(&spread.argument);
                    result.push(JsxAttribute {
                        name: JsxAttributeName::Spread,
                        value,
                    });
                }
            }
        }
        result
    }

    fn lower_jsx_children(&mut self, children: &[JSXChild<'_>]) -> Vec<Place> {
        let mut result = Vec::new();
        for child in children {
            match child {
                JSXChild::Text(text) => {
                    let trimmed = text.value.to_string();
                    if !trimmed.trim().is_empty() {
                        result.push(
                            self.emit(InstructionValue::JSXText { value: trimmed }, text.span),
                        );
                    }
                }
                JSXChild::Element(el) => {
                    result.push(self.lower_jsx_element(el, el.span));
                }
                JSXChild::Fragment(frag) => {
                    result.push(self.lower_jsx_fragment(frag, frag.span));
                }
                JSXChild::ExpressionContainer(container) => {
                    match &container.expression {
                        JSXExpression::EmptyExpression(_) => {
                            // Empty expression container — skip
                        }
                        _ => {
                            if let Some(expr) = jsx_expression_as_expression(&container.expression)
                            {
                                result.push(self.lower_expression(expr));
                            }
                        }
                    }
                }
                JSXChild::Spread(spread) => {
                    result.push(self.lower_expression(&spread.expression));
                }
            }
        }
        result
    }

    // ------------------------------------------------------------------
    // Chain expression (optional chaining: a?.b, a?.b(), a?.[b])
    // ------------------------------------------------------------------

    fn lower_chain_expression(&mut self, chain: &ast::ChainExpression<'_>, loc: Span) -> Place {
        use ast::ChainElement;
        match &chain.expression {
            ChainElement::CallExpression(call) => {
                // a?.b() — lower as regular call for now
                // In a full implementation we'd emit Optional terminal
                self.lower_call_expression(call, loc)
            }
            ChainElement::StaticMemberExpression(member) => {
                let object = self.lower_expression(&member.object);
                self.emit(
                    InstructionValue::PropertyLoad {
                        object,
                        property: member.property.name.to_string(),
                    },
                    loc,
                )
            }
            ChainElement::ComputedMemberExpression(member) => {
                let object = self.lower_expression(&member.object);
                let property = self.lower_expression(&member.expression);
                self.emit(InstructionValue::ComputedLoad { object, property }, loc)
            }
            ChainElement::PrivateFieldExpression(member) => {
                let object = self.lower_expression(&member.object);
                self.emit(
                    InstructionValue::PropertyLoad {
                        object,
                        property: format!("#{}", member.field.name),
                    },
                    loc,
                )
            }
            _ => {
                // TSNonNullExpression and other TS-specific chain elements
                self.emit(
                    InstructionValue::UnsupportedNode {
                        node: "ChainElement".to_string(),
                    },
                    loc,
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Free helper functions
// ---------------------------------------------------------------------------

/// Check whether a name is likely a global (heuristic).
/// Extract the simple name of a callee expression, if it's a plain identifier.
fn extract_callee_name(expr: &Expression<'_>) -> Option<String> {
    match expr.without_parentheses() {
        Expression::Identifier(ident) => Some(ident.name.to_string()),
        _ => None,
    }
}

fn is_global_name(name: &str) -> bool {
    matches!(
        name,
        "undefined"
            | "NaN"
            | "Infinity"
            | "globalThis"
            | "console"
            | "Math"
            | "JSON"
            | "Array"
            | "Object"
            | "String"
            | "Number"
            | "Boolean"
            | "Symbol"
            | "BigInt"
            | "Map"
            | "Set"
            | "WeakMap"
            | "WeakSet"
            | "Promise"
            | "Date"
            | "RegExp"
            | "Error"
            | "TypeError"
            | "RangeError"
            | "SyntaxError"
            | "ReferenceError"
            | "parseInt"
            | "parseFloat"
            | "isNaN"
            | "isFinite"
            | "encodeURI"
            | "decodeURI"
            | "encodeURIComponent"
            | "decodeURIComponent"
            | "setTimeout"
            | "setInterval"
            | "clearTimeout"
            | "clearInterval"
            | "fetch"
            | "alert"
            | "confirm"
            | "prompt"
            | "window"
            | "document"
            | "navigator"
            | "location"
            | "history"
            | "performance"
            | "require"
            | "module"
            | "exports"
            | "process"
            | "__dirname"
            | "__filename"
    )
}

/// Get a short name for a statement kind (for UnsupportedNode).
fn stmt_kind_name(stmt: &Statement<'_>) -> &'static str {
    match stmt {
        Statement::BlockStatement(_) => "BlockStatement",
        Statement::BreakStatement(_) => "BreakStatement",
        Statement::ContinueStatement(_) => "ContinueStatement",
        Statement::DebuggerStatement(_) => "DebuggerStatement",
        Statement::DoWhileStatement(_) => "DoWhileStatement",
        Statement::EmptyStatement(_) => "EmptyStatement",
        Statement::ExpressionStatement(_) => "ExpressionStatement",
        Statement::ForInStatement(_) => "ForInStatement",
        Statement::ForOfStatement(_) => "ForOfStatement",
        Statement::ForStatement(_) => "ForStatement",
        Statement::IfStatement(_) => "IfStatement",
        Statement::LabeledStatement(_) => "LabeledStatement",
        Statement::ReturnStatement(_) => "ReturnStatement",
        Statement::SwitchStatement(_) => "SwitchStatement",
        Statement::ThrowStatement(_) => "ThrowStatement",
        Statement::TryStatement(_) => "TryStatement",
        Statement::WhileStatement(_) => "WhileStatement",
        Statement::WithStatement(_) => "WithStatement",
        _ => "Other",
    }
}

/// Try to extract an Expression from a ForStatementInit.
fn for_init_as_expression<'a>(init: &'a ForStatementInit<'a>) -> Option<&'a Expression<'a>> {
    // ForStatementInit inherits Expression variants
    match init {
        ForStatementInit::BooleanLiteral(e) => {
            Some(unsafe { &*(e.as_ref() as *const ast::BooleanLiteral as *const Expression<'a>) })
        }
        // For simplicity, handle the common case by emitting UnsupportedNode
        // A full implementation would match all Expression variants
        _ => None,
    }
}

/// Try to extract an Expression from an Argument.
fn arg_as_expression<'a>(arg: &'a Argument<'a>) -> Option<&'a Expression<'a>> {
    // Argument inherits from Expression; we can cast for the common cases.
    // The macro `inherit_variants!` means these are structurally identical for
    // all expression variants. We handle the safe subset.
    match arg {
        Argument::SpreadElement(_) => None,
        // All other variants ARE expression variants; use a transmute-free approach
        // by matching common ones we care about.
        _ => {
            // SAFETY: Argument inherits Expression variants with the same layout
            // when it's not SpreadElement.
            Some(unsafe { &*(arg as *const Argument<'a> as *const Expression<'a>) })
        }
    }
}

/// Try to extract an Expression from an ArrayExpressionElement.
fn array_elem_as_expression<'a>(
    elem: &'a ArrayExpressionElement<'a>,
) -> Option<&'a Expression<'a>> {
    match elem {
        ArrayExpressionElement::SpreadElement(_) | ArrayExpressionElement::Elision(_) => None,
        _ => {
            // SAFETY: inherited Expression variants have the same layout
            Some(unsafe { &*(elem as *const ArrayExpressionElement<'a> as *const Expression<'a>) })
        }
    }
}

/// Try to extract an Expression from a JSXExpression.
fn jsx_expression_as_expression<'a>(expr: &'a JSXExpression<'a>) -> Option<&'a Expression<'a>> {
    match expr {
        JSXExpression::EmptyExpression(_) => None,
        _ => {
            // SAFETY: inherited Expression variants
            Some(unsafe { &*(expr as *const JSXExpression<'a> as *const Expression<'a>) })
        }
    }
}

/// Try to extract an Expression from a PropertyKey.
fn property_key_as_expression<'a>(key: &'a PropertyKey<'a>) -> Option<&'a Expression<'a>> {
    match key {
        PropertyKey::StaticIdentifier(_) | PropertyKey::PrivateIdentifier(_) => None,
        _ => {
            // SAFETY: inherited Expression variants
            Some(unsafe { &*(key as *const PropertyKey<'a> as *const Expression<'a>) })
        }
    }
}

use oxc_span::GetSpan;
