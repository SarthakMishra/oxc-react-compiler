#![allow(dead_code)]

use std::borrow::Cow;

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
        output.push_str(&format!("function {name}("));
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
        output.push_str(&format!("  const $ = _c({total_slots});\n"));
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
                Primitive::BigInt(n) => format!("{n}n"),
            };
            output.push_str(&format!("{indent}const {lvalue_name} = {val_str};\n"));
        }
        InstructionValue::LoadLocal { place } => {
            let name = place_name(place);
            if name != lvalue_name {
                output.push_str(&format!("{indent}const {lvalue_name} = {name};\n"));
            }
        }
        InstructionValue::StoreLocal { lvalue: target, value, type_ } => {
            let target_name = place_name(target);
            let value_name = place_name(value);
            let keyword = match type_ {
                Some(crate::hir::types::InstructionKind::Const) => "const ",
                Some(crate::hir::types::InstructionKind::Let) => "let ",
                Some(crate::hir::types::InstructionKind::Var) => "var ",
                _ => "",
            };
            output.push_str(&format!("{indent}{keyword}{target_name} = {value_name};\n"));
        }
        InstructionValue::CallExpression { callee, args } => {
            let callee_name = place_name(callee);
            let args_str: Vec<Cow<'_, str>> = args.iter().map(place_name).collect();
            output.push_str(&format!(
                "{}const {} = {}({});\n",
                indent,
                lvalue_name,
                callee_name,
                args_str.join(", ")
            ));
        }
        InstructionValue::MethodCall { receiver, property, args } => {
            let receiver_name = place_name(receiver);
            let args_str: Vec<Cow<'_, str>> = args.iter().map(place_name).collect();
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
        InstructionValue::PropertyStore { object, property, value } => {
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
        InstructionValue::JsxExpression { tag, props, children } => {
            let tag_name = place_name(tag);
            output.push_str(&format!("{indent}const {lvalue_name} = <{tag_name}"));
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
                output.push_str(&format!("</{tag_name}>;\n"));
            }
        }
        InstructionValue::JsxFragment { children } => {
            output.push_str(&format!("{indent}const {lvalue_name} = <>"));
            for child in children {
                output.push_str(&format!("{{{}}}", place_name(child)));
            }
            output.push_str("</>;\n");
        }
        InstructionValue::ObjectExpression { properties } => {
            if properties.is_empty() {
                output.push_str(&format!("{indent}const {lvalue_name} = {{}};\n"));
            } else {
                output.push_str(&format!("{indent}const {lvalue_name} = {{ "));
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
            output.push_str(&format!("{indent}const {lvalue_name} = ["));
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
        InstructionValue::TemplateLiteral { quasis, subexpressions } => {
            output.push_str(&format!("{indent}const {lvalue_name} = `"));
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
            let args_str: Vec<Cow<'_, str>> = args.iter().map(place_name).collect();
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
        InstructionValue::Destructure { lvalue_pattern, value } => {
            let value_name = place_name(value);
            output.push_str(&format!("{indent}const "));
            codegen_destructure_pattern(lvalue_pattern, output);
            output.push_str(&format!(" = {value_name};\n"));
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
        ReactiveTerminal::If { test, consequent, alternate, .. } => {
            output.push_str(&format!("{}if ({}) {{\n", indent_str, place_name(test)));
            codegen_block(consequent, output, cache_slot, indent + 1);
            output.push_str(&format!("{indent_str}}} else {{\n"));
            codegen_block(alternate, output, cache_slot, indent + 1);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            output.push_str(&format!("{}switch ({}) {{\n", indent_str, place_name(test)));
            for (test_val, block) in cases {
                if let Some(tv) = test_val {
                    output.push_str(&format!("{}  case {}:\n", indent_str, place_name(tv)));
                } else {
                    output.push_str(&format!("{indent_str}  default:\n"));
                }
                codegen_block(block, output, cache_slot, indent + 2);
            }
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::While { test, body, .. } => {
            output.push_str(&format!("{indent_str}while (true) {{\n"));
            codegen_block(test, output, cache_slot, indent + 1);
            codegen_block(body, output, cache_slot, indent + 1);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::DoWhile { body, test, .. } => {
            output.push_str(&format!("{indent_str}do {{\n"));
            codegen_block(body, output, cache_slot, indent + 1);
            output.push_str(&format!("{indent_str}}} while (true);\n"));
            // Test block is evaluated inside the loop for condition
            let _ = test;
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            output.push_str(&format!("{indent_str}for (;;) {{\n"));
            codegen_block(init, output, cache_slot, indent + 1);
            codegen_block(test, output, cache_slot, indent + 1);
            codegen_block(body, output, cache_slot, indent + 1);
            if let Some(upd) = update {
                codegen_block(upd, output, cache_slot, indent + 1);
            }
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::ForOf { init, test, body, .. } => {
            output.push_str(&format!("{indent_str}for (const _ of _) {{\n"));
            codegen_block(init, output, cache_slot, indent + 1);
            codegen_block(test, output, cache_slot, indent + 1);
            codegen_block(body, output, cache_slot, indent + 1);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::ForIn { init, test, body, .. } => {
            output.push_str(&format!("{indent_str}for (const _ in _) {{\n"));
            codegen_block(init, output, cache_slot, indent + 1);
            codegen_block(test, output, cache_slot, indent + 1);
            codegen_block(body, output, cache_slot, indent + 1);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            output.push_str(&format!("{indent_str}try {{\n"));
            codegen_block(block, output, cache_slot, indent + 1);
            output.push_str(&format!("{indent_str}}} catch (e) {{\n"));
            codegen_block(handler, output, cache_slot, indent + 1);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Label { block, label, .. } => {
            output.push_str(&format!("{indent_str}bb{label}: {{\n"));
            codegen_block(block, output, cache_slot, indent + 1);
            output.push_str(&format!("{indent_str}}}\n"));
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
            "{indent_str}if ($[{slot_start}] === Symbol.for(\"react.memo_cache_sentinel\")) {{\n"
        ));
        *cache_slot += 1;
    } else {
        // Generate dep checks
        let checks: Vec<String> = deps
            .iter()
            .enumerate()
            .map(|(i, dep)| {
                let dep_name = identifier_display_name(&dep.identifier);
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
        let decl_name = identifier_display_name(&decl.identifier);
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
            let dep_name = identifier_display_name(&dep.identifier);
            output.push_str(&format!(
                "{}$[{}] = {};\n",
                inner_indent,
                slot_start + i as u32,
                dep_name
            ));
        }
    }

    output.push_str(&format!("{indent_str}}} else {{\n"));

    // Load cached declarations
    for (i, (_, decl)) in scope.scope.declarations.iter().enumerate() {
        let decl_name = identifier_display_name(&decl.identifier);
        let inner_indent = "  ".repeat(indent + 1);
        output.push_str(&format!(
            "{}{} = $[{}];\n",
            inner_indent,
            decl_name,
            decl_slot_start + i as u32
        ));
    }

    output.push_str(&format!("{indent_str}}}\n"));
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
        ReactiveTerminal::If { consequent, alternate, .. } => {
            count_cache_slots(consequent) + count_cache_slots(alternate)
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            count_cache_slots(init)
                + count_cache_slots(test)
                + update.as_ref().map_or(0, count_cache_slots)
                + count_cache_slots(body)
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            count_cache_slots(test) + count_cache_slots(body)
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            count_cache_slots(init) + count_cache_slots(test) + count_cache_slots(body)
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            count_cache_slots(block) + count_cache_slots(handler)
        }
        ReactiveTerminal::Switch { cases, .. } => {
            cases.iter().map(|(_, block)| count_cache_slots(block)).sum()
        }
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

// Optimization opportunity: this clones the String when a name exists and
// allocates via format!() for unnamed temporaries. Returning Cow<'_, str>
// would avoid both allocations (Cow::Borrowed for named, Cow::Owned for
// unnamed). However, many call sites collect into Vec<String> and feed into
// join()/format!(), so the signature change cascades widely. Deferred until
// profiling shows this is a measurable hotspot.
/// Resolve a Place's display name without allocating when a name exists.
/// Returns a borrowed `Cow` for named identifiers, avoiding the String clone
/// that the previous implementation performed on every call.
fn place_name(place: &Place) -> Cow<'_, str> {
    match &place.identifier.name {
        Some(name) => Cow::Borrowed(name.as_str()),
        None => Cow::Owned(format!("t{}", place.identifier.id.0)),
    }
}

/// Resolve an identifier's display name without allocating when a name exists.
fn identifier_display_name(identifier: &crate::hir::types::Identifier) -> Cow<'_, str> {
    match &identifier.name {
        Some(name) => Cow::Borrowed(name.as_str()),
        None => Cow::Owned(format!("t{}", identifier.id.0)),
    }
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
// Destructure pattern codegen
// ---------------------------------------------------------------------------

fn codegen_destructure_pattern(
    pattern: &crate::hir::types::DestructurePattern,
    output: &mut String,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};

    match pattern {
        DestructurePattern::Object { properties, rest } => {
            output.push_str("{ ");
            for (i, prop) in properties.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                match &prop.value {
                    DestructureTarget::Place(place) => {
                        let name = place_name(place);
                        if prop.shorthand && prop.key == *name {
                            output.push_str(&name);
                        } else {
                            output.push_str(&format!("{}: {}", prop.key, name));
                        }
                    }
                    DestructureTarget::Pattern(nested) => {
                        output.push_str(&format!("{}: ", prop.key));
                        codegen_destructure_pattern(nested, output);
                    }
                }
            }
            if let Some(rest_place) = rest {
                if !properties.is_empty() {
                    output.push_str(", ");
                }
                output.push_str(&format!("...{}", place_name(rest_place)));
            }
            output.push_str(" }");
        }
        DestructurePattern::Array { items, rest } => {
            output.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                match item {
                    DestructureArrayItem::Value(target) => match target {
                        DestructureTarget::Place(place) => {
                            output.push_str(&place_name(place));
                        }
                        DestructureTarget::Pattern(nested) => {
                            codegen_destructure_pattern(nested, output);
                        }
                    },
                    DestructureArrayItem::Hole => {
                        // Leave empty for hole
                    }
                    DestructureArrayItem::Spread(place) => {
                        output.push_str(&format!("...{}", place_name(place)));
                    }
                }
            }
            if let Some(rest_place) = rest {
                if !items.is_empty() {
                    output.push_str(", ");
                }
                output.push_str(&format!("...{}", place_name(rest_place)));
            }
            output.push(']');
        }
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
        Self { mappings: Vec::new() }
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

/// Encode source map mappings as a VLQ-encoded string suitable for the
/// `"mappings"` field of a v3 source map JSON.
///
/// Simplified encoding: each entry maps a generated position to an original
/// position within a single source file (source index 0).
impl SourceMap {
    pub fn to_vlq_mappings(&self) -> String {
        if self.mappings.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        let mut prev_gen_line: u32 = 0;
        let mut prev_gen_col: i64 = 0;
        let mut prev_orig_line: i64 = 0;
        let mut prev_orig_col: i64 = 0;
        let mut prev_source: i64 = 0;

        for entry in &self.mappings {
            // Emit semicolons for skipped lines.
            while prev_gen_line < entry.generated_line {
                result.push(';');
                prev_gen_line += 1;
                prev_gen_col = 0;
            }

            if !result.is_empty() && !result.ends_with(';') {
                result.push(',');
            }

            // Field 1: generated column (relative).
            let gen_col = i64::from(entry.generated_column);
            vlq_encode(&mut result, gen_col - prev_gen_col);
            prev_gen_col = gen_col;

            // Field 2: source index (relative, always 0).
            vlq_encode(&mut result, 0 - prev_source);
            prev_source = 0;

            // Field 3: original line (relative).
            let orig_line = i64::from(entry.original_line);
            vlq_encode(&mut result, orig_line - prev_orig_line);
            prev_orig_line = orig_line;

            // Field 4: original column (relative).
            let orig_col = i64::from(entry.original_column);
            vlq_encode(&mut result, orig_col - prev_orig_col);
            prev_orig_col = orig_col;
        }

        result
    }

    /// Serialize to a complete v3 source map JSON string.
    pub fn to_json(&self, file: &str, source_file: &str) -> String {
        format!(
            r#"{{"version":3,"file":"{}","sources":["{}"],"mappings":"{}"}}"#,
            file,
            source_file,
            self.to_vlq_mappings()
        )
    }
}

/// Encode a single signed integer as a VLQ string.
fn vlq_encode(output: &mut String, value: i64) {
    const VLQ_BASE_SHIFT: u32 = 5;
    const VLQ_BASE: i64 = 1 << VLQ_BASE_SHIFT; // 32
    const VLQ_BASE_MASK: i64 = VLQ_BASE - 1; // 31
    const VLQ_CONTINUATION_BIT: i64 = VLQ_BASE; // 32

    static VLQ_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    // Convert signed to VLQ-signed representation.
    let mut vlq = if value < 0 { ((-value) << 1) + 1 } else { value << 1 };

    loop {
        let mut digit = vlq & VLQ_BASE_MASK;
        vlq >>= VLQ_BASE_SHIFT;
        if vlq > 0 {
            digit |= VLQ_CONTINUATION_BIT;
        }
        output.push(VLQ_CHARS[digit as usize] as char);
        if vlq == 0 {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// CodegenContext: output buffer with position tracking
// ---------------------------------------------------------------------------

/// A code generation context that tracks output line/column positions
/// and builds a source map alongside the generated code.
struct CodegenContext {
    output: String,
    source_map: SourceMap,
    line: u32,
    column: u32,
}

impl CodegenContext {
    fn new() -> Self {
        Self { output: String::new(), source_map: SourceMap::new(), line: 0, column: 0 }
    }

    /// Write a string to the output buffer, updating line/column tracking.
    fn write(&mut self, s: &str) {
        for ch in s.chars() {
            self.output.push(ch);
            if ch == '\n' {
                self.line += 1;
                self.column = 0;
            } else {
                self.column += 1;
            }
        }
    }

    /// Record a source mapping from the current generated position to the
    /// given original source position (0-based line and column from Span).
    fn map_from_span(&mut self, span: oxc_span::Span, source: &str) {
        if span.start == 0 && span.end == 0 {
            return; // Dummy span, skip.
        }
        // Convert byte offset to line/column.
        let (orig_line, orig_col) = byte_offset_to_line_col(source, span.start as usize);
        self.source_map.add_mapping(self.line, self.column, orig_line as u32, orig_col as u32);
    }
}

/// Convert a byte offset in source text to 0-based (line, column).
fn byte_offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Generate JavaScript code with a source map from a ReactiveFunction.
///
/// Like `codegen_function` but also produces a `SourceMap` that maps
/// generated positions back to positions in `source_text`.
pub fn codegen_function_with_source_map(
    rf: &ReactiveFunction,
    source_text: &str,
) -> (String, SourceMap) {
    let mut ctx = CodegenContext::new();

    // Map function start to original span.
    ctx.map_from_span(rf.loc, source_text);

    // Generate function header.
    if let Some(ref name) = rf.id {
        ctx.write(&format!("function {name}("));
    } else {
        ctx.write("function (");
    }

    for (i, param) in rf.params.iter().enumerate() {
        if i > 0 {
            ctx.write(", ");
        }
        match param {
            crate::hir::types::Param::Identifier(place) => {
                ctx.write(&place_name(place));
            }
            crate::hir::types::Param::Spread(place) => {
                ctx.write("...");
                ctx.write(&place_name(place));
            }
        }
    }
    ctx.write(") {\n");

    let total_slots = count_cache_slots(&rf.body);
    if total_slots > 0 {
        ctx.write(&format!("  const $ = _c({total_slots});\n"));
    }

    codegen_block_with_map(&rf.body, &mut ctx, &mut 0u32, 1, source_text);

    ctx.write("}\n");
    (ctx.output, ctx.source_map)
}

fn codegen_block_with_map(
    block: &ReactiveBlock,
    ctx: &mut CodegenContext,
    cache_slot: &mut u32,
    indent: usize,
    source: &str,
) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                let indent_str = "  ".repeat(indent);
                // Record mapping for this instruction.
                ctx.map_from_span(instruction.loc, source);
                codegen_instruction(instruction, &mut ctx.output, &indent_str);
                // Update line/col tracking after codegen_instruction wrote to output.
                recompute_position(ctx);
            }
            ReactiveInstruction::Terminal(terminal) => {
                codegen_terminal(terminal, &mut ctx.output, cache_slot, indent);
                recompute_position(ctx);
            }
            ReactiveInstruction::Scope(scope_block) => {
                codegen_scope(scope_block, &mut ctx.output, cache_slot, indent);
                recompute_position(ctx);
            }
        }
    }
}

/// Recompute line/column position from the output string.
/// Called after delegating to functions that write directly to the String.
fn recompute_position(ctx: &mut CodegenContext) {
    let mut line = 0u32;
    let mut col = 0u32;
    for ch in ctx.output.chars() {
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    ctx.line = line;
    ctx.column = col;
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
