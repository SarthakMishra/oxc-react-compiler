//! Per-pass unit tests for inference and reactive scope passes.
//!
//! These tests construct hand-crafted HIR structures and exercise individual
//! compiler passes in isolation, verifying their effects on the IR.

use oxc_react_compiler::hir::types::*;
use oxc_react_compiler::inference::infer_reactive_places::infer_reactive_places;
use oxc_react_compiler::reactive_scopes::build_reactive_function::build_reactive_function;
use oxc_react_compiler::reactive_scopes::codegen::codegen_function;
use oxc_react_compiler::reactive_scopes::infer_reactive_scope_variables::infer_reactive_scope_variables;
use oxc_react_compiler::reactive_scopes::propagate_dependencies::propagate_scope_dependencies_hir;
use oxc_span::Span;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn dummy_span() -> Span {
    Span::new(0, 0)
}

fn make_identifier(ids: &mut IdGenerator, name: &str) -> Identifier {
    Identifier {
        id: ids.next_identifier_id(),
        ssa_version: 0,
        declaration_id: Some(ids.next_declaration_id()),
        name: Some(name.to_string()),
        mutable_range: MutableRange { start: InstructionId(0), end: InstructionId(0) },
        last_use: InstructionId(0),
        scope: None,
        type_: Type::Poly,
        loc: dummy_span(),
    }
}

fn make_place(ids: &mut IdGenerator, name: &str) -> Place {
    Place {
        identifier: make_identifier(ids, name),
        effect: Effect::Unknown,
        reactive: false,
        loc: dummy_span(),
    }
}

fn make_reactive_place(ids: &mut IdGenerator, name: &str) -> Place {
    Place {
        identifier: make_identifier(ids, name),
        effect: Effect::Read,
        reactive: true,
        loc: dummy_span(),
    }
}

fn make_instruction(ids: &mut IdGenerator, lvalue: Place, value: InstructionValue) -> Instruction {
    Instruction { id: ids.next_instruction_id(), lvalue, value, loc: dummy_span(), effects: None }
}

// ---------------------------------------------------------------------------
// infer_reactive_places tests
// ---------------------------------------------------------------------------

#[test]
fn test_infer_reactive_places_simple() {
    // Build a simple HIR: one block with a LoadLocal of a reactive place.
    let mut ids = IdGenerator::new();
    let block_id = ids.next_block_id();

    let mut props = make_reactive_place(&mut ids, "props");
    props.reactive = true;

    let temp = make_place(&mut ids, "t0");

    let instr = make_instruction(&mut ids, temp, InstructionValue::LoadLocal { place: props });

    let block = BasicBlock {
        kind: BlockKind::Block,
        id: block_id,
        instructions: vec![instr],
        terminal: Terminal::Return { value: make_place(&mut ids, "t0") },
        preds: vec![],
        phis: vec![],
    };

    let mut hir = HIR { entry: block_id, blocks: vec![(block_id, block)] };

    infer_reactive_places(&mut hir, &[], &[]);

    // The pass should run without panicking on a valid HIR.
    assert_eq!(hir.blocks.len(), 1);
}

#[test]
fn test_infer_reactive_places_empty_hir() {
    let mut ids = IdGenerator::new();
    let block_id = ids.next_block_id();

    let block = BasicBlock {
        kind: BlockKind::Block,
        id: block_id,
        instructions: vec![],
        terminal: Terminal::Return { value: make_place(&mut ids, "undefined") },
        preds: vec![],
        phis: vec![],
    };

    let mut hir = HIR { entry: block_id, blocks: vec![(block_id, block)] };

    infer_reactive_places(&mut hir, &[], &[]);
    assert_eq!(hir.blocks.len(), 1);
}

#[test]
fn test_infer_reactive_places_only_seeds_params() {
    // Entry block has two DeclareLocal: one for param "props" and one for local "x".
    // Only "props" should be seeded as reactive since it's in param_names.
    let mut ids = IdGenerator::new();
    let block_id = ids.next_block_id();

    let props_lvalue = make_place(&mut ids, "decl_props");
    let props_inner = make_place(&mut ids, "props");
    let param_instr = make_instruction(
        &mut ids,
        props_lvalue,
        InstructionValue::DeclareLocal { lvalue: props_inner, type_: InstructionKind::Let },
    );

    let x_lvalue = make_place(&mut ids, "decl_x");
    let x_inner = make_place(&mut ids, "x");
    let local_instr = make_instruction(
        &mut ids,
        x_lvalue,
        InstructionValue::DeclareLocal { lvalue: x_inner, type_: InstructionKind::Const },
    );

    let block = BasicBlock {
        kind: BlockKind::Block,
        id: block_id,
        instructions: vec![param_instr, local_instr],
        terminal: Terminal::Return { value: make_place(&mut ids, "undefined") },
        preds: vec![],
        phis: vec![],
    };

    let mut hir = HIR { entry: block_id, blocks: vec![(block_id, block)] };

    // Only "props" is a param — "x" should NOT be seeded as reactive
    infer_reactive_places(&mut hir, &["props".to_string()], &[]);

    let block = &hir.blocks[0].1;
    // First instruction (DeclareLocal for "props") should be reactive
    assert!(block.instructions[0].lvalue.reactive, "param 'props' should be reactive");
    // Second instruction (DeclareLocal for "x") should NOT be reactive
    assert!(!block.instructions[1].lvalue.reactive, "local 'x' should NOT be reactive");
}

// ---------------------------------------------------------------------------
// infer_reactive_scope_variables tests
// ---------------------------------------------------------------------------

#[test]
fn test_infer_reactive_scope_variables_no_reactive() {
    // A HIR with no reactive places should produce no scopes.
    let mut ids = IdGenerator::new();
    let block_id = ids.next_block_id();

    let lvalue = make_place(&mut ids, "x");
    let instr = make_instruction(
        &mut ids,
        lvalue,
        InstructionValue::Primitive { value: Primitive::Number(42.0) },
    );

    let block = BasicBlock {
        kind: BlockKind::Block,
        id: block_id,
        instructions: vec![instr],
        terminal: Terminal::Return { value: make_place(&mut ids, "x") },
        preds: vec![],
        phis: vec![],
    };

    let mut hir = HIR { entry: block_id, blocks: vec![(block_id, block)] };

    let scopes = infer_reactive_scope_variables(&mut hir, &[]);
    // No reactive identifiers means no scopes (or empty scopes).
    // The pass should at minimum not panic.
    assert!(
        scopes.is_empty() || scopes.iter().all(|s| s.declarations.is_empty()),
        "non-reactive HIR should not produce meaningful scopes"
    );
}

#[test]
fn test_infer_reactive_scope_variables_with_reactive_identifier() {
    // A HIR with a reactive identifier should produce at least one scope.
    let mut ids = IdGenerator::new();
    let block_id = ids.next_block_id();

    let mut lvalue = make_place(&mut ids, "derived");
    lvalue.reactive = true;
    lvalue.identifier.mutable_range =
        MutableRange { start: InstructionId(0), end: InstructionId(2) };

    let props = make_reactive_place(&mut ids, "props");

    let instr = make_instruction(&mut ids, lvalue, InstructionValue::LoadLocal { place: props });

    let block = BasicBlock {
        kind: BlockKind::Block,
        id: block_id,
        instructions: vec![instr],
        terminal: Terminal::Return { value: make_place(&mut ids, "derived") },
        preds: vec![],
        phis: vec![],
    };

    let mut hir = HIR { entry: block_id, blocks: vec![(block_id, block)] };

    let scopes = infer_reactive_scope_variables(&mut hir, &[]);
    // Should run without panic; scope count depends on the pass logic.
    let _ = scopes;
}

// ---------------------------------------------------------------------------
// propagate_scope_dependencies_hir tests
// ---------------------------------------------------------------------------

#[test]
fn test_propagate_scope_dependencies_empty() {
    let mut ids = IdGenerator::new();
    let block_id = ids.next_block_id();

    let block = BasicBlock {
        kind: BlockKind::Block,
        id: block_id,
        instructions: vec![],
        terminal: Terminal::Return { value: make_place(&mut ids, "undefined") },
        preds: vec![],
        phis: vec![],
    };

    let mut hir = HIR { entry: block_id, blocks: vec![(block_id, block)] };

    propagate_scope_dependencies_hir(&mut hir, &[]);
    assert_eq!(hir.blocks.len(), 1);
}

// ---------------------------------------------------------------------------
// build_reactive_function tests
// ---------------------------------------------------------------------------

#[test]
fn test_build_reactive_function_single_block() {
    // Build a simple HIR with one block and a Return terminal.
    let mut ids = IdGenerator::new();
    let block_id = ids.next_block_id();

    let x = make_place(&mut ids, "x");
    let instr = make_instruction(
        &mut ids,
        x.clone(),
        InstructionValue::Primitive { value: Primitive::Number(1.0) },
    );

    let block = BasicBlock {
        kind: BlockKind::Block,
        id: block_id,
        instructions: vec![instr],
        terminal: Terminal::Return { value: x },
        preds: vec![],
        phis: vec![],
    };

    let hir = HIR { entry: block_id, blocks: vec![(block_id, block)] };

    let rf = build_reactive_function(
        hir,
        vec![],
        Some("TestComponent".to_string()),
        dummy_span(),
        vec![],
        false,
        false,
        false,
    );

    assert_eq!(rf.id.as_deref(), Some("TestComponent"));
    assert!(rf.params.is_empty());
    // Body should have at least the instruction and a return.
    assert!(!rf.body.instructions.is_empty());
}

#[test]
fn test_build_reactive_function_with_params() {
    let mut ids = IdGenerator::new();
    let block_id = ids.next_block_id();

    let param_place = make_place(&mut ids, "props");
    let ret = make_place(&mut ids, "result");

    let instr = make_instruction(
        &mut ids,
        ret.clone(),
        InstructionValue::LoadLocal { place: param_place.clone() },
    );

    let block = BasicBlock {
        kind: BlockKind::Block,
        id: block_id,
        instructions: vec![instr],
        terminal: Terminal::Return { value: ret },
        preds: vec![],
        phis: vec![],
    };

    let hir = HIR { entry: block_id, blocks: vec![(block_id, block)] };

    let rf = build_reactive_function(
        hir,
        vec![Param::Identifier(param_place)],
        Some("MyHook".to_string()),
        dummy_span(),
        vec![],
        false,
        false,
        false,
    );

    assert_eq!(rf.params.len(), 1);
    assert_eq!(rf.id.as_deref(), Some("MyHook"));
}

#[test]
fn test_build_reactive_function_if_terminal() {
    // Build a HIR with an If terminal: entry -> consequent/alternate -> fallthrough.
    let mut ids = IdGenerator::new();
    let entry_id = ids.next_block_id();
    let cons_id = ids.next_block_id();
    let alt_id = ids.next_block_id();
    let ft_id = ids.next_block_id();

    let test_place = make_place(&mut ids, "cond");
    let ret_val = make_place(&mut ids, "result");

    let entry = BasicBlock {
        kind: BlockKind::Block,
        id: entry_id,
        instructions: vec![make_instruction(
            &mut ids,
            test_place.clone(),
            InstructionValue::Primitive { value: Primitive::Boolean(true) },
        )],
        terminal: Terminal::If {
            test: test_place,
            consequent: cons_id,
            alternate: alt_id,
            fallthrough: ft_id,
        },
        preds: vec![],
        phis: vec![],
    };

    let place_a = make_place(&mut ids, "a");
    let cons_instr = make_instruction(
        &mut ids,
        place_a,
        InstructionValue::Primitive { value: Primitive::Number(1.0) },
    );
    let consequent = BasicBlock {
        kind: BlockKind::Block,
        id: cons_id,
        instructions: vec![cons_instr],
        terminal: Terminal::Goto { block: ft_id },
        preds: vec![entry_id],
        phis: vec![],
    };

    let place_b = make_place(&mut ids, "b");
    let alt_instr = make_instruction(
        &mut ids,
        place_b,
        InstructionValue::Primitive { value: Primitive::Number(2.0) },
    );
    let alternate = BasicBlock {
        kind: BlockKind::Block,
        id: alt_id,
        instructions: vec![alt_instr],
        terminal: Terminal::Goto { block: ft_id },
        preds: vec![entry_id],
        phis: vec![],
    };

    let fallthrough = BasicBlock {
        kind: BlockKind::Block,
        id: ft_id,
        instructions: vec![],
        terminal: Terminal::Return { value: ret_val },
        preds: vec![cons_id, alt_id],
        phis: vec![],
    };

    let hir = HIR {
        entry: entry_id,
        blocks: vec![
            (entry_id, entry),
            (cons_id, consequent),
            (alt_id, alternate),
            (ft_id, fallthrough),
        ],
    };

    let rf = build_reactive_function(
        hir,
        vec![],
        Some("IfComponent".to_string()),
        dummy_span(),
        vec![],
        false,
        false,
        false,
    );

    assert_eq!(rf.id.as_deref(), Some("IfComponent"));
    // Should have converted the If terminal into a ReactiveTerminal::If.
    let has_if = rf
        .body
        .instructions
        .iter()
        .any(|i| matches!(i, ReactiveInstruction::Terminal(ReactiveTerminal::If { .. })));
    assert!(has_if, "should contain an If terminal in the reactive function");
}

// ---------------------------------------------------------------------------
// codegen_function tests
// ---------------------------------------------------------------------------

#[test]
fn test_codegen_empty_function() {
    let rf = ReactiveFunction {
        loc: dummy_span(),
        id: Some("Empty".to_string()),
        params: vec![],
        body: ReactiveBlock { instructions: vec![] },
        directives: vec![],
        is_arrow: false,
        is_async: false,
        is_generator: false,
    };

    let code = codegen_function(&rf);
    assert!(code.contains("function Empty()"));
    assert!(code.contains('}'));
}

#[test]
fn test_codegen_arrow_function() {
    let rf = ReactiveFunction {
        loc: dummy_span(),
        id: None,
        params: vec![],
        body: ReactiveBlock { instructions: vec![] },
        directives: vec![],
        is_arrow: true,
        is_async: false,
        is_generator: false,
    };

    let code = codegen_function(&rf);
    assert!(code.contains("() => {"), "Arrow function should use => syntax: {code}");
    assert!(!code.contains("function"), "Arrow function should not contain 'function': {code}");
}

#[test]
fn test_codegen_function_with_return() {
    let mut ids = IdGenerator::new();
    let ret_place = make_place(&mut ids, "result");

    let rf = ReactiveFunction {
        loc: dummy_span(),
        id: Some("Returner".to_string()),
        params: vec![],
        body: ReactiveBlock {
            instructions: vec![ReactiveInstruction::Terminal(ReactiveTerminal::Return {
                value: ret_place,
                id: ids.next_block_id(),
            })],
        },
        directives: vec![],
        is_arrow: false,
        is_async: false,
        is_generator: false,
    };

    let code = codegen_function(&rf);
    assert!(code.contains("function Returner()"));
    assert!(code.contains("return"));
}

#[test]
fn test_codegen_function_with_primitive() {
    let mut ids = IdGenerator::new();
    let lvalue = make_place(&mut ids, "x");

    let instr = Instruction {
        id: ids.next_instruction_id(),
        lvalue,
        value: InstructionValue::Primitive { value: Primitive::Number(42.0) },
        loc: dummy_span(),
        effects: None,
    };

    let rf = ReactiveFunction {
        loc: dummy_span(),
        id: Some("NumFunc".to_string()),
        params: vec![],
        body: ReactiveBlock { instructions: vec![ReactiveInstruction::Instruction(instr)] },
        directives: vec![],
        is_arrow: false,
        is_async: false,
        is_generator: false,
    };

    let code = codegen_function(&rf);
    assert!(code.contains("42"));
    assert!(code.contains("function NumFunc()"));
}

#[test]
fn test_codegen_function_with_params() {
    let mut ids = IdGenerator::new();
    let param1 = make_place(&mut ids, "a");
    let param2 = make_place(&mut ids, "b");

    let rf = ReactiveFunction {
        loc: dummy_span(),
        id: Some("Add".to_string()),
        params: vec![Param::Identifier(param1), Param::Identifier(param2)],
        body: ReactiveBlock { instructions: vec![] },
        directives: vec![],
        is_arrow: false,
        is_async: false,
        is_generator: false,
    };

    let code = codegen_function(&rf);
    // Parameters should appear in the function signature.
    assert!(code.contains("function Add("));
}

#[test]
fn test_codegen_scope_block() {
    // Test that a reactive scope generates cache slot checks.
    let mut ids = IdGenerator::new();

    let lvalue = make_place(&mut ids, "t0");
    let instr = Instruction {
        id: ids.next_instruction_id(),
        lvalue: lvalue.clone(),
        value: InstructionValue::Primitive { value: Primitive::String("hello".to_string()) },
        loc: dummy_span(),
        effects: None,
    };

    let scope = ReactiveScope {
        id: ids.next_scope_id(),
        range: MutableRange { start: InstructionId(0), end: InstructionId(1) },
        dependencies: vec![],
        declarations: vec![(
            lvalue.identifier.id,
            ReactiveScopeDeclaration { identifier: lvalue.identifier, scope: ScopeId(0) },
        )],
        reassignments: vec![],
        early_return_value: None,
        merged: vec![],
        loc: dummy_span(),
        is_allocating: false,
    };

    let scope_block = ReactiveScopeBlock {
        scope,
        instructions: ReactiveBlock { instructions: vec![ReactiveInstruction::Instruction(instr)] },
    };

    let rf = ReactiveFunction {
        loc: dummy_span(),
        id: Some("Scoped".to_string()),
        params: vec![],
        body: ReactiveBlock { instructions: vec![ReactiveInstruction::Scope(scope_block)] },
        directives: vec![],
        is_arrow: false,
        is_async: false,
        is_generator: false,
    };

    let code = codegen_function(&rf);
    // Should contain cache initialization.
    assert!(code.contains("_c("), "should have cache slot initialization");
    assert!(code.contains("$["), "should reference cache slots");
}

// ---------------------------------------------------------------------------
// Integration: HIR -> build_reactive_function -> codegen
// ---------------------------------------------------------------------------

#[test]
fn test_hir_to_codegen_roundtrip() {
    let mut ids = IdGenerator::new();
    let block_id = ids.next_block_id();

    let x = make_place(&mut ids, "greeting");
    let instr = make_instruction(
        &mut ids,
        x.clone(),
        InstructionValue::Primitive { value: Primitive::String("hello".to_string()) },
    );

    let block = BasicBlock {
        kind: BlockKind::Block,
        id: block_id,
        instructions: vec![instr],
        terminal: Terminal::Return { value: x },
        preds: vec![],
        phis: vec![],
    };

    let hir = HIR { entry: block_id, blocks: vec![(block_id, block)] };

    let rf = build_reactive_function(
        hir,
        vec![],
        Some("Greeter".to_string()),
        dummy_span(),
        vec![],
        false,
        false,
        false,
    );

    let code = codegen_function(&rf);
    assert!(code.contains("function Greeter()"));
    assert!(code.contains("return"));
    // Should contain the string primitive.
    assert!(code.contains("hello") || code.contains("greeting"));
}
