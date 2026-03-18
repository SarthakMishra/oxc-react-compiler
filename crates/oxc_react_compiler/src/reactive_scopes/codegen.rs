#![allow(dead_code)]

use std::borrow::Cow;

use crate::hir::types::IdentifierId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::hir::types::{
    InstructionValue, Place, Primitive, ReactiveBlock, ReactiveFunction, ReactiveInstruction,
    ReactiveScopeBlock, ReactiveTerminal,
};

// ---------------------------------------------------------------------------
// Expression inlining
// ---------------------------------------------------------------------------

/// A map from temp variable name (e.g. "t11") to the inlined expression string
/// (e.g. `"\"div\""`).  When a temp is in this map, it should **not** be
/// emitted as a separate `const tN = …` statement; instead, its expression
/// is substituted directly at the use-site.
type InlineMap = FxHashMap<String, String>;

/// Returns `true` when the identifier corresponds to a compiler-generated
/// temporary (unnamed, printed as `tN`).
fn is_temp_place(place: &Place) -> bool {
    match &place.identifier.name {
        None => true,
        // After promote_used_temporaries, unnamed temps get synthetic names like "t{id}".
        // Detect these so we can still inline them.
        Some(name) => {
            let expected = format!("t{}", place.identifier.id.0);
            name == &expected
        }
    }
}

/// Count how many times each temp identifier is *used* (appears on the RHS)
/// within a flat slice of reactive instructions, only counting plain
/// `Instruction` and `Terminal` place references at this level (not nested
/// terminals / scopes).
///
/// The returned map only contains entries for temp places (`name.is_none()`).
fn count_temp_uses_flat(instructions: &[ReactiveInstruction]) -> FxHashMap<String, u32> {
    let mut counts: FxHashMap<String, u32> = FxHashMap::default();
    for ri in instructions {
        match ri {
            ReactiveInstruction::Instruction(instr) => {
                visit_instr_uses(&instr.value, &mut counts);
            }
            ReactiveInstruction::Terminal(terminal) => {
                visit_terminal_uses(terminal, &mut counts);
            }
            ReactiveInstruction::Scope(_) => {}
        }
    }
    counts
}

/// Count how many times each temp identifier is *used* across the entire
/// subtree rooted at the given instruction slice — recursing into nested
/// `Scope` bodies and `Terminal` child blocks.
fn count_temp_uses_recursive(instructions: &[ReactiveInstruction]) -> FxHashMap<String, u32> {
    let mut counts: FxHashMap<String, u32> = FxHashMap::default();
    count_temp_uses_in_slice(instructions, &mut counts);
    counts
}

fn count_temp_uses_in_slice(
    instructions: &[ReactiveInstruction],
    counts: &mut FxHashMap<String, u32>,
) {
    for ri in instructions {
        match ri {
            ReactiveInstruction::Instruction(instr) => {
                visit_instr_uses(&instr.value, counts);
            }
            ReactiveInstruction::Terminal(terminal) => {
                visit_terminal_uses(terminal, counts);
                visit_terminal_child_blocks(terminal, counts);
            }
            ReactiveInstruction::Scope(scope_block) => {
                count_temp_uses_in_slice(&scope_block.instructions.instructions, counts);
            }
        }
    }
}

/// Recurse into the child `ReactiveBlock` fields of a terminal to count temp
/// uses in nested blocks (e.g. if/else branches, loop bodies, etc.).
fn visit_terminal_child_blocks(terminal: &ReactiveTerminal, counts: &mut FxHashMap<String, u32>) {
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            count_temp_uses_in_slice(&consequent.instructions, counts);
            count_temp_uses_in_slice(&alternate.instructions, counts);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                count_temp_uses_in_slice(&block.instructions, counts);
            }
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            count_temp_uses_in_slice(&test.instructions, counts);
            count_temp_uses_in_slice(&body.instructions, counts);
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            count_temp_uses_in_slice(&init.instructions, counts);
            count_temp_uses_in_slice(&test.instructions, counts);
            if let Some(upd) = update {
                count_temp_uses_in_slice(&upd.instructions, counts);
            }
            count_temp_uses_in_slice(&body.instructions, counts);
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            count_temp_uses_in_slice(&init.instructions, counts);
            count_temp_uses_in_slice(&test.instructions, counts);
            count_temp_uses_in_slice(&body.instructions, counts);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            count_temp_uses_in_slice(&block.instructions, counts);
            count_temp_uses_in_slice(&handler.instructions, counts);
        }
        ReactiveTerminal::Label { block, .. } => {
            count_temp_uses_in_slice(&block.instructions, counts);
        }
        ReactiveTerminal::Return { .. } | ReactiveTerminal::Throw { .. } => {}
    }
}

/// Count temp uses in terminal place references (test conditions, return values, etc.)
fn visit_terminal_uses(terminal: &ReactiveTerminal, counts: &mut FxHashMap<String, u32>) {
    match terminal {
        ReactiveTerminal::Return { value, .. } | ReactiveTerminal::Throw { value, .. } => {
            bump_temp(value, counts);
        }
        ReactiveTerminal::If { test, .. } => {
            bump_temp(test, counts);
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            bump_temp(test, counts);
            for (test_val, _) in cases {
                if let Some(tv) = test_val {
                    bump_temp(tv, counts);
                }
            }
        }
        // These terminals have ReactiveBlock test fields, not Place fields
        ReactiveTerminal::While { .. }
        | ReactiveTerminal::DoWhile { .. }
        | ReactiveTerminal::For { .. }
        | ReactiveTerminal::ForOf { .. }
        | ReactiveTerminal::ForIn { .. }
        | ReactiveTerminal::Try { .. }
        | ReactiveTerminal::Label { .. } => {}
    }
}

/// Walk all *operand* places in an `InstructionValue` and increment the
/// use-count for any that are temps.
fn visit_instr_uses(value: &InstructionValue, counts: &mut FxHashMap<String, u32>) {
    match value {
        InstructionValue::LoadLocal { place }
        | InstructionValue::LoadContext { place }
        | InstructionValue::TypeCastExpression { value: place, .. }
        | InstructionValue::Await { value: place }
        | InstructionValue::GetIterator { collection: place }
        | InstructionValue::IteratorNext { iterator: place, .. }
        | InstructionValue::NextPropertyOf { value: place }
        | InstructionValue::UnaryExpression { value: place, .. }
        | InstructionValue::FinishMemoize { decl: place, .. } => {
            bump_temp(place, counts);
        }
        InstructionValue::JsxFragment { children } => {
            for c in children {
                bump_temp(c, counts);
            }
        }
        InstructionValue::StoreLocal { lvalue, value, .. } => {
            bump_temp(lvalue, counts);
            bump_temp(value, counts);
        }
        InstructionValue::StoreContext { lvalue, value } => {
            bump_temp(lvalue, counts);
            bump_temp(value, counts);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            bump_temp(left, counts);
            bump_temp(right, counts);
        }
        InstructionValue::CallExpression { callee, args } => {
            bump_temp(callee, counts);
            for a in args {
                bump_temp(a, counts);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            bump_temp(receiver, counts);
            for a in args {
                bump_temp(a, counts);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            bump_temp(callee, counts);
            for a in args {
                bump_temp(a, counts);
            }
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            bump_temp(object, counts);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            bump_temp(object, counts);
            bump_temp(value, counts);
        }
        InstructionValue::ComputedLoad { object, property } => {
            bump_temp(object, counts);
            bump_temp(property, counts);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            bump_temp(object, counts);
            bump_temp(property, counts);
            bump_temp(value, counts);
        }
        InstructionValue::ComputedDelete { object, property } => {
            bump_temp(object, counts);
            bump_temp(property, counts);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                if let crate::hir::types::ObjectPropertyKey::Computed(k) = &prop.key {
                    bump_temp(k, counts);
                }
                bump_temp(&prop.value, counts);
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Expression(p)
                    | crate::hir::types::ArrayElement::Spread(p) => bump_temp(p, counts),
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for p in subexpressions {
                bump_temp(p, counts);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            bump_temp(tag, counts);
            for p in &value.subexpressions {
                bump_temp(p, counts);
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            bump_temp(tag, counts);
            for attr in props {
                bump_temp(&attr.value, counts);
            }
            for c in children {
                bump_temp(c, counts);
            }
        }
        InstructionValue::Destructure { value, .. } => {
            bump_temp(value, counts);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            bump_temp(lvalue, counts);
        }
        InstructionValue::StoreGlobal { value, .. } => {
            bump_temp(value, counts);
        }
        InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. }
        | InstructionValue::DeclareLocal { .. }
        | InstructionValue::DeclareContext { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

fn bump_temp(place: &Place, counts: &mut FxHashMap<String, u32>) {
    if is_temp_place(place) {
        let name = format!("t{}", place.identifier.id.0);
        *counts.entry(name).or_insert(0) += 1;
    }
}

/// Returns `true` if an `InstructionValue` is a "pure" expression that can be
/// safely inlined without changing observable behaviour.
///
/// Pure means: no side-effects, deterministic given the same operands, and
/// safe to evaluate at a different point in program order.
///
/// Impure / not inlinable:
/// - `CallExpression` / `MethodCall` / `NewExpression` — may have side effects
/// - `PropertyStore` / `ComputedStore` / `StoreGlobal` / `StoreLocal` /
///   `StoreContext` — mutations
/// - `DeclareLocal` / `DeclareContext` — declarations (no value)
/// - `StartMemoize` / `FinishMemoize` — compiler markers
/// - `FunctionExpression` / `ObjectMethod` — complex, create closures
/// - `GetIterator` / `IteratorNext` / `NextPropertyOf` — iteration protocol
/// - `PrefixUpdate` / `PostfixUpdate` — mutate in place
/// - `Await` — async; must stay in order
/// - `UnsupportedNode` — unknown
///
/// Pure (safe to inline):
/// `Primitive`, `LoadLocal`, `LoadContext`, `LoadGlobal`, `PropertyLoad`,
/// `ComputedLoad`, `BinaryExpression`, `UnaryExpression`, `ObjectExpression`,
/// `ArrayExpression`, `TemplateLiteral`, `JSXText`, `RegExpLiteral`,
/// `TypeCastExpression`, `JsxExpression`, `JsxFragment`,
/// `TaggedTemplateExpression`
fn is_inlinable(value: &InstructionValue) -> bool {
    matches!(
        value,
        InstructionValue::Primitive { .. }
            | InstructionValue::LoadLocal { .. }
            | InstructionValue::LoadContext { .. }
            | InstructionValue::LoadGlobal { .. }
            | InstructionValue::PropertyLoad { .. }
            | InstructionValue::ComputedLoad { .. }
            | InstructionValue::BinaryExpression { .. }
            | InstructionValue::UnaryExpression { .. }
            | InstructionValue::ObjectExpression { .. }
            | InstructionValue::ArrayExpression { .. }
            | InstructionValue::TemplateLiteral { .. }
            | InstructionValue::JSXText { .. }
            | InstructionValue::RegExpLiteral { .. }
            | InstructionValue::TypeCastExpression { .. }
            | InstructionValue::JsxExpression { .. }
            | InstructionValue::JsxFragment { .. }
            | InstructionValue::TaggedTemplateExpression { .. }
            | InstructionValue::CallExpression { .. }
            | InstructionValue::MethodCall { .. }
            | InstructionValue::NewExpression { .. }
    )
}

/// Build the inline map for a flat block of instructions.
///
/// Algorithm:
/// 1. Count how many times each temp is used — both at this flat level
///    (`flat_counts`) and across the entire subtree (`total_counts`).
/// 2. Walk instructions in order.  For each instruction whose lvalue is an
///    unnamed temp:
///    a. Its total use-count must be exactly 1.
///    b. The value must be `is_inlinable`.
///    c. For call-like instructions (CallExpression / MethodCall /
///    NewExpression) we additionally require that the single use is at
///    the *same* nesting level (flat_count == 1) — we must not inline
///    a side-effecting call into a reactive scope body that may be
///    skipped by the cache guard.
///    If all conditions hold, generate the expression string and insert into
///    the map.
///
/// The expression string is generated exactly as `codegen_instruction` would
/// emit the RHS, but without the `const tN = ` prefix, and using the inline
/// map built so far (so chained inlining works: `_jsx(t11, …)` where t11 was
/// already inlined to `"div"` becomes `_jsx("div", …)`).
fn build_inline_map(
    instructions: &[ReactiveInstruction],
    protected_names: &FxHashSet<String>,
    tag_constants: &TagConstantMap,
) -> InlineMap {
    // Flat counts: only uses at this block level (no recursion into scopes/terminals)
    let flat_counts = count_temp_uses_flat(instructions);
    // Total counts: uses across the entire subtree (recursive into scopes/terminals)
    let total_counts = count_temp_uses_recursive(instructions);
    let mut inline_map: InlineMap = FxHashMap::default();

    // Collect temps that are used in scope declarations (outputs from child scopes) —
    // these must never be removed as dead even if they have 0 intra-block uses.
    let mut scope_output_temps: FxHashSet<String> = FxHashSet::default();
    for ri in instructions {
        if let ReactiveInstruction::Scope(scope_block) = ri {
            for (_, decl) in &scope_block.scope.declarations {
                let decl_name = identifier_display_name(&decl.identifier);
                scope_output_temps.insert(decl_name.to_string());
            }
        }
    }
    for ri in instructions {
        let ReactiveInstruction::Instruction(instr) = ri else { continue };
        let lvalue = &instr.lvalue;
        // Only unnamed temps
        if !is_temp_place(lvalue) {
            continue;
        }
        let temp_name = format!("t{}", lvalue.identifier.id.0);
        // Check total use count (across all nested blocks)
        let total_count = total_counts.get(&temp_name).copied().unwrap_or(0);
        if total_count == 0 {
            // Dead temp — mark for removal by inserting empty sentinel
            // But NOT if it's a scope output (used in cache storage/else branch)
            // And NOT if it has side effects
            if !scope_output_temps.contains(&temp_name)
                && !protected_names.contains(&temp_name)
                && !matches!(
                    &instr.value,
                    InstructionValue::CallExpression { .. }
                        | InstructionValue::MethodCall { .. }
                        | InstructionValue::NewExpression { .. }
                        | InstructionValue::PropertyStore { .. }
                        | InstructionValue::ComputedStore { .. }
                        | InstructionValue::StoreLocal { .. }
                        | InstructionValue::StoreContext { .. }
                        | InstructionValue::StoreGlobal { .. }
                        | InstructionValue::Destructure { .. }
                )
            {
                inline_map.insert(temp_name, String::new()); // sentinel: skip emission
            }
            continue;
        }
        if total_count != 1 {
            continue;
        }
        // Must be an inlinable value
        if !is_inlinable(&instr.value) {
            continue;
        }
        // For call-like values (side-effecting), only inline if the single use
        // is at the same nesting level. If flat_count == 0, the use is inside
        // a nested scope/terminal — the scope body may be skipped by the cache
        // guard, which would suppress the call entirely.
        let is_call_like = matches!(
            &instr.value,
            InstructionValue::CallExpression { .. }
                | InstructionValue::MethodCall { .. }
                | InstructionValue::NewExpression { .. }
        );
        if is_call_like {
            let flat_count = flat_counts.get(&temp_name).copied().unwrap_or(0);
            if flat_count == 0 {
                continue; // use is inside a nested scope — unsafe to inline call
            }
        }
        // Generate the RHS expression string
        if let Some(expr) = expr_string(&instr.value, &inline_map, tag_constants) {
            inline_map.insert(temp_name, expr);
        }
    }

    inline_map
}

/// Generate the RHS expression string for an inlinable instruction value,
/// resolving any operand temps via `inline_map`.
///
/// Returns `None` if expression generation is not supported for this variant
/// (should not happen for variants accepted by `is_inlinable`, but acts as a
/// safety valve).
fn expr_string(
    value: &InstructionValue,
    im: &InlineMap,
    tag_constants: &TagConstantMap,
) -> Option<String> {
    let resolve = |p: &Place| -> String {
        if is_temp_place(p) {
            let name = format!("t{}", p.identifier.id.0);
            if let Some(expr) = im.get(&name) {
                return expr.clone();
            }
        }
        match &p.identifier.name {
            Some(n) => n.clone(),
            None => format!("t{}", p.identifier.id.0),
        }
    };

    match value {
        InstructionValue::Primitive { value } => {
            let s = match value {
                Primitive::Null => "null".to_string(),
                Primitive::Undefined => "undefined".to_string(),
                Primitive::Boolean(b) => b.to_string(),
                Primitive::Number(n) => n.to_string(),
                Primitive::String(s) => format!("\"{}\"", s.replace('\"', "\\\"")),
                Primitive::BigInt(n) => format!("{n}n"),
            };
            Some(s)
        }
        InstructionValue::LoadLocal { place }
        | InstructionValue::LoadContext { place }
        | InstructionValue::TypeCastExpression { value: place, .. } => Some(resolve(place)),
        InstructionValue::LoadGlobal { binding } => Some(binding.name.clone()),
        InstructionValue::PropertyLoad { object, property } => {
            Some(format!("{}.{}", resolve(object), property))
        }
        InstructionValue::ComputedLoad { object, property } => {
            Some(format!("{}[{}]", resolve(object), resolve(property)))
        }
        InstructionValue::BinaryExpression { op, left, right } => {
            Some(format!("{} {} {}", resolve(left), binary_op_str(*op), resolve(right)))
        }
        InstructionValue::UnaryExpression { op, value } => {
            Some(format!("{}{}", unary_op_str(*op), resolve(value)))
        }
        InstructionValue::JSXText { value } => Some(format!(
            "\"{}\"",
            value
                .replace('\\', "\\\\")
                .replace('\"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
        )),
        InstructionValue::RegExpLiteral { pattern, flags } => Some(format!("/{pattern}/{flags}")),
        InstructionValue::TemplateLiteral { quasis, subexpressions } => {
            let mut s = "`".to_string();
            for (i, quasi) in quasis.iter().enumerate() {
                s.push_str(quasi);
                if i < subexpressions.len() {
                    s.push_str(&format!("${{{}}}", resolve(&subexpressions[i])));
                }
            }
            s.push('`');
            Some(s)
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            let mut s = format!("{}`", resolve(tag));
            for (i, quasi) in value.quasis.iter().enumerate() {
                s.push_str(quasi);
                if i < value.subexpressions.len() {
                    s.push_str(&format!("${{{}}}", resolve(&value.subexpressions[i])));
                }
            }
            s.push('`');
            Some(s)
        }
        InstructionValue::ObjectExpression { properties } => {
            if properties.is_empty() {
                return Some("{}".to_string());
            }
            let mut parts = Vec::new();
            for prop in properties {
                match &prop.key {
                    crate::hir::types::ObjectPropertyKey::Identifier(name) if name == "..." => {
                        parts.push(format!("...{}", resolve(&prop.value)));
                    }
                    crate::hir::types::ObjectPropertyKey::Identifier(name) => {
                        if prop.shorthand {
                            parts.push(name.clone());
                        } else {
                            parts.push(format!("{}: {}", name, resolve(&prop.value)));
                        }
                    }
                    crate::hir::types::ObjectPropertyKey::Computed(k) => {
                        parts.push(format!("[{}]: {}", resolve(k), resolve(&prop.value)));
                    }
                }
            }
            Some(format!("{{ {} }}", parts.join(", ")))
        }
        InstructionValue::ArrayExpression { elements } => {
            let mut parts = Vec::new();
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Expression(p) => {
                        parts.push(resolve(p));
                    }
                    crate::hir::types::ArrayElement::Spread(p) => {
                        parts.push(format!("...{}", resolve(p)));
                    }
                    crate::hir::types::ArrayElement::Hole => parts.push(String::new()),
                }
            }
            Some(format!("[{}]", parts.join(", ")))
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            let raw_tag = resolve(tag);
            // Check tag_constants for cross-scope JSX tag resolution
            let raw_tag = if is_temp_place(tag) && !is_jsx_text_str(&raw_tag) {
                let temp_name = format!("t{}", tag.identifier.id.0);
                tag_constants.get(&temp_name).cloned().unwrap_or(raw_tag)
            } else {
                raw_tag
            };
            let tag_name = jsx_tag_name(&raw_tag);
            // Build props string in JSX syntax.
            // NOTE: This duplicates `build_jsx_props_str` because `expr_string`
            // uses a local closure `resolve` rather than an `InlineMap` reference.
            // Keep both in sync when modifying JSX attribute emission.
            let mut props_str = String::new();
            for attr in props {
                match &attr.name {
                    crate::hir::types::JsxAttributeName::Named(name) => {
                        let val = resolve(&attr.value);
                        let attr_val = jsx_attr_value_str(&val);
                        props_str.push_str(&format!(" {name}{attr_val}"));
                    }
                    crate::hir::types::JsxAttributeName::Spread => {
                        props_str.push_str(&format!(" {{...{}}}", resolve(&attr.value)));
                    }
                }
            }
            // Emit JSX syntax
            if children.is_empty() {
                Some(format!("<{tag_name}{props_str} />"))
            } else {
                let mut children_str = String::new();
                for child in children {
                    let resolved = resolve(child);
                    children_str.push_str(&jsx_child_str(&resolved));
                }
                Some(format!("<{tag_name}{props_str}>{children_str}</{tag_name}>"))
            }
        }
        InstructionValue::JsxFragment { children } => {
            if children.is_empty() {
                Some("<></>".to_string())
            } else {
                let mut children_str = String::new();
                for child in children {
                    let resolved = resolve(child);
                    children_str.push_str(&jsx_child_str(&resolved));
                }
                Some(format!("<>{children_str}</>"))
            }
        }
        InstructionValue::CallExpression { callee, args } => {
            let callee_name = resolve(callee);
            let args_str: Vec<String> = args.iter().map(&resolve).collect();
            Some(format!("{}({})", callee_name, args_str.join(", ")))
        }
        InstructionValue::MethodCall { receiver, property, args } => {
            let receiver_name = resolve(receiver);
            let args_str: Vec<String> = args.iter().map(&resolve).collect();
            Some(format!("{}.{}({})", receiver_name, property, args_str.join(", ")))
        }
        InstructionValue::NewExpression { callee, args } => {
            let callee_name = resolve(callee);
            let args_str: Vec<String> = args.iter().map(resolve).collect();
            Some(format!("new {}({})", callee_name, args_str.join(", ")))
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Global JSX tag constant resolution
// ---------------------------------------------------------------------------

/// A map from temp variable name (e.g. "t15") to the constant expression string
/// (e.g. `"\"button\""`). Built once per function by scanning all instructions
/// (recursively into scopes/terminals) for temps assigned `Primitive::String`
/// or `LoadGlobal` values. Used by JSX codegen to resolve tag temps that are
/// scope outputs and thus not available in the block-local `InlineMap`.
type TagConstantMap = FxHashMap<String, String>;

/// Build a map of temp names to their constant expression strings for JSX tag
/// resolution. This walks the entire reactive tree to find temps assigned
/// constant values (Primitive::String, LoadGlobal) that may be used as JSX tags
/// across scope boundaries.
fn build_tag_constant_map(block: &ReactiveBlock) -> TagConstantMap {
    let mut map = TagConstantMap::default();
    collect_tag_constants_from_block(block, &mut map);
    map
}

fn collect_tag_constants_from_block(block: &ReactiveBlock, map: &mut TagConstantMap) {
    for ri in &block.instructions {
        match ri {
            ReactiveInstruction::Instruction(instr) => {
                if !is_temp_place(&instr.lvalue) {
                    continue;
                }
                let temp_name = format!("t{}", instr.lvalue.identifier.id.0);
                match &instr.value {
                    InstructionValue::Primitive { value } => {
                        let s = match value {
                            Primitive::String(s) => {
                                format!("\"{}\"", s.replace('\"', "\\\""))
                            }
                            _ => continue,
                        };
                        map.insert(temp_name, s);
                    }
                    InstructionValue::LoadGlobal { binding } => {
                        map.insert(temp_name, binding.name.clone());
                    }
                    _ => {}
                }
            }
            ReactiveInstruction::Scope(scope) => {
                collect_tag_constants_from_block(&scope.instructions, map);
            }
            ReactiveInstruction::Terminal(terminal) => {
                collect_tag_constants_from_terminal(terminal, map);
            }
        }
    }
}

fn collect_tag_constants_from_terminal(terminal: &ReactiveTerminal, map: &mut TagConstantMap) {
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            collect_tag_constants_from_block(consequent, map);
            collect_tag_constants_from_block(alternate, map);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                collect_tag_constants_from_block(block, map);
            }
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { body, test, .. } => {
            collect_tag_constants_from_block(test, map);
            collect_tag_constants_from_block(body, map);
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            collect_tag_constants_from_block(init, map);
            collect_tag_constants_from_block(test, map);
            collect_tag_constants_from_block(body, map);
            if let Some(upd) = update {
                collect_tag_constants_from_block(upd, map);
            }
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            collect_tag_constants_from_block(init, map);
            collect_tag_constants_from_block(test, map);
            collect_tag_constants_from_block(body, map);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            collect_tag_constants_from_block(block, map);
            collect_tag_constants_from_block(handler, map);
        }
        ReactiveTerminal::Label { block, .. } => {
            collect_tag_constants_from_block(block, map);
        }
        ReactiveTerminal::Return { .. } | ReactiveTerminal::Throw { .. } => {}
    }
}

// ---------------------------------------------------------------------------
// JSX syntax helpers
// ---------------------------------------------------------------------------

/// Returns true if the given resolved expression string is a string literal
/// (double-quoted), indicating it came from a JSXText node or string constant.
fn is_jsx_text_str(s: &str) -> bool {
    s.len() >= 2 && s.starts_with('"') && s.ends_with('"')
}

/// Strip surrounding double quotes from a string literal representation.
/// Input: `"hello world"`, Output: `hello world`
fn strip_string_quotes(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') { &s[1..s.len() - 1] } else { s }
}

/// Format a JSX attribute value. String literals use `="value"` syntax,
/// everything else uses `={expr}` syntax.
fn jsx_attr_value_str(resolved: &str) -> String {
    if is_jsx_text_str(resolved) {
        // String literal attribute: attr="value"
        format!("={resolved}")
    } else if resolved == "true" {
        // Boolean true shorthand: just the attribute name, no value
        String::new()
    } else {
        // Expression attribute: attr={expr}
        format!("={{{resolved}}}")
    }
}

/// Convert a resolved tag name to JSX tag syntax.
/// String literals like `"div"` → `div` (strip quotes for intrinsic elements).
/// Non-string expressions like `Component` or `_temp` → pass through unchanged.
fn jsx_tag_name(resolved: &str) -> &str {
    if is_jsx_text_str(resolved) { strip_string_quotes(resolved) } else { resolved }
}

/// Format a JSX child for embedding in a JSX element body.
/// - String literals → raw text (strip quotes)
/// - Nested JSX elements (starts with `<`) → embed directly (no `{}` wrapper)
/// - Everything else → wrap in `{expr}`
fn jsx_child_str(resolved: &str) -> String {
    if is_jsx_text_str(resolved) {
        strip_string_quotes(resolved).to_string()
    } else if resolved.starts_with('<') {
        // Nested JSX element — embed directly without {} wrapper
        resolved.to_string()
    } else {
        format!("{{{resolved}}}")
    }
}

/// Build JSX props string for an opening tag: ` prop1={val1} prop2="str"`.
fn build_jsx_props_str(
    props: &[crate::hir::types::JsxAttribute],
    inline_map: &InlineMap,
) -> String {
    let mut s = String::new();
    for attr in props {
        match &attr.name {
            crate::hir::types::JsxAttributeName::Named(name) => {
                let val = resolve_place(&attr.value, inline_map);
                let attr_val = jsx_attr_value_str(&val);
                s.push_str(&format!(" {name}{attr_val}"));
            }
            crate::hir::types::JsxAttributeName::Spread => {
                let val = resolve_place(&attr.value, inline_map);
                s.push_str(&format!(" {{...{val}}}"));
            }
        }
    }
    s
}

/// Resolve a place's name, substituting the inline expression if available.
fn resolve_place<'a>(place: &'a Place, inline_map: &'a InlineMap) -> Cow<'a, str> {
    if is_temp_place(place) {
        let name = format!("t{}", place.identifier.id.0);
        if let Some(expr) = inline_map.get(&name) {
            return Cow::Owned(expr.clone());
        }
    }
    place_name(place)
}

/// Generate JavaScript code from a ReactiveFunction.
///
/// This is the final pass that produces the compiled output.
/// It generates code with `useMemoCache` calls and conditional blocks.
pub fn codegen_function(rf: &ReactiveFunction) -> String {
    let mut output = String::new();
    let mut cache_slot = 0u32;

    // Generate function header — preserve arrow vs function syntax from source
    if rf.is_arrow {
        output.push('(');
    } else if let Some(ref name) = rf.id {
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
    if rf.is_arrow {
        output.push_str(") => {\n");
    } else {
        output.push_str(") {\n");
    }

    // Emit function body directives (e.g., "use memo", "use forget", "use foo")
    // These must appear before any other statements.
    for directive in &rf.directives {
        output.push_str(&format!("  \"{directive}\";\n"));
    }

    // Count total cache slots needed
    let total_slots = count_cache_slots(&rf.body);
    if total_slots > 0 {
        output.push_str(&format!("  const $ = _c({total_slots});\n"));
    }

    // Track variables declared via DeclareLocal so that subsequent
    // StoreLocal / Destructure can emit bare assignments instead of
    // re-declaring with const/let.
    let mut declared = FxHashSet::default();

    // Collect parameter names for destructuring hoisting
    let param_names: FxHashSet<String> = rf
        .params
        .iter()
        .map(|p| match p {
            crate::hir::types::Param::Identifier(place) => place_name(place).to_string(),
            crate::hir::types::Param::Spread(place) => place_name(place).to_string(),
        })
        .collect();

    // Pre-declare ALL scope declaration variables at function level.
    // This ensures that any StoreLocal targeting a scope output uses bare
    // assignment instead of `const`/`let`, preventing "Assignment to constant
    // variable" errors and duplicate declarations. This matches the upstream
    // compiler's approach where scope outputs are always `let`-declared at
    // function scope before the scope guard.
    collect_all_scope_declarations(&rf.body, &mut output, &mut declared, 1);

    // Build a global map of temp → constant expression for JSX tag resolution.
    // This allows JSX tags assigned in one scope to be resolved in another.
    let tag_constants = build_tag_constant_map(&rf.body);

    // Hoist Destructure instructions that destructure from function parameters
    // (e.g., `const { status } = t0;`) to the top of the function body,
    // before any reactive scope checks that may reference those variables.
    // Hoisted parameter destructures are never temps, so we use an empty inline map.
    let empty_inline_map = InlineMap::default();
    let mut hoisted_indices = FxHashSet::default();
    for (i, instr) in rf.body.instructions.iter().enumerate() {
        if let ReactiveInstruction::Instruction(instruction) = instr
            && let InstructionValue::Destructure { value, .. } = &instruction.value
        {
            let value_name = place_name(value);
            if param_names.contains(value_name.as_ref()) {
                let indent_str = "  ";
                codegen_instruction(
                    instruction,
                    &mut output,
                    indent_str,
                    &mut declared,
                    &empty_inline_map,
                    &tag_constants,
                );
                hoisted_indices.insert(i);
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
        &tag_constants,
    );

    output.push_str("}\n");
    output
}

fn codegen_block(
    block: &ReactiveBlock,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
    declared: &mut FxHashSet<String>,
    tag_constants: &TagConstantMap,
) {
    let inline_map = build_inline_map(&block.instructions, &FxHashSet::default(), tag_constants);
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                let indent_str = "  ".repeat(indent);
                codegen_instruction(
                    instruction,
                    output,
                    &indent_str,
                    declared,
                    &inline_map,
                    tag_constants,
                );
            }
            ReactiveInstruction::Terminal(terminal) => {
                codegen_terminal(
                    terminal,
                    output,
                    cache_slot,
                    indent,
                    declared,
                    &inline_map,
                    tag_constants,
                );
            }
            ReactiveInstruction::Scope(scope_block) => {
                codegen_scope(scope_block, output, cache_slot, indent, declared, tag_constants);
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
    declared: &mut FxHashSet<String>,
    hoisted_indices: &FxHashSet<usize>,
    tag_constants: &TagConstantMap,
) {
    let inline_map = build_inline_map(&block.instructions, &FxHashSet::default(), tag_constants);
    for (i, instr) in block.instructions.iter().enumerate() {
        if hoisted_indices.contains(&i) {
            continue;
        }
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                let indent_str = "  ".repeat(indent);
                codegen_instruction(
                    instruction,
                    output,
                    &indent_str,
                    declared,
                    &inline_map,
                    tag_constants,
                );
            }
            ReactiveInstruction::Terminal(terminal) => {
                codegen_terminal(
                    terminal,
                    output,
                    cache_slot,
                    indent,
                    declared,
                    &inline_map,
                    tag_constants,
                );
            }
            ReactiveInstruction::Scope(scope_block) => {
                codegen_scope(scope_block, output, cache_slot, indent, declared, tag_constants);
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
    declared: &mut FxHashSet<String>,
    protected_names: &FxHashSet<String>,
    tag_constants: &TagConstantMap,
) {
    let inline_map = build_inline_map(&block.instructions, protected_names, tag_constants);
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
                codegen_instruction(
                    instruction,
                    output,
                    &indent_str,
                    declared,
                    &inline_map,
                    tag_constants,
                );
            }
            ReactiveInstruction::Terminal(terminal) => {
                codegen_terminal(
                    terminal,
                    output,
                    cache_slot,
                    indent,
                    declared,
                    &inline_map,
                    tag_constants,
                );
            }
            ReactiveInstruction::Scope(scope_block) => {
                codegen_scope(scope_block, output, cache_slot, indent, declared, tag_constants);
            }
        }
    }
}

fn codegen_instruction(
    instr: &crate::hir::types::Instruction,
    output: &mut String,
    indent: &str,
    declared: &mut FxHashSet<String>,
    inline_map: &InlineMap,
    tag_constants: &TagConstantMap,
) {
    let lvalue_name = place_name(&instr.lvalue);

    // If this instruction's lvalue has been selected for inlining, skip emitting it —
    // the expression will be substituted at its single use site.
    if is_temp_place(&instr.lvalue) {
        let temp_name = format!("t{}", instr.lvalue.identifier.id.0);
        if inline_map.contains_key(&temp_name) {
            return;
        }
    }

    // If the lvalue was already declared (by DeclareLocal or scope pre-declaration),
    // use bare assignment; otherwise use `let` (not `const`) so the compiler's
    // scope reload logic can reassign variables without "Assignment to constant variable" errors.
    let decl_keyword = if declared.contains(lvalue_name.as_ref()) { "" } else { "let " };

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
            let name = resolve_place(place, inline_map);
            if name != lvalue_name {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {name};\n"));
            }
        }
        InstructionValue::StoreLocal { lvalue: target, value, type_ } => {
            let target_name = place_name(target);
            let value_name = resolve_place(value, inline_map);
            let already_declared = declared.contains(target_name.as_ref());
            let keyword = if already_declared {
                ""
            } else {
                // Use `let` for all declaration kinds (including original `const`)
                // because the compiler's scope logic may reassign these variables
                // in scope reload branches. `var` is preserved for hoisting semantics.
                let kw = match type_ {
                    Some(
                        crate::hir::types::InstructionKind::Const
                        | crate::hir::types::InstructionKind::HoistedConst
                        | crate::hir::types::InstructionKind::Let
                        | crate::hir::types::InstructionKind::HoistedFunction,
                    ) => "let ",
                    Some(crate::hir::types::InstructionKind::Var) => "var ",
                    Some(crate::hir::types::InstructionKind::Reassign) | None => "",
                };
                // Register the variable as declared so later scopes don't re-declare it
                if !kw.is_empty() {
                    declared.insert(target_name.to_string());
                }
                kw
            };
            output.push_str(&format!("{indent}{keyword}{target_name} = {value_name};\n"));
        }
        InstructionValue::CallExpression { callee, args } => {
            let callee_name = resolve_place(callee, inline_map);
            let args_str: Vec<Cow<'_, str>> =
                args.iter().map(|a| resolve_place(a, inline_map)).collect();
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
            let receiver_name = resolve_place(receiver, inline_map);
            let args_str: Vec<Cow<'_, str>> =
                args.iter().map(|a| resolve_place(a, inline_map)).collect();
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
                resolve_place(object, inline_map),
                property
            ));
        }
        InstructionValue::PropertyStore { object, property, value } => {
            output.push_str(&format!(
                "{}{}.{} = {};\n",
                indent,
                resolve_place(object, inline_map),
                property,
                resolve_place(value, inline_map)
            ));
        }
        InstructionValue::BinaryExpression { op, left, right } => {
            let op_str = binary_op_str(*op);
            output.push_str(&format!(
                "{}{}{} = {} {} {};\n",
                indent,
                decl_keyword,
                lvalue_name,
                resolve_place(left, inline_map),
                op_str,
                resolve_place(right, inline_map)
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
                resolve_place(value, inline_map)
            ));
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            let raw_tag = resolve_place(tag, inline_map);
            // If the tag didn't resolve via the block-local inline map (still a
            // temp name like "t15"), check the global tag constants map. This
            // handles the case where the tag was assigned in a different scope
            // (e.g. `t15 = "button"` inside a reactive scope body) and thus
            // isn't in the current block's inline map.
            let raw_tag = if is_temp_place(tag) && !is_jsx_text_str(&raw_tag) {
                let temp_name = format!("t{}", tag.identifier.id.0);
                if let Some(constant_val) = tag_constants.get(&temp_name) {
                    Cow::Owned(constant_val.clone())
                } else {
                    raw_tag
                }
            } else {
                raw_tag
            };
            let tag_name = jsx_tag_name(&raw_tag);
            // Build JSX props string
            let props_str = build_jsx_props_str(props, inline_map);
            // Resolve children
            let resolved_children: Vec<Cow<'_, str>> =
                children.iter().map(|c| resolve_place(c, inline_map)).collect();

            if resolved_children.is_empty() {
                // Self-closing: <Tag props />
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = <{tag_name}{props_str} />;\n"
                ));
            } else if resolved_children.len() == 1 && !is_jsx_text_str(&resolved_children[0]) {
                // Single expression child inline: <Tag props>{child}</Tag>
                let child = jsx_child_str(&resolved_children[0]);
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = <{tag_name}{props_str}>{child}</{tag_name}>;\n"
                ));
            } else {
                // Multi-line: wrap in parens with indented children
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = (\n"));
                output.push_str(&format!("{indent}  <{tag_name}{props_str}>\n"));
                for child in &resolved_children {
                    let child_str = jsx_child_str(child);
                    output.push_str(&format!("{indent}    {child_str}\n"));
                }
                output.push_str(&format!("{indent}  </{tag_name}>\n"));
                output.push_str(&format!("{indent});\n"));
            }
        }
        InstructionValue::JsxFragment { children } => {
            let resolved_children: Vec<Cow<'_, str>> =
                children.iter().map(|c| resolve_place(c, inline_map)).collect();

            if resolved_children.is_empty() {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = <></>;\n"));
            } else if resolved_children.len() == 1 && !is_jsx_text_str(&resolved_children[0]) {
                let child = jsx_child_str(&resolved_children[0]);
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = <>{child}</>;\n"));
            } else {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = (\n"));
                output.push_str(&format!("{indent}  <>\n"));
                for child in &resolved_children {
                    let child_str = jsx_child_str(child);
                    output.push_str(&format!("{indent}    {child_str}\n"));
                }
                output.push_str(&format!("{indent}  </>\n"));
                output.push_str(&format!("{indent});\n"));
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
                            output.push_str(&format!(
                                "...{}",
                                resolve_place(&prop.value, inline_map)
                            ));
                        }
                        crate::hir::types::ObjectPropertyKey::Identifier(name) => {
                            if prop.shorthand {
                                output.push_str(name);
                            } else {
                                output.push_str(&format!(
                                    "{}: {}",
                                    name,
                                    resolve_place(&prop.value, inline_map)
                                ));
                            }
                        }
                        crate::hir::types::ObjectPropertyKey::Computed(key) => {
                            output.push_str(&format!(
                                "[{}]: {}",
                                resolve_place(key, inline_map),
                                resolve_place(&prop.value, inline_map)
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
                        output.push_str(&resolve_place(p, inline_map));
                    }
                    crate::hir::types::ArrayElement::Spread(p) => {
                        output.push_str(&format!("...{}", resolve_place(p, inline_map)));
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
                    output.push_str(&format!(
                        "${{{}}}",
                        resolve_place(&subexpressions[i], inline_map)
                    ));
                }
            }
            output.push_str("`;\n");
        }
        InstructionValue::NewExpression { callee, args } => {
            let callee_name = resolve_place(callee, inline_map);
            let args_str: Vec<Cow<'_, str>> =
                args.iter().map(|a| resolve_place(a, inline_map)).collect();
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
                resolve_place(value, inline_map)
            ));
        }
        InstructionValue::Destructure { lvalue_pattern, value } => {
            let value_name = resolve_place(value, inline_map);
            // Check if any of the pattern's top-level names are already declared
            let any_declared = pattern_has_declared_names(lvalue_pattern, declared);
            if any_declared {
                // Bare assignment — variables were declared by DeclareLocal or scope pre-declaration
                output.push_str(&format!("{indent}("));
                codegen_destructure_pattern(lvalue_pattern, output);
                output.push_str(&format!(" = {value_name});\n"));
            } else {
                // Use `let` instead of `const` so destructured names can be reassigned
                // by scope reload logic (else branches). `const` would create block-scoped
                // variables inside scope guard `if` blocks that can't be accessed outside.
                output.push_str(&format!("{indent}let "));
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
            let name = resolve_place(place, inline_map);
            if name != lvalue_name {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {name};\n"));
            }
        }
        InstructionValue::StoreContext { lvalue: target, value } => {
            let target_name = place_name(target);
            let value_name = resolve_place(value, inline_map);
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
            output.push_str(&format!("{indent}{name} = {};\n", resolve_place(value, inline_map)));
        }
        InstructionValue::ComputedLoad { object, property } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = {}[{}];\n",
                resolve_place(object, inline_map),
                resolve_place(property, inline_map)
            ));
        }
        InstructionValue::ComputedStore { object, property, value } => {
            output.push_str(&format!(
                "{indent}{}[{}] = {};\n",
                resolve_place(object, inline_map),
                resolve_place(property, inline_map),
                resolve_place(value, inline_map)
            ));
        }
        InstructionValue::PropertyDelete { object, property } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = delete {}.{};\n",
                resolve_place(object, inline_map),
                property
            ));
        }
        InstructionValue::ComputedDelete { object, property } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = delete {}[{}];\n",
                resolve_place(object, inline_map),
                resolve_place(property, inline_map)
            ));
        }
        InstructionValue::TypeCastExpression { value, .. } => {
            // Type casts are erased at runtime — just pass through the value
            let name = resolve_place(value, inline_map);
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
            let tag_name = resolve_place(tag, inline_map);
            output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = {tag_name}`"));
            for (i, quasi) in value.quasis.iter().enumerate() {
                output.push_str(quasi);
                if i < value.subexpressions.len() {
                    output.push_str(&format!(
                        "${{{}}}",
                        resolve_place(&value.subexpressions[i], inline_map)
                    ));
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
                resolve_place(collection, inline_map)
            ));
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = {}.next();\n",
                resolve_place(iterator, inline_map)
            ));
        }
        InstructionValue::NextPropertyOf { value } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = {};\n",
                resolve_place(value, inline_map)
            ));
        }
        InstructionValue::StartMemoize { .. } => {
            // Manual memoization marker — no runtime code needed
        }
        InstructionValue::FinishMemoize { decl, .. } => {
            let name = resolve_place(decl, inline_map);
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
    declared: &mut FxHashSet<String>,
    inline_map: &InlineMap,
    tag_constants: &TagConstantMap,
) {
    let indent_str = "  ".repeat(indent);

    match terminal {
        ReactiveTerminal::Return { value, .. } => {
            output.push_str(&format!(
                "{}return {};\n",
                indent_str,
                resolve_place(value, inline_map)
            ));
        }
        ReactiveTerminal::Throw { value, .. } => {
            output.push_str(&format!(
                "{}throw {};\n",
                indent_str,
                resolve_place(value, inline_map)
            ));
        }
        ReactiveTerminal::If { test, consequent, alternate, .. } => {
            output.push_str(&format!(
                "{}if ({}) {{\n",
                indent_str,
                resolve_place(test, inline_map)
            ));
            codegen_block(consequent, output, cache_slot, indent + 1, declared, tag_constants);
            if !alternate.instructions.is_empty() {
                output.push_str(&format!("{indent_str}}} else {{\n"));
                codegen_block(alternate, output, cache_slot, indent + 1, declared, tag_constants);
            }
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Switch { test, cases, .. } => {
            output.push_str(&format!(
                "{}switch ({}) {{\n",
                indent_str,
                resolve_place(test, inline_map)
            ));
            for (test_val, block) in cases {
                if let Some(tv) = test_val {
                    output.push_str(&format!(
                        "{}  case {}:\n",
                        indent_str,
                        resolve_place(tv, inline_map)
                    ));
                } else {
                    output.push_str(&format!("{indent_str}  default:\n"));
                }
                codegen_block(block, output, cache_slot, indent + 2, declared, tag_constants);
            }
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::While { test, body, .. } => {
            output.push_str(&format!("{indent_str}while (true) {{\n"));
            codegen_block(test, output, cache_slot, indent + 1, declared, tag_constants);
            codegen_block(body, output, cache_slot, indent + 1, declared, tag_constants);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::DoWhile { body, test, .. } => {
            output.push_str(&format!("{indent_str}do {{\n"));
            codegen_block(body, output, cache_slot, indent + 1, declared, tag_constants);
            output.push_str(&format!("{indent_str}}} while (true);\n"));
            // Test block is evaluated inside the loop for condition
            let _ = test;
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            output.push_str(&format!("{indent_str}for (;;) {{\n"));
            codegen_block(init, output, cache_slot, indent + 1, declared, tag_constants);
            codegen_block(test, output, cache_slot, indent + 1, declared, tag_constants);
            codegen_block(body, output, cache_slot, indent + 1, declared, tag_constants);
            if let Some(upd) = update {
                codegen_block(upd, output, cache_slot, indent + 1, declared, tag_constants);
            }
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::ForOf { init, test, body, .. } => {
            let (loop_var, collection) = extract_for_of_parts(init);
            output.push_str(&format!("{indent_str}for (const {loop_var} of {collection}) {{\n"));
            codegen_block(test, output, cache_slot, indent + 1, declared, tag_constants);
            codegen_block(body, output, cache_slot, indent + 1, declared, tag_constants);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::ForIn { init, test, body, .. } => {
            let (loop_var, collection) = extract_for_in_parts(init);
            output.push_str(&format!("{indent_str}for (const {loop_var} in {collection}) {{\n"));
            codegen_block(test, output, cache_slot, indent + 1, declared, tag_constants);
            codegen_block(body, output, cache_slot, indent + 1, declared, tag_constants);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            output.push_str(&format!("{indent_str}try {{\n"));
            codegen_block(block, output, cache_slot, indent + 1, declared, tag_constants);
            output.push_str(&format!("{indent_str}}} catch (e) {{\n"));
            codegen_block(handler, output, cache_slot, indent + 1, declared, tag_constants);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Label { block, label, .. } => {
            output.push_str(&format!("{indent_str}bb{label}: {{\n"));
            codegen_block(block, output, cache_slot, indent + 1, declared, tag_constants);
            output.push_str(&format!("{indent_str}}}\n"));
        }
    }
}

fn codegen_scope(
    scope: &ReactiveScopeBlock,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
    declared: &mut FxHashSet<String>,
    tag_constants: &TagConstantMap,
) {
    let indent_str = "  ".repeat(indent);
    let deps = &scope.scope.dependencies;
    let slot_start = *cache_slot;

    // Sort declarations by source location for deterministic cache slot ordering.
    // Upstream's CodegenReactiveFunction uses compareScopeDeclaration() for this.
    let mut sorted_decls: Vec<_> = scope.scope.declarations.iter().collect();
    sorted_decls.sort_by_key(|(_, decl)| decl.identifier.loc.start);

    // Hoist DeclareLocal instructions for scope DECLARATIONS only.
    // Variables that are scope outputs (stored in cache, loaded in else branch)
    // need `let` declarations before the scope guard. Variables that are only
    // used inside the scope body should remain as `const` inside the if-block.
    let scope_decl_ids: FxHashSet<IdentifierId> =
        scope.scope.declarations.iter().map(|(id, _)| *id).collect();
    let empty_inline_map = InlineMap::default();
    for instr in &scope.instructions.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr
            && let InstructionValue::DeclareLocal { lvalue, .. }
            | InstructionValue::DeclareContext { lvalue, .. } = &instruction.value
        {
            // Only hoist if this variable is a scope declaration (output)
            if scope_decl_ids.contains(&lvalue.identifier.id)
                || scope_decl_ids.contains(&instruction.lvalue.identifier.id)
            {
                codegen_instruction(
                    instruction,
                    output,
                    &indent_str,
                    declared,
                    &empty_inline_map,
                    tag_constants,
                );
            }
        }
    }

    // Pre-declare scope output variables with `let` so the else-branch
    // (cache reload) can assign to them. Variables already declared by
    // DeclareLocal above are skipped.
    for (_, decl) in &sorted_decls {
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
                let dep_name = dependency_display_name(dep);
                format!("$[{}] !== {}", slot_start + i as u32, dep_name)
            })
            .collect();
        output.push_str(&format!("{}if ({}) {{\n", indent_str, checks.join(" || ")));
        *cache_slot += deps.len() as u32;
    }

    // Generate scope body (DeclareLocal/DeclareContext already hoisted above)
    // Scope declaration names are "protected" — they're used by the cache storage
    // and else-branch reload, so they must not be eliminated as dead temps.
    let scope_decl_names: FxHashSet<String> = scope
        .scope
        .declarations
        .iter()
        .map(|(_, decl)| identifier_display_name(&decl.identifier).to_string())
        .collect();
    codegen_block_skip_declares(
        &scope.instructions,
        output,
        cache_slot,
        indent + 1,
        declared,
        &scope_decl_names,
        tag_constants,
    );

    // Store dep values and declarations into cache slots.
    //
    // For sentinel scopes (0 deps): declarations share the sentinel slot range.
    // The sentinel check uses $[slot_start], and declarations are stored at
    // $[slot_start], $[slot_start+1], ..., $[slot_start+N-1]. The sentinel
    // check's slot IS the first declaration's slot — storing a declaration
    // value there marks the sentinel as computed.
    //
    // For reactive scopes (>0 deps): deps occupy $[slot_start..slot_start+deps.len()],
    // then declarations occupy $[slot_start+deps.len()..].
    let inner_indent = "  ".repeat(indent + 1);
    if deps.is_empty() {
        // Sentinel scope: store declarations starting from slot_start
        // (reusing the sentinel slot for the first declaration)
        for (i, (_, decl)) in sorted_decls.iter().enumerate() {
            let decl_name = identifier_display_name(&decl.identifier);
            output.push_str(&format!(
                "{}$[{}] = {};\n",
                inner_indent,
                slot_start + i as u32,
                decl_name
            ));
        }
        // Advance cache_slot past the declarations (sentinel slot is included)
        *cache_slot = slot_start + (scope.scope.declarations.len() as u32).max(1);
    } else {
        // Reactive scope: store dep values for next comparison
        for (i, dep) in deps.iter().enumerate() {
            let dep_name = dependency_display_name(dep);
            output.push_str(&format!(
                "{}$[{}] = {};\n",
                inner_indent,
                slot_start + i as u32,
                dep_name
            ));
        }
        // Store declarations after deps
        let decl_slot_start = slot_start + deps.len() as u32;
        for (i, (_, decl)) in sorted_decls.iter().enumerate() {
            let decl_name = identifier_display_name(&decl.identifier);
            output.push_str(&format!(
                "{}$[{}] = {};\n",
                inner_indent,
                decl_slot_start + i as u32,
                decl_name
            ));
        }
        *cache_slot = decl_slot_start + scope.scope.declarations.len() as u32;
    }

    // Compute the declaration slot start for the else-branch reload
    let decl_reload_start =
        if deps.is_empty() { slot_start } else { slot_start + deps.len() as u32 };

    // Only emit else block if there are declarations to load from cache
    if !scope.scope.declarations.is_empty() {
        output.push_str(&format!("{indent_str}}} else {{\n"));

        // Load cached declarations
        for (i, (_, decl)) in sorted_decls.iter().enumerate() {
            let decl_name = identifier_display_name(&decl.identifier);
            let inner_indent = "  ".repeat(indent + 1);
            output.push_str(&format!(
                "{}{} = $[{}];\n",
                inner_indent,
                decl_name,
                decl_reload_start + i as u32
            ));
        }
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
    let mut declared = FxHashSet::default();
    let mut cache_slot = 0u32;
    // Nested function bodies (lambdas, callbacks) don't have reactive scopes,
    // so JSX tag constants within them are resolved locally.
    let tag_constants = build_tag_constant_map(&reactive_block);
    codegen_block(&reactive_block, output, &mut cache_slot, indent, &mut declared, &tag_constants);
}

/// Recursively collect all scope declaration names from the reactive block tree
/// and emit `let` declarations for them at the current indent level.
/// This ensures scope output variables are always declared before any scope
/// guard or StoreLocal can reference them.
fn collect_all_scope_declarations(
    block: &ReactiveBlock,
    output: &mut String,
    declared: &mut FxHashSet<String>,
    indent: usize,
) {
    let indent_str = "  ".repeat(indent);
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Scope(scope) => {
                // Sort declarations by source location for deterministic ordering
                let mut sorted_decls: Vec<_> = scope.scope.declarations.iter().collect();
                sorted_decls.sort_by_key(|(_, decl)| decl.identifier.loc.start);

                for (_, decl) in &sorted_decls {
                    let decl_name = identifier_display_name(&decl.identifier);
                    if !declared.contains(decl_name.as_ref()) {
                        declared.insert(decl_name.to_string());
                        output.push_str(&format!("{indent_str}let {decl_name};\n"));
                    }
                }
                // Recurse into scope body for nested scopes
                collect_all_scope_declarations(&scope.instructions, output, declared, indent);
            }
            ReactiveInstruction::Terminal(terminal) => {
                collect_scope_declarations_in_terminal(terminal, output, declared, indent);
            }
            ReactiveInstruction::Instruction(_) => {}
        }
    }
}

/// Recurse into terminal branches to find nested scopes.
fn collect_scope_declarations_in_terminal(
    terminal: &ReactiveTerminal,
    output: &mut String,
    declared: &mut FxHashSet<String>,
    indent: usize,
) {
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            collect_all_scope_declarations(consequent, output, declared, indent);
            collect_all_scope_declarations(alternate, output, declared, indent);
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                collect_all_scope_declarations(block, output, declared, indent);
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            collect_all_scope_declarations(init, output, declared, indent);
            collect_all_scope_declarations(test, output, declared, indent);
            if let Some(update) = update {
                collect_all_scope_declarations(update, output, declared, indent);
            }
            collect_all_scope_declarations(body, output, declared, indent);
        }
        ReactiveTerminal::ForOf { body, .. } | ReactiveTerminal::ForIn { body, .. } => {
            collect_all_scope_declarations(body, output, declared, indent);
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { test, body, .. } => {
            collect_all_scope_declarations(test, output, declared, indent);
            collect_all_scope_declarations(body, output, declared, indent);
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            collect_all_scope_declarations(block, output, declared, indent);
            collect_all_scope_declarations(handler, output, declared, indent);
        }
        ReactiveTerminal::Label { block, .. } => {
            collect_all_scope_declarations(block, output, declared, indent);
        }
        ReactiveTerminal::Return { .. } | ReactiveTerminal::Throw { .. } => {}
    }
}

/// Returns `true` if the reactive function has any cache slots to memoize.
///
/// When a function has 0 cache slots, it means no reactive scopes survived
/// the pruning pipeline and memoization would add no value. In this case,
/// the compiler should skip the function and return source unchanged,
/// matching Babel's behavior.
pub fn has_cache_slots(rf: &ReactiveFunction) -> bool {
    count_cache_slots(&rf.body) > 0
}

fn count_cache_slots(block: &ReactiveBlock) -> u32 {
    let mut count = 0u32;
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Scope(scope) => {
                let deps = &scope.scope.dependencies;
                let decls = scope.scope.declarations.len() as u32;
                if deps.is_empty() {
                    // Sentinel scope: the sentinel check reuses the first
                    // declaration's slot. Total = max(declarations, 1).
                    count += decls.max(1);
                } else {
                    // Reactive scope: deps + declarations as separate slots
                    count += deps.len() as u32 + decls;
                }
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
///
/// JSX syntax is preserved in output (not lowered to `_jsx()` calls), so only
/// the compiler-runtime import is needed. The downstream bundler's JSX
/// transform handles JSX lowering.
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

    // Add compiler-runtime import at the top (JSX syntax is preserved, no jsx-runtime needed)
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
/// Extract the loop variable name and collection expression from a `ForIn` init block.
/// The init block contains `NextPropertyOf { value: collection }` followed by
/// `StoreLocal/DeclareLocal { lvalue: loop_var }`.
fn extract_for_in_parts(init: &ReactiveBlock) -> (String, String) {
    let mut collection_name = "_".to_string();
    let mut loop_var_name = "_".to_string();

    for instr in &init.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr {
            match &instruction.value {
                InstructionValue::NextPropertyOf { value } => {
                    collection_name = place_name(value).into_owned();
                }
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::DeclareLocal { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name {
                        loop_var_name.clone_from(name);
                    }
                }
                _ => {}
            }
        }
    }
    (loop_var_name, collection_name)
}

/// Extract the loop variable name and collection expression from a `ForOf` init block.
/// The init block contains `IteratorNext { iterator }` followed by
/// `StoreLocal/DeclareLocal { lvalue: loop_var }`. The collection is found via
/// `GetIterator { collection }` which may be in a preceding block, but the iterator
/// place's name often reveals the collection.
fn extract_for_of_parts(init: &ReactiveBlock) -> (String, String) {
    let mut collection_name = "_".to_string();
    let mut loop_var_name = "_".to_string();

    for instr in &init.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr {
            match &instruction.value {
                InstructionValue::GetIterator { collection } => {
                    collection_name = place_name(collection).into_owned();
                }
                InstructionValue::IteratorNext { iterator, .. } => {
                    // If we haven't found the collection via GetIterator yet,
                    // try to use the iterator's place name
                    if collection_name == "_" {
                        collection_name = place_name(iterator).into_owned();
                    }
                }
                InstructionValue::StoreLocal { lvalue, .. }
                | InstructionValue::DeclareLocal { lvalue, .. } => {
                    if let Some(name) = &lvalue.identifier.name {
                        loop_var_name.clone_from(name);
                    }
                }
                _ => {}
            }
        }
    }
    (loop_var_name, collection_name)
}

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

/// Render a scope dependency as a string, including its property path.
/// E.g., `{identifier: props, path: ["x", "y"]}` → `"props.x.y"`.
fn dependency_display_name(dep: &crate::hir::types::ReactiveScopeDependency) -> String {
    let base = identifier_display_name(&dep.identifier);
    if dep.path.is_empty() {
        return base.into_owned();
    }
    let mut result = base.into_owned();
    for entry in &dep.path {
        if entry.optional {
            result.push_str("?.");
        } else {
            result.push('.');
        }
        result.push_str(&entry.property);
    }
    result
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
    declared: &FxHashSet<String>,
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
            if let Some(rest_place) = rest
                && declared.contains(place_name(rest_place).as_ref())
            {
                return true;
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
            if let Some(rest_place) = rest
                && declared.contains(place_name(rest_place).as_ref())
            {
                return true;
            }
            false
        }
    }
}

/// Collect all variable names from a destructure pattern into the declared set.
fn collect_pattern_names(
    pattern: &crate::hir::types::DestructurePattern,
    declared: &mut FxHashSet<String>,
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

    // Generate function header — preserve arrow vs function syntax from source.
    if rf.is_arrow {
        ctx.write("(");
    } else if let Some(ref name) = rf.id {
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
    if rf.is_arrow {
        ctx.write(") => {\n");
    } else {
        ctx.write(") {\n");
    }

    // Emit directives
    for directive in &rf.directives {
        ctx.write(&format!("  \"{directive}\";\n"));
    }

    let total_slots = count_cache_slots(&rf.body);
    if total_slots > 0 {
        ctx.write(&format!("  const $ = _c({total_slots});\n"));
    }

    let tag_constants = build_tag_constant_map(&rf.body);
    codegen_block_with_map(&rf.body, &mut ctx, &mut 0u32, 1, source_text, &tag_constants);

    ctx.write("}\n");
    (ctx.output, ctx.source_map)
}

fn codegen_block_with_map(
    block: &ReactiveBlock,
    ctx: &mut CodegenContext,
    cache_slot: &mut u32,
    indent: usize,
    source: &str,
    tag_constants: &TagConstantMap,
) {
    let inline_map = build_inline_map(&block.instructions, &FxHashSet::default(), tag_constants);
    let mut declared = FxHashSet::default();
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                let indent_str = "  ".repeat(indent);
                // Record mapping for this instruction.
                ctx.map_from_span(instruction.loc, source);
                codegen_instruction(
                    instruction,
                    &mut ctx.output,
                    &indent_str,
                    &mut declared,
                    &inline_map,
                    tag_constants,
                );
                // Update line/col tracking after codegen_instruction wrote to output.
                recompute_position(ctx);
            }
            ReactiveInstruction::Terminal(terminal) => {
                codegen_terminal(
                    terminal,
                    &mut ctx.output,
                    cache_slot,
                    indent,
                    &mut declared,
                    &inline_map,
                    tag_constants,
                );
                recompute_position(ctx);
            }
            ReactiveInstruction::Scope(scope_block) => {
                codegen_scope(
                    scope_block,
                    &mut ctx.output,
                    cache_slot,
                    indent,
                    &mut declared,
                    tag_constants,
                );
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
