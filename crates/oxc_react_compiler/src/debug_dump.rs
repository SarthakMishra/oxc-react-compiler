//! Debug IR dumping for differential tracing.
//!
//! Enable by setting `DEBUG_IR=1` environment variable.
//! Dumps intermediate compiler state at key pass boundaries.

// This module intentionally writes diagnostic output to stderr.
#![allow(clippy::print_stderr)]

use crate::hir::types::{
    HIR, InstructionValue, ReactiveBlock, ReactiveFunction, ReactiveInstruction, ReactiveTerminal,
    Terminal,
};

/// Check if debug IR dumping is enabled.
pub fn is_debug_ir_enabled() -> bool {
    std::env::var("DEBUG_IR").is_ok()
}

/// Dump a summary of the HIR state at a pass boundary.
pub fn dump_hir_summary(hir: &HIR, pass_name: &str) {
    if !is_debug_ir_enabled() {
        return;
    }
    eprintln!("=== {pass_name} ===");
    eprintln!("  blocks: {}", hir.blocks.len());
    let total_instrs: usize = hir.blocks.iter().map(|(_, b)| b.instructions.len()).sum();
    eprintln!("  instructions: {total_instrs}");
    for (block_id, block) in &hir.blocks {
        let term_name = terminal_kind_name(&block.terminal);
        eprintln!(
            "  block {} ({} instrs, term={})",
            block_id.0,
            block.instructions.len(),
            term_name
        );
        for instr in &block.instructions {
            let name = instr.lvalue.identifier.name.as_deref().unwrap_or("_");
            let range = instr.lvalue.identifier.mutable_range;
            let scope = instr.lvalue.identifier.scope.as_ref().map(|s| s.id.0);
            let kind = instruction_kind_name(&instr.value);
            eprintln!(
                "    [{}] {kind} -> {name} (id={}, range=[{},{}), scope={:?})",
                instr.id.0, instr.lvalue.identifier.id.0, range.start.0, range.end.0, scope
            );
        }
    }
}

/// Dump a summary of the reactive function.
pub fn dump_rf_summary(rf: &ReactiveFunction, pass_name: &str) {
    if !is_debug_ir_enabled() {
        return;
    }
    eprintln!("=== {pass_name} (reactive function) ===");
    eprintln!("  id: {:?}", rf.id);
    eprintln!("  params: {}", rf.params.len());
    let (scopes, instrs) = count_rf_nodes(&rf.body);
    eprintln!("  scope blocks: {scopes}");
    eprintln!("  instructions: {instrs}");
}

fn count_rf_nodes(block: &ReactiveBlock) -> (usize, usize) {
    let mut scopes = 0usize;
    let mut instrs = 0usize;
    for item in &block.instructions {
        match item {
            ReactiveInstruction::Instruction(_) => instrs += 1,
            ReactiveInstruction::Terminal(term) => {
                let (s, i) = count_terminal(term);
                scopes += s;
                instrs += i;
            }
            ReactiveInstruction::Scope(sb) => {
                scopes += 1;
                let (s, i) = count_rf_nodes(&sb.instructions);
                scopes += s;
                instrs += i;
            }
        }
    }
    (scopes, instrs)
}

fn count_terminal(term: &ReactiveTerminal) -> (usize, usize) {
    let mut blocks: Vec<&ReactiveBlock> = Vec::new();
    match term {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            blocks.push(consequent);
            blocks.push(alternate);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, b) in cases {
                blocks.push(b);
            }
        }
        ReactiveTerminal::For { body, init, test, update, .. } => {
            blocks.push(body);
            blocks.push(init);
            blocks.push(test);
            if let Some(u) = update {
                blocks.push(u);
            }
        }
        ReactiveTerminal::ForOf { body, init, test, .. }
        | ReactiveTerminal::ForIn { body, init, test, .. } => {
            blocks.push(body);
            blocks.push(init);
            blocks.push(test);
        }
        ReactiveTerminal::While { body, test, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            blocks.push(body);
            blocks.push(test);
        }
        ReactiveTerminal::Label { block, .. } => {
            blocks.push(block);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            blocks.push(block);
            blocks.push(handler);
        }
        ReactiveTerminal::Logical { right, .. } => {
            blocks.push(right);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
    }
    let mut s = 0;
    let mut i = 0;
    for b in blocks {
        let (s2, i2) = count_rf_nodes(b);
        s += s2;
        i += i2;
    }
    (s, i)
}

fn instruction_kind_name(value: &InstructionValue) -> &'static str {
    match value {
        InstructionValue::DeclareLocal { .. } => "DeclareLocal",
        InstructionValue::DeclareContext { .. } => "DeclareContext",
        InstructionValue::StoreLocal { .. } => "StoreLocal",
        InstructionValue::StoreContext { .. } => "StoreContext",
        InstructionValue::StoreGlobal { .. } => "StoreGlobal",
        InstructionValue::LoadLocal { .. } => "LoadLocal",
        InstructionValue::LoadContext { .. } => "LoadContext",
        InstructionValue::LoadGlobal { .. } => "LoadGlobal",
        InstructionValue::Primitive { .. } => "Primitive",
        InstructionValue::JSXText { .. } => "JSXText",
        InstructionValue::RegExpLiteral { .. } => "RegExpLiteral",
        InstructionValue::CallExpression { .. } => "CallExpression",
        InstructionValue::NewExpression { .. } => "NewExpression",
        InstructionValue::MethodCall { .. } => "MethodCall",
        InstructionValue::PropertyLoad { .. } => "PropertyLoad",
        InstructionValue::PropertyStore { .. } => "PropertyStore",
        InstructionValue::PropertyDelete { .. } => "PropertyDelete",
        InstructionValue::ComputedLoad { .. } => "ComputedLoad",
        InstructionValue::ComputedStore { .. } => "ComputedStore",
        InstructionValue::ComputedDelete { .. } => "ComputedDelete",
        InstructionValue::BinaryExpression { .. } => "BinaryExpression",
        InstructionValue::UnaryExpression { .. } => "UnaryExpression",
        InstructionValue::Await { .. } => "Await",
        InstructionValue::TypeCastExpression { .. } => "TypeCastExpression",
        InstructionValue::ObjectExpression { .. } => "ObjectExpression",
        InstructionValue::ArrayExpression { .. } => "ArrayExpression",
        InstructionValue::JsxExpression { .. } => "JsxExpression",
        InstructionValue::JsxFragment { .. } => "JsxFragment",
        InstructionValue::TemplateLiteral { .. } => "TemplateLiteral",
        InstructionValue::TaggedTemplateExpression { .. } => "TaggedTemplateExpression",
        InstructionValue::FunctionExpression { .. } => "FunctionExpression",
        InstructionValue::ObjectMethod { .. } => "ObjectMethod",
        InstructionValue::Destructure { .. } => "Destructure",
        InstructionValue::PrefixUpdate { .. } => "PrefixUpdate",
        InstructionValue::PostfixUpdate { .. } => "PostfixUpdate",
        InstructionValue::GetIterator { .. } => "GetIterator",
        InstructionValue::IteratorNext { .. } => "IteratorNext",
        InstructionValue::NextPropertyOf { .. } => "NextPropertyOf",
        InstructionValue::StartMemoize { .. } => "StartMemoize",
        InstructionValue::FinishMemoize { .. } => "FinishMemoize",
        InstructionValue::UnsupportedNode { .. } => "UnsupportedNode",
    }
}

fn terminal_kind_name(terminal: &Terminal) -> &'static str {
    match terminal {
        Terminal::Goto { .. } => "Goto",
        Terminal::If { .. } => "If",
        Terminal::Branch { .. } => "Branch",
        Terminal::Switch { .. } => "Switch",
        Terminal::Return { .. } => "Return",
        Terminal::Throw { .. } => "Throw",
        Terminal::For { .. } => "For",
        Terminal::ForOf { .. } => "ForOf",
        Terminal::ForIn { .. } => "ForIn",
        Terminal::While { .. } => "While",
        Terminal::DoWhile { .. } => "DoWhile",
        Terminal::Ternary { .. } => "Ternary",
        Terminal::Logical { .. } => "Logical",
        Terminal::Sequence { .. } => "Sequence",
        Terminal::Optional { .. } => "Optional",
        Terminal::Label { .. } => "Label",
        Terminal::Try { .. } => "Try",
        Terminal::Scope { .. } => "Scope",
        Terminal::PrunedScope { .. } => "PrunedScope",
        Terminal::MaybeThrow { .. } => "MaybeThrow",
        Terminal::Unreachable => "Unreachable",
    }
}

// Aliases used by pipeline.rs
pub fn is_enabled() -> bool {
    is_debug_ir_enabled()
}
pub fn dump_scopes(hir: &HIR, pass_name: &str) {
    dump_hir_summary(hir, pass_name);
}
pub fn dump_reactive_function(rf: &ReactiveFunction, pass_name: &str) {
    dump_rf_summary(rf, pass_name);
}
