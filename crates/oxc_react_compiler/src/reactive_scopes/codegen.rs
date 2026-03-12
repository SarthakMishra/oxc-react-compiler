#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::HashSet;

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

    // Track variables declared via DeclareLocal so that subsequent
    // StoreLocal / Destructure can emit bare assignments instead of
    // re-declaring with const/let.
    let mut declared = HashSet::new();

    // Collect parameter names for destructuring hoisting
    let param_names: HashSet<String> = rf
        .params
        .iter()
        .map(|p| match p {
            crate::hir::types::Param::Identifier(place) => place_name(place).to_string(),
            crate::hir::types::Param::Spread(place) => place_name(place).to_string(),
        })
        .collect();

    // Hoist Destructure instructions that destructure from function parameters
    // (e.g., `const { status } = t0;`) to the top of the function body,
    // before any reactive scope checks that may reference those variables.
    let mut hoisted_indices = HashSet::new();
    for (i, instr) in rf.body.instructions.iter().enumerate() {
        if let ReactiveInstruction::Instruction(instruction) = instr {
            if let InstructionValue::Destructure { value, .. } = &instruction.value {
                let value_name = place_name(value);
                if param_names.contains(value_name.as_ref()) {
                    let indent_str = "  ";
                    codegen_instruction(instruction, &mut output, indent_str, &mut declared);
                    hoisted_indices.insert(i);
                }
            }
        }
    }

    // Generate body, skipping hoisted instructions
    codegen_block_skip_hoisted(
        &rf.body,
        &mut output,
        &mut cache_slot,
        1,
        &mut declared,
        &hoisted_indices,
    );

    output.push_str("}\n");
    output
}

fn codegen_block(
    block: &ReactiveBlock,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
    declared: &mut HashSet<String>,
) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                let indent_str = "  ".repeat(indent);
                codegen_instruction(instruction, output, &indent_str, declared);
            }
            ReactiveInstruction::Terminal(terminal) => {
                codegen_terminal(terminal, output, cache_slot, indent, declared);
            }
            ReactiveInstruction::Scope(scope_block) => {
                codegen_scope(scope_block, output, cache_slot, indent, declared);
            }
        }
    }
}

/// Like `codegen_block` but skips instructions at specific indices (already hoisted).
fn codegen_block_skip_hoisted(
    block: &ReactiveBlock,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
    declared: &mut HashSet<String>,
    hoisted_indices: &HashSet<usize>,
) {
    for (i, instr) in block.instructions.iter().enumerate() {
        if hoisted_indices.contains(&i) {
            continue;
        }
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                let indent_str = "  ".repeat(indent);
                codegen_instruction(instruction, output, &indent_str, declared);
            }
            ReactiveInstruction::Terminal(terminal) => {
                codegen_terminal(terminal, output, cache_slot, indent, declared);
            }
            ReactiveInstruction::Scope(scope_block) => {
                codegen_scope(scope_block, output, cache_slot, indent, declared);
            }
        }
    }
}

/// Like `codegen_block` but skips DeclareLocal/DeclareContext instructions
/// (they have already been hoisted before the scope guard).
fn codegen_block_skip_declares(
    block: &ReactiveBlock,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
    declared: &mut HashSet<String>,
) {
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                if matches!(
                    &instruction.value,
                    InstructionValue::DeclareLocal { .. } | InstructionValue::DeclareContext { .. }
                ) {
                    continue;
                }
                let indent_str = "  ".repeat(indent);
                codegen_instruction(instruction, output, &indent_str, declared);
            }
            ReactiveInstruction::Terminal(terminal) => {
                codegen_terminal(terminal, output, cache_slot, indent, declared);
            }
            ReactiveInstruction::Scope(scope_block) => {
                codegen_scope(scope_block, output, cache_slot, indent, declared);
            }
        }
    }
}

fn codegen_instruction(
    instr: &crate::hir::types::Instruction,
    output: &mut String,
    indent: &str,
    declared: &mut HashSet<String>,
) {
    let lvalue_name = place_name(&instr.lvalue);
    // If the lvalue was already declared (by DeclareLocal or scope pre-declaration),
    // use bare assignment; otherwise use `const`.
    let decl_keyword = if declared.contains(lvalue_name.as_ref()) { "" } else { "const " };

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
            output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {val_str};\n"));
        }
        InstructionValue::LoadLocal { place } => {
            let name = place_name(place);
            if name != lvalue_name {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {name};\n"));
            }
        }
        InstructionValue::StoreLocal { lvalue: target, value, type_ } => {
            let target_name = place_name(target);
            let value_name = place_name(value);
            let already_declared = declared.contains(target_name.as_ref());
            let keyword = if already_declared {
                ""
            } else {
                match type_ {
                    Some(crate::hir::types::InstructionKind::Const)
                    | Some(crate::hir::types::InstructionKind::HoistedConst) => "const ",
                    Some(crate::hir::types::InstructionKind::Let) => "let ",
                    Some(crate::hir::types::InstructionKind::Var) => "var ",
                    Some(crate::hir::types::InstructionKind::HoistedFunction) => "const ",
                    Some(crate::hir::types::InstructionKind::Reassign) | None => "",
                }
            };
            output.push_str(&format!("{indent}{keyword}{target_name} = {value_name};\n"));
        }
        InstructionValue::CallExpression { callee, args } => {
            let callee_name = place_name(callee);
            let args_str: Vec<Cow<'_, str>> = args.iter().map(place_name).collect();
            output.push_str(&format!(
                "{}{}{} = {}({});\n",
                indent,
                decl_keyword,
                lvalue_name,
                callee_name,
                args_str.join(", ")
            ));
        }
        InstructionValue::MethodCall { receiver, property, args } => {
            let receiver_name = place_name(receiver);
            let args_str: Vec<Cow<'_, str>> = args.iter().map(place_name).collect();
            output.push_str(&format!(
                "{}{}{} = {}.{}({});\n",
                indent,
                decl_keyword,
                lvalue_name,
                receiver_name,
                property,
                args_str.join(", ")
            ));
        }
        InstructionValue::PropertyLoad { object, property } => {
            output.push_str(&format!(
                "{}{}{} = {}.{};\n",
                indent,
                decl_keyword,
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
                "{}{}{} = {} {} {};\n",
                indent,
                decl_keyword,
                lvalue_name,
                place_name(left),
                op_str,
                place_name(right)
            ));
        }
        InstructionValue::UnaryExpression { op, value } => {
            let op_str = unary_op_str(*op);
            output.push_str(&format!(
                "{}{}{} = {}{};\n",
                indent,
                decl_keyword,
                lvalue_name,
                op_str,
                place_name(value)
            ));
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            let tag_name = place_name(tag);
            // Build props object
            let mut props_parts = Vec::new();
            let mut has_spread = false;
            for attr in props {
                match &attr.name {
                    crate::hir::types::JsxAttributeName::Named(name) => {
                        // Quote attribute names that aren't valid JS identifiers (e.g. aria-label, data-testid)
                        let key = if name.contains('-') || name.contains(':') {
                            format!("\"{}\"", name)
                        } else {
                            name.clone()
                        };
                        props_parts.push(format!("{}: {}", key, place_name(&attr.value)));
                    }
                    crate::hir::types::JsxAttributeName::Spread => {
                        has_spread = true;
                        props_parts.push(format!("...{}", place_name(&attr.value)));
                    }
                }
            }
            // Add children to props
            if children.len() == 1 {
                props_parts.push(format!("children: {}", place_name(&children[0])));
            } else if children.len() > 1 {
                let child_strs: Vec<Cow<'_, str>> = children.iter().map(place_name).collect();
                props_parts.push(format!("children: [{}]", child_strs.join(", ")));
            }
            let props_str = if props_parts.is_empty() && !has_spread {
                "{}".to_string()
            } else {
                format!("{{ {} }}", props_parts.join(", "))
            };
            // Use _jsxs for multiple children, _jsx for 0-1
            let jsx_fn = if children.len() > 1 { "_jsxs" } else { "_jsx" };
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = {jsx_fn}({tag_name}, {props_str});\n"
            ));
        }
        InstructionValue::JsxFragment { children } => {
            if children.is_empty() {
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = _jsx(_Fragment, {{}});\n"
                ));
            } else if children.len() == 1 {
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = _jsx(_Fragment, {{ children: {} }});\n",
                    place_name(&children[0])
                ));
            } else {
                let child_strs: Vec<Cow<'_, str>> = children.iter().map(place_name).collect();
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = _jsxs(_Fragment, {{ children: [{}] }});\n",
                    child_strs.join(", ")
                ));
            }
        }
        InstructionValue::ObjectExpression { properties } => {
            if properties.is_empty() {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {{}};\n"));
            } else {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {{ "));
                for (i, prop) in properties.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    match &prop.key {
                        crate::hir::types::ObjectPropertyKey::Identifier(name) if name == "..." => {
                            // Spread property
                            output.push_str(&format!("...{}", place_name(&prop.value)));
                        }
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
            output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = ["));
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
            output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = `"));
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
                "{}{}{} = new {}({});\n",
                indent,
                decl_keyword,
                lvalue_name,
                callee_name,
                args_str.join(", ")
            ));
        }
        InstructionValue::Await { value } => {
            output.push_str(&format!(
                "{}{}{} = await {};\n",
                indent,
                decl_keyword,
                lvalue_name,
                place_name(value)
            ));
        }
        InstructionValue::Destructure { lvalue_pattern, value } => {
            let value_name = place_name(value);
            // Check if any of the pattern's top-level names are already declared
            let any_declared = pattern_has_declared_names(lvalue_pattern, declared);
            if any_declared {
                // Bare assignment — variables were declared by DeclareLocal
                output.push_str(&format!("{indent}("));
                codegen_destructure_pattern(lvalue_pattern, output);
                output.push_str(&format!(" = {value_name});\n"));
            } else {
                output.push_str(&format!("{indent}const "));
                codegen_destructure_pattern(lvalue_pattern, output);
                output.push_str(&format!(" = {value_name};\n"));
                // Register destructured names so later DeclareLocal/scopes don't re-declare
                collect_pattern_names(lvalue_pattern, declared);
            }
        }
        InstructionValue::DeclareLocal { lvalue, type_ } => {
            let name = place_name(lvalue);
            // Skip if already declared (e.g., by hoisted param destructuring)
            if declared.contains(name.as_ref()) {
                return;
            }
            let keyword = match type_ {
                crate::hir::types::InstructionKind::Const
                | crate::hir::types::InstructionKind::HoistedConst
                | crate::hir::types::InstructionKind::Let
                | crate::hir::types::InstructionKind::HoistedFunction => "let",
                crate::hir::types::InstructionKind::Var => "var",
                crate::hir::types::InstructionKind::Reassign => {
                    return;
                }
            };
            declared.insert(name.to_string());
            output.push_str(&format!("{indent}{keyword} {name};\n"));
        }
        InstructionValue::DeclareContext { lvalue } => {
            let name = place_name(lvalue);
            declared.insert(name.to_string());
            output.push_str(&format!("{indent}let {name};\n"));
        }
        InstructionValue::LoadContext { place } => {
            let name = place_name(place);
            if name != lvalue_name {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {name};\n"));
            }
        }
        InstructionValue::StoreContext { lvalue: target, value } => {
            let target_name = place_name(target);
            let value_name = place_name(value);
            output.push_str(&format!("{indent}{target_name} = {value_name};\n"));
        }
        InstructionValue::LoadGlobal { binding } => {
            if binding.name != *lvalue_name {
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = {};\n",
                    binding.name
                ));
            }
        }
        InstructionValue::StoreGlobal { name, value } => {
            output.push_str(&format!("{indent}{name} = {};\n", place_name(value)));
        }
        InstructionValue::ComputedLoad { object, property } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = {}[{}];\n",
                place_name(object),
                place_name(property)
            ));
        }
        InstructionValue::ComputedStore { object, property, value } => {
            output.push_str(&format!(
                "{indent}{}[{}] = {};\n",
                place_name(object),
                place_name(property),
                place_name(value)
            ));
        }
        InstructionValue::PropertyDelete { object, property } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = delete {}.{};\n",
                place_name(object),
                property
            ));
        }
        InstructionValue::ComputedDelete { object, property } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = delete {}[{}];\n",
                place_name(object),
                place_name(property)
            ));
        }
        InstructionValue::TypeCastExpression { value, .. } => {
            // Type casts are erased at runtime — just pass through the value
            let name = place_name(value);
            if name != lvalue_name {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {name};\n"));
            }
        }
        InstructionValue::JSXText { value } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = \"{}\";\n",
                value
                    .replace('\\', "\\\\")
                    .replace('\"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
            ));
        }
        InstructionValue::RegExpLiteral { pattern, flags } => {
            output
                .push_str(&format!("{indent}{decl_keyword}{lvalue_name} = /{pattern}/{flags};\n"));
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            let tag_name = place_name(tag);
            output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {tag_name}`"));
            for (i, quasi) in value.quasis.iter().enumerate() {
                output.push_str(quasi);
                if i < value.subexpressions.len() {
                    output.push_str(&format!("${{{}}}", place_name(&value.subexpressions[i])));
                }
            }
            output.push_str("`;\n");
        }
        InstructionValue::FunctionExpression { name, lowered_func, expr_type } => {
            let func = lowered_func;
            let is_arrow = matches!(expr_type, crate::hir::types::FunctionExprType::ArrowFunction);
            let async_prefix = if func.is_async { "async " } else { "" };

            // Build params
            let params: Vec<Cow<'_, str>> = func
                .params
                .iter()
                .map(|p| match p {
                    crate::hir::types::Param::Identifier(place) => place_name(place),
                    crate::hir::types::Param::Spread(place) => {
                        Cow::Owned(format!("...{}", place_name(place)))
                    }
                })
                .collect();
            let params_str = params.join(", ");

            if is_arrow {
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = {async_prefix}({params_str}) => {{\n"
                ));
            } else {
                let fn_name = name.as_deref().unwrap_or("");
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = {async_prefix}function {fn_name}({params_str}) {{\n"
                ));
            }

            let indent_level = indent.len() / 2;
            codegen_hir_body(&func.body, output, indent_level + 1);
            output.push_str(&format!("{indent}}};\n"));
        }
        InstructionValue::ObjectMethod { lowered_func } => {
            let func = lowered_func;
            let params: Vec<Cow<'_, str>> = func
                .params
                .iter()
                .map(|p| match p {
                    crate::hir::types::Param::Identifier(place) => place_name(place),
                    crate::hir::types::Param::Spread(place) => {
                        Cow::Owned(format!("...{}", place_name(place)))
                    }
                })
                .collect();
            let async_prefix = if func.is_async { "async " } else { "" };
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = {async_prefix}function({}) {{\n",
                params.join(", ")
            ));
            let indent_level = indent.len() / 2;
            codegen_hir_body(&func.body, output, indent_level + 1);
            output.push_str(&format!("{indent}}};\n"));
        }
        InstructionValue::PrefixUpdate { op, lvalue } => {
            let op_str = match op {
                crate::hir::types::UpdateOp::Increment => "++",
                crate::hir::types::UpdateOp::Decrement => "--",
            };
            let name = place_name(lvalue);
            output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {op_str}{name};\n"));
        }
        InstructionValue::PostfixUpdate { op, lvalue } => {
            let op_str = match op {
                crate::hir::types::UpdateOp::Increment => "++",
                crate::hir::types::UpdateOp::Decrement => "--",
            };
            let name = place_name(lvalue);
            output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {name}{op_str};\n"));
        }
        InstructionValue::GetIterator { collection } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = {}[Symbol.iterator]();\n",
                place_name(collection)
            ));
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = {}.next();\n",
                place_name(iterator)
            ));
        }
        InstructionValue::NextPropertyOf { value } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = {};\n",
                place_name(value)
            ));
        }
        InstructionValue::StartMemoize { .. } => {
            // Manual memoization marker — no runtime code needed
        }
        InstructionValue::FinishMemoize { decl, .. } => {
            let name = place_name(decl);
            if name != lvalue_name {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {name};\n"));
            }
        }
        InstructionValue::UnsupportedNode { node } => {
            output.push_str(&format!("{indent}/* unsupported: {node} */\n"));
        }
    }
}

fn codegen_terminal(
    terminal: &ReactiveTerminal,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
    declared: &mut HashSet<String>,
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
            codegen_block(consequent, output, cache_slot, indent + 1, declared);
            output.push_str(&format!("{indent_str}}} else {{\n"));
            codegen_block(alternate, output, cache_slot, indent + 1, declared);
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
                codegen_block(block, output, cache_slot, indent + 2, declared);
            }
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::While { test, body, .. } => {
            output.push_str(&format!("{indent_str}while (true) {{\n"));
            codegen_block(test, output, cache_slot, indent + 1, declared);
            codegen_block(body, output, cache_slot, indent + 1, declared);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::DoWhile { body, test, .. } => {
            output.push_str(&format!("{indent_str}do {{\n"));
            codegen_block(body, output, cache_slot, indent + 1, declared);
            output.push_str(&format!("{indent_str}}} while (true);\n"));
            // Test block is evaluated inside the loop for condition
            let _ = test;
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            output.push_str(&format!("{indent_str}for (;;) {{\n"));
            codegen_block(init, output, cache_slot, indent + 1, declared);
            codegen_block(test, output, cache_slot, indent + 1, declared);
            codegen_block(body, output, cache_slot, indent + 1, declared);
            if let Some(upd) = update {
                codegen_block(upd, output, cache_slot, indent + 1, declared);
            }
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::ForOf { init, test, body, .. } => {
            output.push_str(&format!("{indent_str}for (const _ of _) {{\n"));
            codegen_block(init, output, cache_slot, indent + 1, declared);
            codegen_block(test, output, cache_slot, indent + 1, declared);
            codegen_block(body, output, cache_slot, indent + 1, declared);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::ForIn { init, test, body, .. } => {
            output.push_str(&format!("{indent_str}for (const _ in _) {{\n"));
            codegen_block(init, output, cache_slot, indent + 1, declared);
            codegen_block(test, output, cache_slot, indent + 1, declared);
            codegen_block(body, output, cache_slot, indent + 1, declared);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            output.push_str(&format!("{indent_str}try {{\n"));
            codegen_block(block, output, cache_slot, indent + 1, declared);
            output.push_str(&format!("{indent_str}}} catch (e) {{\n"));
            codegen_block(handler, output, cache_slot, indent + 1, declared);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Label { block, label, .. } => {
            output.push_str(&format!("{indent_str}bb{label}: {{\n"));
            codegen_block(block, output, cache_slot, indent + 1, declared);
            output.push_str(&format!("{indent_str}}}\n"));
        }
    }
}

fn codegen_scope(
    scope: &ReactiveScopeBlock,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
    declared: &mut HashSet<String>,
) {
    let indent_str = "  ".repeat(indent);
    let deps = &scope.scope.dependencies;
    let slot_start = *cache_slot;

    // Hoist DeclareLocal instructions out of the scope body.
    // These must be emitted before the scope's dependency check since the
    // check may reference these variables (e.g., `$[0] !== count`).
    for instr in &scope.instructions.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr {
            if let InstructionValue::DeclareLocal { .. } | InstructionValue::DeclareContext { .. } =
                &instruction.value
            {
                codegen_instruction(instruction, output, &indent_str, declared);
            }
        }
    }

    // Pre-declare scope output variables with `let` so the else-branch
    // (cache reload) can assign to them. Variables already declared by
    // DeclareLocal above are skipped.
    for (_, decl) in scope.scope.declarations.iter() {
        let decl_name = identifier_display_name(&decl.identifier);
        if !declared.contains(decl_name.as_ref()) {
            declared.insert(decl_name.to_string());
            output.push_str(&format!("{indent_str}let {decl_name};\n"));
        }
    }

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

    // Generate scope body (DeclareLocal/DeclareContext already hoisted above)
    codegen_block_skip_declares(&scope.instructions, output, cache_slot, indent + 1, declared);

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

/// Generate JavaScript from a lowered HIR function body (used for nested
/// function expressions and object methods whose body hasn't been through
/// the reactive transform).
///
/// Converts the HIR to a ReactiveBlock via `build_reactive_block` and then
/// reuses the standard codegen pipeline, ensuring proper CFG traversal
/// (no duplicate blocks, correct ordering, ternary/logical lowering).
fn codegen_hir_body(hir: &crate::hir::types::HIR, output: &mut String, indent: usize) {
    let reactive_block =
        crate::reactive_scopes::build_reactive_function::build_reactive_block_from_hir(
            hir, hir.entry,
        );
    let mut declared = HashSet::new();
    let mut cache_slot = 0u32;
    codegen_block(&reactive_block, output, &mut cache_slot, indent, &mut declared);
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
    "import { c as _c } from \"react/compiler-runtime\";\nimport { jsx as _jsx, jsxs as _jsxs, Fragment as _Fragment } from \"react/jsx-runtime\";\n".to_string()
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

fn pattern_has_declared_names(
    pattern: &crate::hir::types::DestructurePattern,
    declared: &HashSet<String>,
) -> bool {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};
    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                match &prop.value {
                    DestructureTarget::Place(place) => {
                        if declared.contains(place_name(place).as_ref()) {
                            return true;
                        }
                    }
                    DestructureTarget::Pattern(nested) => {
                        if pattern_has_declared_names(nested, declared) {
                            return true;
                        }
                    }
                }
            }
            if let Some(rest_place) = rest {
                if declared.contains(place_name(rest_place).as_ref()) {
                    return true;
                }
            }
            false
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(target) => match target {
                        DestructureTarget::Place(place) => {
                            if declared.contains(place_name(place).as_ref()) {
                                return true;
                            }
                        }
                        DestructureTarget::Pattern(nested) => {
                            if pattern_has_declared_names(nested, declared) {
                                return true;
                            }
                        }
                    },
                    DestructureArrayItem::Hole | DestructureArrayItem::Spread(_) => {}
                }
            }
            if let Some(rest_place) = rest {
                if declared.contains(place_name(rest_place).as_ref()) {
                    return true;
                }
            }
            false
        }
    }
}

/// Collect all variable names from a destructure pattern into the declared set.
fn collect_pattern_names(
    pattern: &crate::hir::types::DestructurePattern,
    declared: &mut HashSet<String>,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};
    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                match &prop.value {
                    DestructureTarget::Place(place) => {
                        declared.insert(place_name(place).to_string());
                    }
                    DestructureTarget::Pattern(nested) => {
                        collect_pattern_names(nested, declared);
                    }
                }
            }
            if let Some(rest_place) = rest {
                declared.insert(place_name(rest_place).to_string());
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(target) => match target {
                        DestructureTarget::Place(place) => {
                            declared.insert(place_name(place).to_string());
                        }
                        DestructureTarget::Pattern(nested) => {
                            collect_pattern_names(nested, declared);
                        }
                    },
                    DestructureArrayItem::Hole | DestructureArrayItem::Spread(_) => {}
                }
            }
            if let Some(rest_place) = rest {
                declared.insert(place_name(rest_place).to_string());
            }
        }
    }
}

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
    let mut declared = HashSet::new();
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                let indent_str = "  ".repeat(indent);
                // Record mapping for this instruction.
                ctx.map_from_span(instruction.loc, source);
                codegen_instruction(instruction, &mut ctx.output, &indent_str, &mut declared);
                // Update line/col tracking after codegen_instruction wrote to output.
                recompute_position(ctx);
            }
            ReactiveInstruction::Terminal(terminal) => {
                codegen_terminal(terminal, &mut ctx.output, cache_slot, indent, &mut declared);
                recompute_position(ctx);
            }
            ReactiveInstruction::Scope(scope_block) => {
                codegen_scope(scope_block, &mut ctx.output, cache_slot, indent, &mut declared);
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
