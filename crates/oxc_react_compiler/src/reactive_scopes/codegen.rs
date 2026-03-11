#![allow(dead_code)]

use crate::hir::types::{
    InstructionValue, Place, Primitive, ReactiveBlock, ReactiveFunction, ReactiveInstruction,
    ReactiveScopeBlock, ReactiveTerminal,
};

/// Generate JavaScript code from a ReactiveFunction.
///
/// This is the final pass that produces the compiled output.
/// It generates code with `useMemoCache` calls and conditional blocks.
pub fn codegen_function(rf: &ReactiveFunction) -> String {
    let mut output = String::new();
    let mut cache_slot = 0u32;

    // Generate function header
    if let Some(ref name) = rf.id {
        output.push_str(&format!("function {}(", name));
    } else {
        output.push_str("function (");
    }

    // Generate parameters
    for (i, param) in rf.params.iter().enumerate() {
        if i > 0 {
            output.push_str(", ");
        }
        match param {
            crate::hir::types::Param::Identifier(place) => {
                output.push_str(&place_name(place));
            }
            crate::hir::types::Param::Spread(place) => {
                output.push_str("...");
                output.push_str(&place_name(place));
            }
        }
    }
    output.push_str(") {\n");

    // Count total cache slots needed
    let total_slots = count_cache_slots(&rf.body);
    if total_slots > 0 {
        output.push_str(&format!("  const $ = _c({});\n", total_slots));
    }

    // Generate body
    codegen_block(&rf.body, &mut output, &mut cache_slot, 1);

    output.push_str("}\n");
    output
}

fn codegen_block(block: &ReactiveBlock, output: &mut String, cache_slot: &mut u32, indent: usize) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                let indent_str = "  ".repeat(indent);
                codegen_instruction(instruction, output, &indent_str);
            }
            ReactiveInstruction::Terminal(terminal) => {
                codegen_terminal(terminal, output, cache_slot, indent);
            }
            ReactiveInstruction::Scope(scope_block) => {
                codegen_scope(scope_block, output, cache_slot, indent);
            }
        }
    }
}

fn codegen_instruction(instr: &crate::hir::types::Instruction, output: &mut String, indent: &str) {
    let lvalue_name = place_name(&instr.lvalue);

    match &instr.value {
        InstructionValue::Primitive { value } => {
            let val_str = match value {
                Primitive::Null => "null".to_string(),
                Primitive::Undefined => "undefined".to_string(),
                Primitive::Boolean(b) => b.to_string(),
                Primitive::Number(n) => n.to_string(),
                Primitive::String(s) => format!("\"{}\"", s.replace('\"', "\\\"")),
                Primitive::BigInt(n) => format!("{}n", n),
            };
            output.push_str(&format!("{}const {} = {};\n", indent, lvalue_name, val_str));
        }
        InstructionValue::LoadLocal { place } => {
            let name = place_name(place);
            if name != lvalue_name {
                output.push_str(&format!("{}const {} = {};\n", indent, lvalue_name, name));
            }
        }
        InstructionValue::StoreLocal {
            lvalue: target,
            value,
            type_,
        } => {
            let target_name = place_name(target);
            let value_name = place_name(value);
            let keyword = match type_ {
                Some(crate::hir::types::InstructionKind::Const) => "const ",
                Some(crate::hir::types::InstructionKind::Let) => "let ",
                Some(crate::hir::types::InstructionKind::Var) => "var ",
                _ => "",
            };
            output.push_str(&format!(
                "{}{}{} = {};\n",
                indent, keyword, target_name, value_name
            ));
        }
        InstructionValue::CallExpression { callee, args } => {
            let callee_name = place_name(callee);
            let args_str: Vec<String> = args.iter().map(|a| place_name(a)).collect();
            output.push_str(&format!(
                "{}const {} = {}({});\n",
                indent,
                lvalue_name,
                callee_name,
                args_str.join(", ")
            ));
        }
        InstructionValue::MethodCall {
            receiver,
            property,
            args,
        } => {
            let receiver_name = place_name(receiver);
            let args_str: Vec<String> = args.iter().map(|a| place_name(a)).collect();
            output.push_str(&format!(
                "{}const {} = {}.{}({});\n",
                indent,
                lvalue_name,
                receiver_name,
                property,
                args_str.join(", ")
            ));
        }
        InstructionValue::PropertyLoad { object, property } => {
            output.push_str(&format!(
                "{}const {} = {}.{};\n",
                indent,
                lvalue_name,
                place_name(object),
                property
            ));
        }
        InstructionValue::PropertyStore {
            object,
            property,
            value,
        } => {
            output.push_str(&format!(
                "{}{}.{} = {};\n",
                indent,
                place_name(object),
                property,
                place_name(value)
            ));
        }
        InstructionValue::BinaryExpression { op, left, right } => {
            let op_str = binary_op_str(*op);
            output.push_str(&format!(
                "{}const {} = {} {} {};\n",
                indent,
                lvalue_name,
                place_name(left),
                op_str,
                place_name(right)
            ));
        }
        InstructionValue::UnaryExpression { op, value } => {
            let op_str = unary_op_str(*op);
            output.push_str(&format!(
                "{}const {} = {}{};\n",
                indent,
                lvalue_name,
                op_str,
                place_name(value)
            ));
        }
        InstructionValue::JsxExpression {
            tag,
            props,
            children,
        } => {
            let tag_name = place_name(tag);
            output.push_str(&format!("{}const {} = <{}", indent, lvalue_name, tag_name));
            for attr in props {
                match &attr.name {
                    crate::hir::types::JsxAttributeName::Named(name) => {
                        output.push_str(&format!(" {}={{{}}}", name, place_name(&attr.value)));
                    }
                    crate::hir::types::JsxAttributeName::Spread => {
                        output.push_str(&format!(" {{...{}}}", place_name(&attr.value)));
                    }
                }
            }
            if children.is_empty() {
                output.push_str(" />;\n");
            } else {
                output.push('>');
                for child in children {
                    output.push_str(&format!("{{{}}}", place_name(child)));
                }
                output.push_str(&format!("</{}>;\n", tag_name));
            }
        }
        InstructionValue::JsxFragment { children } => {
            output.push_str(&format!("{}const {} = <>", indent, lvalue_name));
            for child in children {
                output.push_str(&format!("{{{}}}", place_name(child)));
            }
            output.push_str("</>;\n");
        }
        InstructionValue::ObjectExpression { properties } => {
            if properties.is_empty() {
                output.push_str(&format!("{}const {} = {{}};\n", indent, lvalue_name));
            } else {
                output.push_str(&format!("{}const {} = {{ ", indent, lvalue_name));
                for (i, prop) in properties.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    match &prop.key {
                        crate::hir::types::ObjectPropertyKey::Identifier(name) => {
                            if prop.shorthand {
                                output.push_str(name);
                            } else {
                                output.push_str(&format!("{}: {}", name, place_name(&prop.value)));
                            }
                        }
                        crate::hir::types::ObjectPropertyKey::Computed(key) => {
                            output.push_str(&format!(
                                "[{}]: {}",
                                place_name(key),
                                place_name(&prop.value)
                            ));
                        }
                    }
                }
                output.push_str(" };\n");
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            output.push_str(&format!("{}const {} = [", indent, lvalue_name));
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                match elem {
                    crate::hir::types::ArrayElement::Expression(p) => {
                        output.push_str(&place_name(p));
                    }
                    crate::hir::types::ArrayElement::Spread(p) => {
                        output.push_str(&format!("...{}", place_name(p)));
                    }
                    crate::hir::types::ArrayElement::Hole => {
                        // Empty for hole
                    }
                }
            }
            output.push_str("];\n");
        }
        InstructionValue::TemplateLiteral {
            quasis,
            subexpressions,
        } => {
            output.push_str(&format!("{}const {} = `", indent, lvalue_name));
            for (i, quasi) in quasis.iter().enumerate() {
                output.push_str(quasi);
                if i < subexpressions.len() {
                    output.push_str(&format!("${{{}}}", place_name(&subexpressions[i])));
                }
            }
            output.push_str("`;\n");
        }
        InstructionValue::NewExpression { callee, args } => {
            let callee_name = place_name(callee);
            let args_str: Vec<String> = args.iter().map(|a| place_name(a)).collect();
            output.push_str(&format!(
                "{}const {} = new {}({});\n",
                indent,
                lvalue_name,
                callee_name,
                args_str.join(", ")
            ));
        }
        InstructionValue::Await { value } => {
            output.push_str(&format!(
                "{}const {} = await {};\n",
                indent,
                lvalue_name,
                place_name(value)
            ));
        }
        _ => {
            // Generic instruction codegen — emit as comment for unsupported patterns
            output.push_str(&format!(
                "{}/* {} = {:?} */\n",
                indent,
                lvalue_name,
                std::mem::discriminant(&instr.value)
            ));
        }
    }
}

fn codegen_terminal(
    terminal: &ReactiveTerminal,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
) {
    let indent_str = "  ".repeat(indent);

    match terminal {
        ReactiveTerminal::Return { value, .. } => {
            output.push_str(&format!("{}return {};\n", indent_str, place_name(value)));
        }
        ReactiveTerminal::Throw { value, .. } => {
            output.push_str(&format!("{}throw {};\n", indent_str, place_name(value)));
        }
        ReactiveTerminal::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            output.push_str(&format!("{}if ({}) {{\n", indent_str, place_name(test)));
            codegen_block(consequent, output, cache_slot, indent + 1);
            output.push_str(&format!("{}}} else {{\n", indent_str));
            codegen_block(alternate, output, cache_slot, indent + 1);
            output.push_str(&format!("{}}}\n", indent_str));
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            output.push_str(&format!("{}switch ({}) {{\n", indent_str, place_name(test)));
            for (test_val, block) in cases {
                if let Some(tv) = test_val {
                    output.push_str(&format!("{}  case {}:\n", indent_str, place_name(tv)));
                } else {
                    output.push_str(&format!("{}  default:\n", indent_str));
                }
                codegen_block(block, output, cache_slot, indent + 2);
            }
            output.push_str(&format!("{}}}\n", indent_str));
        }
        ReactiveTerminal::While { test, body, .. } => {
            output.push_str(&format!("{}while (true) {{\n", indent_str));
            codegen_block(test, output, cache_slot, indent + 1);
            codegen_block(body, output, cache_slot, indent + 1);
            output.push_str(&format!("{}}}\n", indent_str));
        }
        ReactiveTerminal::DoWhile { body, test, .. } => {
            output.push_str(&format!("{}do {{\n", indent_str));
            codegen_block(body, output, cache_slot, indent + 1);
            output.push_str(&format!("{}}} while (true);\n", indent_str));
            // Test block is evaluated inside the loop for condition
            let _ = test;
        }
        ReactiveTerminal::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            output.push_str(&format!("{}for (;;) {{\n", indent_str));
            codegen_block(init, output, cache_slot, indent + 1);
            codegen_block(test, output, cache_slot, indent + 1);
            codegen_block(body, output, cache_slot, indent + 1);
            if let Some(upd) = update {
                codegen_block(upd, output, cache_slot, indent + 1);
            }
            output.push_str(&format!("{}}}\n", indent_str));
        }
        ReactiveTerminal::ForOf {
            init, test, body, ..
        } => {
            output.push_str(&format!("{}for (const _ of _) {{\n", indent_str));
            codegen_block(init, output, cache_slot, indent + 1);
            codegen_block(test, output, cache_slot, indent + 1);
            codegen_block(body, output, cache_slot, indent + 1);
            output.push_str(&format!("{}}}\n", indent_str));
        }
        ReactiveTerminal::ForIn {
            init, test, body, ..
        } => {
            output.push_str(&format!("{}for (const _ in _) {{\n", indent_str));
            codegen_block(init, output, cache_slot, indent + 1);
            codegen_block(test, output, cache_slot, indent + 1);
            codegen_block(body, output, cache_slot, indent + 1);
            output.push_str(&format!("{}}}\n", indent_str));
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            output.push_str(&format!("{}try {{\n", indent_str));
            codegen_block(block, output, cache_slot, indent + 1);
            output.push_str(&format!("{}}} catch (e) {{\n", indent_str));
            codegen_block(handler, output, cache_slot, indent + 1);
            output.push_str(&format!("{}}}\n", indent_str));
        }
        ReactiveTerminal::Label { block, label, .. } => {
            output.push_str(&format!("{}bb{}: {{\n", indent_str, label));
            codegen_block(block, output, cache_slot, indent + 1);
            output.push_str(&format!("{}}}\n", indent_str));
        }
    }
}

fn codegen_scope(
    scope: &ReactiveScopeBlock,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
) {
    let indent_str = "  ".repeat(indent);
    let deps = &scope.scope.dependencies;
    let slot_start = *cache_slot;

    // Generate dependency check
    if deps.is_empty() {
        // Constant scope — check sentinel
        output.push_str(&format!(
            "{}if ($[{}] === Symbol.for(\"react.memo_cache_sentinel\")) {{\n",
            indent_str, slot_start
        ));
        *cache_slot += 1;
    } else {
        // Generate dep checks
        let checks: Vec<String> = deps
            .iter()
            .enumerate()
            .map(|(i, dep)| {
                let dep_name = dep
                    .identifier
                    .name
                    .as_deref()
                    .map_or_else(|| format!("_t{}", dep.identifier.id.0), |n| n.to_string());
                format!("$[{}] !== {}", slot_start + i as u32, dep_name)
            })
            .collect();
        output.push_str(&format!("{}if ({}) {{\n", indent_str, checks.join(" || ")));
        *cache_slot += deps.len() as u32;
    }

    // Generate scope body
    codegen_block(&scope.instructions, output, cache_slot, indent + 1);

    // Store declarations into cache slots
    let decl_slot_start = *cache_slot;
    for (i, (_, decl)) in scope.scope.declarations.iter().enumerate() {
        let decl_name = decl
            .identifier
            .name
            .as_deref()
            .map_or_else(|| format!("_t{}", decl.identifier.id.0), |n| n.to_string());
        let inner_indent = "  ".repeat(indent + 1);
        output.push_str(&format!(
            "{}$[{}] = {};\n",
            inner_indent,
            decl_slot_start + i as u32,
            decl_name
        ));
    }
    *cache_slot += scope.scope.declarations.len() as u32;

    // Store dep values for next comparison
    if !deps.is_empty() {
        let inner_indent = "  ".repeat(indent + 1);
        for (i, dep) in deps.iter().enumerate() {
            let dep_name = dep
                .identifier
                .name
                .as_deref()
                .map_or_else(|| format!("_t{}", dep.identifier.id.0), |n| n.to_string());
            output.push_str(&format!(
                "{}$[{}] = {};\n",
                inner_indent,
                slot_start + i as u32,
                dep_name
            ));
        }
    }

    output.push_str(&format!("{}}} else {{\n", indent_str));

    // Load cached declarations
    for (i, (_, decl)) in scope.scope.declarations.iter().enumerate() {
        let decl_name = decl
            .identifier
            .name
            .as_deref()
            .map_or_else(|| format!("_t{}", decl.identifier.id.0), |n| n.to_string());
        let inner_indent = "  ".repeat(indent + 1);
        output.push_str(&format!(
            "{}{} = $[{}];\n",
            inner_indent,
            decl_name,
            decl_slot_start + i as u32
        ));
    }

    output.push_str(&format!("{}}}\n", indent_str));
}

fn count_cache_slots(block: &ReactiveBlock) -> u32 {
    let mut count = 0u32;
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Scope(scope) => {
                // Slots for deps + slots for declarations
                let dep_slots = scope.scope.dependencies.len().max(1) as u32;
                let decl_slots = scope.scope.declarations.len() as u32;
                count += dep_slots + decl_slots;
                count += count_cache_slots(&scope.instructions);
            }
            ReactiveInstruction::Terminal(terminal) => {
                count += count_terminal_slots(terminal);
            }
            ReactiveInstruction::Instruction(_) => {}
        }
    }
    count
}

fn count_terminal_slots(terminal: &ReactiveTerminal) -> u32 {
    match terminal {
        ReactiveTerminal::If {
            consequent,
            alternate,
            ..
        } => count_cache_slots(consequent) + count_cache_slots(alternate),
        ReactiveTerminal::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            count_cache_slots(init)
                + count_cache_slots(test)
                + update.as_ref().map_or(0, count_cache_slots)
                + count_cache_slots(body)
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            count_cache_slots(test) + count_cache_slots(body)
        }
        ReactiveTerminal::ForOf {
            init, test, body, ..
        }
        | ReactiveTerminal::ForIn {
            init, test, body, ..
        } => count_cache_slots(init) + count_cache_slots(test) + count_cache_slots(body),
        ReactiveTerminal::Try { block, handler, .. } => {
            count_cache_slots(block) + count_cache_slots(handler)
        }
        ReactiveTerminal::Switch { cases, .. } => cases
            .iter()
            .map(|(_, block)| count_cache_slots(block))
            .sum(),
        ReactiveTerminal::Label { block, .. } => count_cache_slots(block),
        ReactiveTerminal::Return { .. } | ReactiveTerminal::Throw { .. } => 0,
    }
}

/// Generate the import statement for the compiler runtime.
pub fn generate_import_statement() -> String {
    "import { c as _c } from \"react/compiler-runtime\";\n".to_string()
}

/// Apply compiled function to original source code.
/// Returns the new source with import added and function bodies replaced.
pub fn apply_compilation(
    original_source: &str,
    compiled_functions: &[(oxc_span::Span, String)],
) -> String {
    if compiled_functions.is_empty() {
        return original_source.to_string();
    }

    let mut result = String::with_capacity(original_source.len() + 256);

    // Add import at the top
    result.push_str(&generate_import_statement());

    // Apply edits in reverse order (to preserve offsets)
    let mut edits: Vec<(usize, usize, &str)> = compiled_functions
        .iter()
        .map(|(span, code)| (span.start as usize, span.end as usize, code.as_str()))
        .collect();
    edits.sort_by(|a, b| b.0.cmp(&a.0));

    let mut source = original_source.to_string();
    for (start, end, replacement) in &edits {
        source.replace_range(*start..*end, replacement);
    }

    result.push_str(&source);
    result
}

fn place_name(place: &Place) -> String {
    place
        .identifier
        .name
        .clone()
        .unwrap_or_else(|| format!("_t{}", place.identifier.id.0))
}

fn binary_op_str(op: crate::hir::types::BinaryOp) -> &'static str {
    use crate::hir::types::BinaryOp;
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Exp => "**",
        BinaryOp::BitwiseAnd => "&",
        BinaryOp::BitwiseOr => "|",
        BinaryOp::BitwiseXor => "^",
        BinaryOp::ShiftLeft => "<<",
        BinaryOp::ShiftRight => ">>",
        BinaryOp::UnsignedShiftRight => ">>>",
        BinaryOp::EqEq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::StrictEq => "===",
        BinaryOp::StrictNotEq => "!==",
        BinaryOp::Lt => "<",
        BinaryOp::LtEq => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::GtEq => ">=",
        BinaryOp::In => "in",
        BinaryOp::InstanceOf => "instanceof",
        BinaryOp::NullishCoalescing => "??",
    }
}

// ---------------------------------------------------------------------------
// Source map generation
// ---------------------------------------------------------------------------

/// Basic source map generation.
/// Maps output positions back to original source positions.
#[derive(Debug, Clone)]
pub struct SourceMap {
    pub mappings: Vec<SourceMapEntry>,
}

#[derive(Debug, Clone)]
pub struct SourceMapEntry {
    pub generated_line: u32,
    pub generated_column: u32,
    pub original_line: u32,
    pub original_column: u32,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    pub fn add_mapping(&mut self, gen_line: u32, gen_col: u32, orig_line: u32, orig_col: u32) {
        self.mappings.push(SourceMapEntry {
            generated_line: gen_line,
            generated_column: gen_col,
            original_line: orig_line,
            original_column: orig_col,
        });
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

fn unary_op_str(op: crate::hir::types::UnaryOp) -> &'static str {
    use crate::hir::types::UnaryOp;
    match op {
        UnaryOp::Minus => "-",
        UnaryOp::Plus => "+",
        UnaryOp::Not => "!",
        UnaryOp::BitwiseNot => "~",
        UnaryOp::TypeOf => "typeof ",
        UnaryOp::Void => "void ",
        UnaryOp::Delete => "delete ",
    }
}
