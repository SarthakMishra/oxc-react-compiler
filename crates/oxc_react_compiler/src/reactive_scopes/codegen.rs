#![allow(dead_code)]

use std::borrow::Cow;

use crate::hir::types::{DeclarationId, IdentifierId};
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

/// Sentinel value in `InlineMap` indicating that a side-effecting call should
/// be emitted as a bare statement (`foo();`) without a `let tN =` prefix.
const STMT_ONLY_SENTINEL: &str = "\x01STMT";

/// Check if a string needs quoting when used as an object property key.
/// Returns true for strings that are NOT valid JS identifiers (contain dots,
/// spaces, operators, start with a digit, etc.).
fn needs_object_key_quoting(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    let mut chars = name.chars();
    // First char must be a letter, underscore, or dollar sign
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' && first != '$' {
        return true;
    }
    // Remaining chars must be alphanumeric, underscore, or dollar sign
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '$' {
            return true;
        }
    }
    false
}

/// Format an object property key for codegen output. Quotes the key if it's
/// not a valid JS identifier (e.g., "a.b", "a b", "a+b").
fn format_object_key(name: &str) -> Cow<'_, str> {
    if needs_object_key_quoting(name) {
        Cow::Owned(format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\"")))
    } else {
        Cow::Borrowed(name)
    }
}

/// Returns `true` when the identifier corresponds to a compiler-generated
/// temporary (unnamed, printed as `tN`).
fn is_temp_place(place: &Place) -> bool {
    match &place.identifier.name {
        None => true,
        // After promote_used_temporaries, unnamed temps get synthetic names
        // like "t{id}". Detect these so we can still inline them.
        Some(name) => is_temp_var_name(name),
    }
}

/// Check if a name matches the compiler temporary pattern `t{digits}`.
/// NOTE: Duplicated in prune_scopes.rs — both are module-private.
fn is_temp_var_name(name: &str) -> bool {
    name.starts_with('t') && name.len() >= 2 && name[1..].chars().all(|c| c.is_ascii_digit())
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
                // Count uses from scope dependencies (these become `$[N] !== dep`
                // checks in codegen). Without this, temps used only as scope
                // dependencies would be incorrectly marked as dead.
                for dep in &scope_block.scope.dependencies {
                    if dep.identifier.name.is_none() {
                        let name = format!("t{}", dep.identifier.id.0);
                        *counts.entry(name).or_insert(0) += 1;
                    }
                }
                // Count uses from scope declarations (stored/reloaded in cache)
                for (_, decl) in &scope_block.scope.declarations {
                    if decl.identifier.name.is_none() {
                        let name = format!("t{}", decl.identifier.id.0);
                        *counts.entry(name).or_insert(0) += 1;
                    }
                }
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
        ReactiveTerminal::Logical { right, .. } => {
            count_temp_uses_in_slice(&right.instructions, counts);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
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
        | ReactiveTerminal::Label { .. }
        | ReactiveTerminal::Logical { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
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
        InstructionValue::CallExpression { callee, args, .. } => {
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
        InstructionValue::ComputedLoad { object, property, .. } => {
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
        InstructionValue::Destructure { value, lvalue_pattern } => {
            bump_temp(value, counts);
            // Count uses of default value temps in destructure properties
            bump_destructure_default_temps(lvalue_pattern, counts);
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

/// Count uses of default value temps in destructure patterns.
/// Default values (e.g., `{ x = defaultVal }`) reference temps that hold
/// the default expressions. These are real uses that prevent the temp from
/// being eliminated as dead code.
fn bump_destructure_default_temps(
    pattern: &crate::hir::types::DestructurePattern,
    counts: &mut FxHashMap<String, u32>,
) {
    use crate::hir::types::DestructurePattern;
    match pattern {
        DestructurePattern::Object { properties, .. } => {
            for prop in properties {
                if let Some(ref default_place) = prop.default_value {
                    bump_temp(default_place, counts);
                }
                // Recurse into nested patterns
                if let crate::hir::types::DestructureTarget::Pattern(nested) = &prop.value {
                    bump_destructure_default_temps(nested, counts);
                }
            }
        }
        DestructurePattern::Array { items, .. } => {
            for item in items {
                if let crate::hir::types::DestructureArrayItem::Value(
                    crate::hir::types::DestructureTarget::Pattern(nested),
                ) = item
                {
                    bump_destructure_default_temps(nested, counts);
                }
            }
        }
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

/// Recursively collect scope-related temp identifiers from the instruction
/// subtree. Adds scope declaration temps and scope dependency temps to
/// `scope_output_temps`, and destructure-phantom temps to
/// `phantom_destructure_temps`. This is used to protect these temps from
/// being inlined or dead-code-eliminated by `build_inline_map`.
fn collect_scope_temps_recursive(
    instructions: &[ReactiveInstruction],
    scope_output_temps: &mut FxHashSet<String>,
    phantom_destructure_temps: &mut FxHashSet<String>,
) {
    for ri in instructions {
        match ri {
            ReactiveInstruction::Scope(scope_block) => {
                let destructured_declare_ids = find_destructured_declare_ids(scope_block);
                for (id, decl) in &scope_block.scope.declarations {
                    let decl_name = identifier_display_name(&decl.identifier);
                    if destructured_declare_ids.contains(id) {
                        phantom_destructure_temps.insert(decl_name.to_string());
                    } else {
                        scope_output_temps.insert(decl_name.to_string());
                    }
                }
                // Scope dependencies are also critical — they appear in
                // `$[N] !== dep` guard checks. Without protecting them,
                // the inline map may remove or inline the producing
                // instruction, leaving the dependency name undefined.
                for dep in &scope_block.scope.dependencies {
                    if dep.identifier.name.is_none() {
                        let dep_name = format!("t{}", dep.identifier.id.0);
                        scope_output_temps.insert(dep_name);
                    }
                }
                // Recurse into scope body for nested scopes
                collect_scope_temps_recursive(
                    &scope_block.instructions.instructions,
                    scope_output_temps,
                    phantom_destructure_temps,
                );
            }
            ReactiveInstruction::Terminal(terminal) => {
                visit_terminal_for_scope_temps(
                    terminal,
                    scope_output_temps,
                    phantom_destructure_temps,
                );
            }
            ReactiveInstruction::Instruction(_) => {}
        }
    }
}

/// Recurse into terminal child blocks to find nested scopes for
/// `collect_scope_temps_recursive`.
fn visit_terminal_for_scope_temps(
    terminal: &ReactiveTerminal,
    scope_output_temps: &mut FxHashSet<String>,
    phantom_destructure_temps: &mut FxHashSet<String>,
) {
    match terminal {
        ReactiveTerminal::If { consequent, alternate, .. } => {
            collect_scope_temps_recursive(
                &consequent.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
            collect_scope_temps_recursive(
                &alternate.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
        }
        ReactiveTerminal::Switch { cases, .. } => {
            for (_, block) in cases {
                collect_scope_temps_recursive(
                    &block.instructions,
                    scope_output_temps,
                    phantom_destructure_temps,
                );
            }
        }
        ReactiveTerminal::For { init, test, update, body, .. } => {
            collect_scope_temps_recursive(
                &init.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
            collect_scope_temps_recursive(
                &test.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
            if let Some(update) = update {
                collect_scope_temps_recursive(
                    &update.instructions,
                    scope_output_temps,
                    phantom_destructure_temps,
                );
            }
            collect_scope_temps_recursive(
                &body.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
        }
        ReactiveTerminal::ForOf { init, test, body, .. }
        | ReactiveTerminal::ForIn { init, test, body, .. } => {
            collect_scope_temps_recursive(
                &init.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
            collect_scope_temps_recursive(
                &test.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
            collect_scope_temps_recursive(
                &body.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
        }
        ReactiveTerminal::While { test, body, .. }
        | ReactiveTerminal::DoWhile { test, body, .. } => {
            collect_scope_temps_recursive(
                &test.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
            collect_scope_temps_recursive(
                &body.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            collect_scope_temps_recursive(
                &block.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
            collect_scope_temps_recursive(
                &handler.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
        }
        ReactiveTerminal::Label { block, .. } => {
            collect_scope_temps_recursive(
                &block.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
        }
        ReactiveTerminal::Logical { right, .. } => {
            collect_scope_temps_recursive(
                &right.instructions,
                scope_output_temps,
                phantom_destructure_temps,
            );
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
    }
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
    // HOWEVER, DeclareLocal temps for destructured variables are excluded:
    // when a scope contains a Destructure, the DeclareLocal temps for the
    // destructured names are hoisted out and replaced by the actual pattern names.
    // The DeclareLocal temp is never assigned a useful value, so caching it
    // would store undefined.
    let mut scope_output_temps: FxHashSet<String> = FxHashSet::default();
    let mut phantom_destructure_temps: FxHashSet<String> = FxHashSet::default();
    collect_scope_temps_recursive(
        instructions,
        &mut scope_output_temps,
        &mut phantom_destructure_temps,
    );
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
            if scope_output_temps.contains(&temp_name) || protected_names.contains(&temp_name) {
                // Scope outputs / protected names must always be emitted.
                continue;
            }
            // Dead call/method/new temps: emit as bare statement (no `let tN =`)
            if matches!(
                &instr.value,
                InstructionValue::CallExpression { .. }
                    | InstructionValue::MethodCall { .. }
                    | InstructionValue::NewExpression { .. }
            ) {
                inline_map.insert(temp_name, STMT_ONLY_SENTINEL.to_string());
                continue;
            }
            // Other side-effecting instructions must be emitted normally
            if matches!(
                &instr.value,
                InstructionValue::PropertyStore { .. }
                    | InstructionValue::ComputedStore { .. }
                    | InstructionValue::StoreLocal { .. }
                    | InstructionValue::StoreContext { .. }
                    | InstructionValue::StoreGlobal { .. }
                    | InstructionValue::Destructure { .. }
                    | InstructionValue::PrefixUpdate { .. }
                    | InstructionValue::PostfixUpdate { .. }
            ) {
                continue;
            }
            // Pure dead temp — mark for removal by inserting empty sentinel
            inline_map.insert(temp_name, String::new()); // sentinel: skip emission
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

    // Add phantom destructure temps as dead (empty sentinel) so that:
    // 1. Any instruction whose lvalue is the phantom temp gets skipped
    // 2. StoreLocal instructions that reference the phantom temp get skipped
    //    (codegen_instruction checks for empty value_name)
    for name in &phantom_destructure_temps {
        inline_map.insert(name.clone(), String::new());
    }

    inline_map
}

/// Build a map of temp names → user variable names for "name promotion".
///
/// When a non-inlinable temp (e.g., FunctionExpression) is assigned and then
/// immediately stored to a named variable via StoreLocal, we promote the temp's
/// name to the user variable name. The codegen emits the temp instruction using
/// the user name directly and skips the redundant StoreLocal.
///
/// Pattern: `t5 = FunctionExpression(...)` followed by `x = StoreLocal(value: t5)`
/// → promote `t5` to `x`, skip the StoreLocal.
fn build_name_promotion_map(
    instructions: &[ReactiveInstruction],
    inline_map: &InlineMap,
) -> FxHashMap<String, String> {
    let mut promotions: FxHashMap<String, String> = FxHashMap::default();

    for window in instructions.windows(2) {
        let (
            ReactiveInstruction::Instruction(temp_instr),
            ReactiveInstruction::Instruction(store_instr),
        ) = (&window[0], &window[1])
        else {
            continue;
        };

        if !is_temp_place(&temp_instr.lvalue) {
            continue;
        }
        let temp_name = format!("t{}", temp_instr.lvalue.identifier.id.0);

        if inline_map.contains_key(&temp_name) {
            continue;
        }

        if let InstructionValue::StoreLocal { lvalue, value, .. } = &store_instr.value
            && let Some(n) = &lvalue.identifier.name
            && !n.is_empty()
            && is_temp_place(value)
            && value.identifier.id == temp_instr.lvalue.identifier.id
        {
            promotions.insert(temp_name, n.clone());
        }
    }

    promotions
}

/// Check if a StoreLocal instruction should be skipped because its value
/// temp has been name-promoted (the temp's defining instruction will use
/// the promoted name directly).
fn should_skip_for_promotion(
    instr: &crate::hir::types::Instruction,
    promotions: &FxHashMap<String, String>,
) -> bool {
    if promotions.is_empty() {
        return false;
    }
    if let InstructionValue::StoreLocal { value, .. } = &instr.value
        && is_temp_place(value)
    {
        let temp_name = format!("t{}", value.identifier.id.0);
        return promotions.contains_key(&temp_name);
    }
    false
}

/// Get the promoted name for a temp place, if one exists.
fn get_promoted_name(place: &Place, promotions: &FxHashMap<String, String>) -> Option<String> {
    if promotions.is_empty() || !is_temp_place(place) {
        return None;
    }
    let temp_name = format!("t{}", place.identifier.id.0);
    promotions.get(&temp_name).cloned()
}

/// Build a map of scope-output IdentifierId → user variable name for
/// "scope output promotion".
///
/// When a reactive scope block is immediately followed by a StoreLocal or
/// DeclareLocal+StoreLocal that assigns a scope output temp to a named
/// variable, we can promote the scope declaration to use the user variable
/// name directly. This avoids emitting unnecessary temporaries like:
///   let t7; if (...) { t7 = ...; $[0] = t7; } else { t7 = $[0]; } let x = t7;
/// and instead emits:
///   let x; if (...) { x = ...; $[0] = x; } else { x = $[0]; }
///
/// Returns a map from scope-output IdentifierId to the user variable name,
/// and a set of instruction indices to skip (the StoreLocal/DeclareLocal that
/// were folded into the scope).
fn build_scope_output_promotions(
    instructions: &[ReactiveInstruction],
) -> (FxHashMap<IdentifierId, String>, FxHashSet<usize>) {
    let mut promotions: FxHashMap<IdentifierId, String> = FxHashMap::default();
    let mut skip_indices: FxHashSet<usize> = FxHashSet::default();

    let mut i = 0;
    while i < instructions.len() {
        if let ReactiveInstruction::Scope(scope_block) = &instructions[i] {
            // Look at instructions following this scope for StoreLocal patterns
            // that assign scope output temps to named variables.
            let mut j = i + 1;

            // Skip over DeclareLocal instructions that just declare the target
            while j < instructions.len()
                && let ReactiveInstruction::Instruction(instr) = &instructions[j]
                && matches!(
                    &instr.value,
                    InstructionValue::DeclareLocal { .. } | InstructionValue::DeclareContext { .. }
                )
            {
                j += 1;
            }

            if j < instructions.len()
                && let ReactiveInstruction::Instruction(store_instr) = &instructions[j]
                && let InstructionValue::StoreLocal { lvalue: target, value: source, .. } =
                    &store_instr.value
            {
                // Check if the source matches a scope output declaration.
                // After SSA renaming and promote_used_temporaries, the IDs
                // may differ between the scope declaration and the StoreLocal
                // source, so we match by display name instead.
                let source_display = identifier_display_name(&source.identifier).to_string();
                let matching_decl =
                    scope_block.scope.declarations.iter().find(|(_, decl)| {
                        identifier_display_name(&decl.identifier) == source_display
                    });

                if let Some((decl_id, _)) = matching_decl
                    && let Some(ref target_name) = target.identifier.name
                    && !target_name.is_empty()
                    && !is_temp_place(target)
                {
                    // Promote: replace the scope output with the named variable
                    promotions.insert(*decl_id, target_name.clone());
                    // Skip the StoreLocal and any DeclareLocal before it
                    skip_indices.insert(j);
                    // Also skip DeclareLocal instructions between scope and
                    // StoreLocal that declare the target variable
                    for (k, instr_k) in instructions.iter().enumerate().take(j).skip(i + 1) {
                        if let ReactiveInstruction::Instruction(decl_instr) = instr_k
                            && let InstructionValue::DeclareLocal { lvalue, .. } = &decl_instr.value
                            && lvalue.identifier.name.as_deref() == Some(target_name)
                        {
                            skip_indices.insert(k);
                        }
                    }
                }
            }
        }
        i += 1;
    }

    (promotions, skip_indices)
}

/// Check if a StoreLocal instruction should be skipped because it assigns
/// a scope-output temp that has been promoted to a named variable.
fn should_skip_scope_promotion(idx: usize, skip_indices: &FxHashSet<usize>) -> bool {
    skip_indices.contains(&idx)
}

/// Build the set of DeclareLocal instruction indices that can be merged with
/// their immediately following StoreLocal. When merged, the DeclareLocal is
/// suppressed and the StoreLocal emits the declaration keyword inline
/// (`let x = expr;` instead of `let x;\nx = expr;`).
fn build_declare_merge_set(
    instructions: &[ReactiveInstruction],
    scope_skip_indices: &FxHashSet<usize>,
    pre_declared: &FxHashSet<String>,
    name_promotions: &FxHashMap<String, String>,
) -> FxHashSet<usize> {
    let mut merge_set = FxHashSet::default();
    for i in 0..instructions.len().saturating_sub(1) {
        // Already handled by scope output promotions — skip
        if scope_skip_indices.contains(&i) {
            continue;
        }
        // Must be a DeclareLocal instruction
        let ReactiveInstruction::Instruction(decl_instr) = &instructions[i] else {
            continue;
        };
        let InstructionValue::DeclareLocal { lvalue: decl_lvalue, .. } = &decl_instr.value else {
            continue;
        };
        let decl_name = place_name(decl_lvalue);
        // Must NOT already be declared (scope output pre-declaration)
        if pre_declared.contains(decl_name.as_ref()) {
            continue;
        }
        let next_i = i + 1;
        // Next instruction must not be folded into scope promotion
        if scope_skip_indices.contains(&next_i) {
            continue;
        }
        // Next must be a StoreLocal for the same variable
        let ReactiveInstruction::Instruction(store_instr) = &instructions[next_i] else {
            continue;
        };
        let InstructionValue::StoreLocal { lvalue: target, .. } = &store_instr.value else {
            continue;
        };
        if place_name(target) != decl_name {
            continue;
        }
        // Must not be a promotion-skip StoreLocal (name-promoted away)
        if should_skip_for_promotion(store_instr, name_promotions) {
            continue;
        }
        merge_set.insert(i);
    }
    merge_set
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
        InstructionValue::PropertyLoad { object, property, optional } => {
            let access = if *optional { "?." } else { "." };
            Some(format!("{}{access}{}", resolve(object), property))
        }
        InstructionValue::ComputedLoad { object, property, optional } => {
            let prop_str = resolve(property);
            if is_dotable_string_literal(&prop_str) {
                let access = if *optional { "?." } else { "." };
                Some(format!("{}{access}{}", resolve(object), extract_dotable_id(&prop_str)))
            } else if *optional {
                Some(format!("{}?.[{}]", resolve(object), prop_str))
            } else {
                Some(format!("{}[{}]", resolve(object), prop_str))
            }
        }
        InstructionValue::BinaryExpression { op, left, right } => {
            // Wrap in parens so that when this expression is inlined into another
            // binary expression, operator precedence is preserved.
            // e.g., `(a - b) / c` instead of `a - b / c`
            Some(format!("({} {} {})", resolve(left), binary_op_str(*op), resolve(right)))
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
                            parts.push(format!(
                                "{}: {}",
                                format_object_key(name),
                                resolve(&prop.value)
                            ));
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
        InstructionValue::CallExpression { callee, args, optional } => {
            let callee_name = resolve(callee);
            let call_op = if *optional { "?." } else { "" };
            let args_str: Vec<String> = args.iter().map(&resolve).collect();
            Some(format!("{}{}({})", callee_name, call_op, args_str.join(", ")))
        }
        InstructionValue::MethodCall { receiver, property, args, optional, optional_receiver } => {
            let receiver_name = resolve(receiver);
            let member_op = if *optional_receiver { "?." } else { "." };
            let call_op = if *optional { "?." } else { "" };
            let args_str: Vec<String> = args.iter().map(&resolve).collect();
            Some(format!(
                "{}{}{}{}({})",
                receiver_name,
                member_op,
                property,
                call_op,
                args_str.join(", ")
            ))
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
        ReactiveTerminal::Logical { right, .. } => {
            collect_tag_constants_from_block(right, map);
        }
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => {}
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
/// - String literals → raw text (strip quotes, unescape JS string escapes)
/// - Nested JSX elements (starts with `<`) → embed directly (no `{}` wrapper)
/// - Everything else → wrap in `{expr}`
fn jsx_child_str(resolved: &str) -> String {
    if is_jsx_text_str(resolved) {
        // Strip surrounding quotes and unescape JS string escapes back to raw text.
        // e.g., `"\"foo\""` → `"foo"` (literal quotes in JSX text)
        let raw = strip_string_quotes(resolved);
        raw.replace("\\\"", "\"").replace("\\\\", "\\").replace("\\n", "\n").replace("\\r", "\r")
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

/// Returns `true` if `s` is a quoted string literal whose content is a valid
/// JavaScript identifier (so `obj["foo"]` can be emitted as `obj.foo`).
fn is_dotable_string_literal(s: &str) -> bool {
    if s.len() < 3 || !s.starts_with('"') || !s.ends_with('"') {
        return false;
    }
    let inner = &s[1..s.len() - 1];
    if inner.is_empty() {
        return false;
    }
    let mut chars = inner.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' && first != '$' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Extract the unquoted identifier from a dotable string literal.
fn extract_dotable_id(s: &str) -> &str {
    &s[1..s.len() - 1]
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
    let async_prefix = if rf.is_async { "async " } else { "" };
    let generator_star = if rf.is_generator { "*" } else { "" };
    if rf.is_arrow {
        output.push_str(&format!("{async_prefix}("));
    } else if let Some(ref name) = rf.id {
        output.push_str(&format!("{async_prefix}function{generator_star} {name}("));
    } else {
        output.push_str(&format!("{async_prefix}function{generator_star} ("));
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

    // Build a global map of temp → constant expression for JSX tag resolution.
    // This allows JSX tags assigned in one scope to be resolved in another.
    let tag_constants = build_tag_constant_map(&rf.body);

    // Hoist Destructure instructions that destructure from function parameters
    // (e.g., `const { status } = t0;`) to the top of the function body,
    // before any reactive scope checks that may reference those variables.
    // This is emitted BEFORE scope pre-declarations to match upstream ordering
    // where destructured params appear before scope output `let` declarations.
    //
    // Build an inline map for default value temps so they're resolved correctly.
    // Default value temps (e.g., `t4 = "member"`) are emitted before the Destructure
    // in the instruction list but won't be in a full block inline_map since we
    // process hoisted instructions separately.
    let mut hoisted_indices = FxHashSet::default();
    for (i, instr) in rf.body.instructions.iter().enumerate() {
        if let ReactiveInstruction::Instruction(instruction) = instr
            && let InstructionValue::Destructure { value, lvalue_pattern } = &instruction.value
        {
            let value_name = place_name(value);
            if param_names.contains(value_name.as_ref()) {
                // Build a mini inline map for default value temps.
                // Find instructions that produce default values referenced by this pattern.
                let mut default_inline_map: InlineMap = FxHashMap::default();
                collect_default_value_inline_entries(
                    lvalue_pattern,
                    &rf.body.instructions,
                    &mut default_inline_map,
                    &tag_constants,
                );
                // Also mark default value temp instructions as hoisted (skip in body)
                for (j, other) in rf.body.instructions.iter().enumerate() {
                    if let ReactiveInstruction::Instruction(other_instr) = other {
                        let temp_name = format!("t{}", other_instr.lvalue.identifier.id.0);
                        if default_inline_map.contains_key(&temp_name) {
                            hoisted_indices.insert(j);
                        }
                    }
                }

                let indent_str = "  ";
                codegen_instruction(
                    instruction,
                    &mut output,
                    indent_str,
                    &mut declared,
                    &default_inline_map,
                    &tag_constants,
                );
                hoisted_indices.insert(i);
            }
        }
    }

    // Scope declarations are emitted lazily by `codegen_scope` just before
    // each scope guard, matching upstream's CodegenReactiveFunction.ts.

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
    renumber_temps_in_output(&mut output);
    output
}

/// Renumber all compiler-generated temporaries (`t5`, `t7`, `t10`, ...) in the
/// codegen output string to sequential `t0`, `t1`, `t2`, ... in order of first
/// appearance. This matches upstream's sequential temp naming convention.
///
/// Uses a two-pass approach (discover then replace) to avoid cascade issues
/// where sequential string replacements like `t7→t1` then `t1→t0` would
/// double-rename.
fn renumber_temps_in_output(output: &mut String) {
    fn is_ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_' || c == '$'
    }

    // Single-pass scan and replace: find each t{N} token, look up its
    // sequential replacement, and build the result string directly.
    let mut seen: FxHashMap<String, u32> = FxHashMap::default();
    let mut next_id = 0u32;
    let mut any_rename_needed = false;

    // First pass: discover all temp tokens and assign sequential IDs.
    let chars: Vec<char> = output.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == 't' {
            let start = i;
            i += 1;
            if i < chars.len() && chars[i].is_ascii_digit() {
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let at_end = i >= chars.len() || !is_ident_char(chars[i]);
                let at_start = start == 0 || !is_ident_char(chars[start - 1]);
                if at_start && at_end {
                    let token: String = chars[start..i].iter().collect();
                    let assigned = *seen.entry(token.clone()).or_insert_with(|| {
                        let id = next_id;
                        next_id += 1;
                        id
                    });
                    let expected = format!("t{assigned}");
                    if token != expected {
                        any_rename_needed = true;
                    }
                }
            }
        } else {
            i += 1;
        }
    }

    if !any_rename_needed {
        return;
    }

    // Second pass: build result with all renames applied in one go.
    let mut result = String::with_capacity(output.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == 't' {
            let start = i;
            i += 1;
            if i < chars.len() && chars[i].is_ascii_digit() {
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let at_end = i >= chars.len() || !is_ident_char(chars[i]);
                let at_start = start == 0 || !is_ident_char(chars[start - 1]);
                if at_start && at_end {
                    let token: String = chars[start..i].iter().collect();
                    if let Some(&new_idx) = seen.get(&token) {
                        result.push_str(&format!("t{new_idx}"));
                        continue;
                    }
                }
            }
            // Not a temp token — emit characters as-is
            for c in &chars[start..i] {
                result.push(*c);
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    *output = result;
}

fn codegen_block(
    block: &ReactiveBlock,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
    declared: &mut FxHashSet<String>,
    tag_constants: &TagConstantMap,
) {
    // Collect scope-related temps from the entire subtree so they're
    // protected from inlining/dead-code elimination by build_inline_map.
    let mut scope_temps = FxHashSet::default();
    let mut phantom = FxHashSet::default();
    collect_scope_temps_recursive(&block.instructions, &mut scope_temps, &mut phantom);
    let inline_map = build_inline_map(&block.instructions, &scope_temps, tag_constants);
    let name_promotions = build_name_promotion_map(&block.instructions, &inline_map);
    // Build scope output promotions: when a scope's temp output is immediately
    // stored to a named variable, use the named variable as the scope declaration.
    let (scope_output_promotions, scope_skip_indices) =
        build_scope_output_promotions(&block.instructions);
    // Build declare-merge set: DeclareLocal instructions that are immediately
    // followed by a StoreLocal for the same variable can be merged into a single
    // `let/const/var x = expr;` statement. We skip the DeclareLocal and let the
    // StoreLocal emit the declaration keyword (since the name won't be in `declared`).
    let declare_merge_set = build_declare_merge_set(
        &block.instructions,
        &scope_skip_indices,
        declared,
        &name_promotions,
    );
    for (idx, instr) in block.instructions.iter().enumerate() {
        // Skip instructions that were folded into scope output promotions
        if should_skip_scope_promotion(idx, &scope_skip_indices) {
            continue;
        }
        // Skip DeclareLocal instructions that will be merged into their following StoreLocal
        if declare_merge_set.contains(&idx) {
            continue;
        }
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                // Skip StoreLocal whose value temp has been name-promoted
                if should_skip_for_promotion(instruction, &name_promotions) {
                    continue;
                }
                let indent_str = "  ".repeat(indent);
                // Apply name promotion: emit with user variable name instead of temp
                if let Some(promoted) = get_promoted_name(&instruction.lvalue, &name_promotions) {
                    let mut promoted_instr = instruction.clone();
                    promoted_instr.lvalue.identifier.name = Some(promoted);
                    codegen_instruction(
                        &promoted_instr,
                        output,
                        &indent_str,
                        declared,
                        &inline_map,
                        tag_constants,
                    );
                } else {
                    codegen_instruction(
                        instruction,
                        output,
                        &indent_str,
                        declared,
                        &inline_map,
                        tag_constants,
                    );
                }
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
                codegen_scope(
                    scope_block,
                    output,
                    cache_slot,
                    indent,
                    declared,
                    tag_constants,
                    &scope_output_promotions,
                );
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
    // Collect scope-related temps from the entire subtree so they're
    // protected from inlining/dead-code elimination.
    let mut scope_temps = FxHashSet::default();
    let mut phantom = FxHashSet::default();
    collect_scope_temps_recursive(&block.instructions, &mut scope_temps, &mut phantom);
    let inline_map = build_inline_map(&block.instructions, &scope_temps, tag_constants);
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
                let empty_promotions = FxHashMap::default();
                codegen_scope(
                    scope_block,
                    output,
                    cache_slot,
                    indent,
                    declared,
                    tag_constants,
                    &empty_promotions,
                );
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
    codegen_block_skip_declares_and_ids(
        block,
        output,
        cache_slot,
        indent,
        declared,
        protected_names,
        tag_constants,
        &FxHashSet::default(),
    );
}

fn codegen_block_skip_declares_and_ids(
    block: &ReactiveBlock,
    output: &mut String,
    cache_slot: &mut u32,
    indent: usize,
    declared: &mut FxHashSet<String>,
    protected_names: &FxHashSet<String>,
    tag_constants: &TagConstantMap,
    skip_lvalue_ids: &FxHashSet<IdentifierId>,
) {
    let inline_map = build_inline_map(&block.instructions, protected_names, tag_constants);
    let name_promotions = build_name_promotion_map(&block.instructions, &inline_map);
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                if matches!(
                    &instruction.value,
                    InstructionValue::DeclareLocal { .. } | InstructionValue::DeclareContext { .. }
                ) {
                    continue;
                }
                // Skip instructions whose lvalue IDs are in the skip set
                // (e.g., hoisted Destructure instructions)
                if skip_lvalue_ids.contains(&instruction.lvalue.identifier.id) {
                    continue;
                }
                // Skip StoreLocal whose value temp has been name-promoted
                if should_skip_for_promotion(instruction, &name_promotions) {
                    continue;
                }
                let indent_str = "  ".repeat(indent);
                // Apply name promotion
                if let Some(promoted) = get_promoted_name(&instruction.lvalue, &name_promotions) {
                    let mut promoted_instr = instruction.clone();
                    promoted_instr.lvalue.identifier.name = Some(promoted);
                    codegen_instruction(
                        &promoted_instr,
                        output,
                        &indent_str,
                        declared,
                        &inline_map,
                        tag_constants,
                    );
                } else {
                    codegen_instruction(
                        instruction,
                        output,
                        &indent_str,
                        declared,
                        &inline_map,
                        tag_constants,
                    );
                }
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
                let empty_promotions = FxHashMap::default();
                codegen_scope(
                    scope_block,
                    output,
                    cache_slot,
                    indent,
                    declared,
                    tag_constants,
                    &empty_promotions,
                );
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
    // Exception: STMT_ONLY_SENTINEL means "emit as bare statement without lvalue".
    let is_stmt_only = if is_temp_place(&instr.lvalue) {
        let temp_name = format!("t{}", instr.lvalue.identifier.id.0);
        if let Some(mapped) = inline_map.get(&temp_name) {
            if mapped == STMT_ONLY_SENTINEL {
                true // fall through to emit as bare statement
            } else {
                return; // inlined or dead — skip emission
            }
        } else {
            false
        }
    } else {
        false
    };

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
            // Skip StoreLocal when the value is a phantom destructure temp.
            // Check both resolve_place result (for true temps) and direct
            // inline_map lookup by name (for promoted temps whose name doesn't
            // match their ID, e.g. name="t61" but id=4).
            if value_name.is_empty() {
                return;
            }
            let value_display = place_name(value);
            if let Some(mapped) = inline_map.get(value_display.as_ref())
                && mapped.is_empty()
            {
                return;
            }
            let already_declared = declared.contains(target_name.as_ref());
            let keyword = if already_declared {
                ""
            } else {
                // Use `const` for Const/HoistedConst when the variable hasn't been
                // pre-declared by codegen_scope (not a scope output). Scope output
                // variables are pre-declared with `let` and already in `declared`,
                // so they get bare assignment above. Non-scope-output variables
                // keep their original `const` keyword matching upstream behavior.
                // `let` and `var` are preserved as-is. HoistedFunction uses `let`
                // because function declarations may be reassigned in some patterns.
                let kw = match type_ {
                    Some(
                        crate::hir::types::InstructionKind::Const
                        | crate::hir::types::InstructionKind::HoistedConst,
                    ) => "const ",
                    Some(
                        crate::hir::types::InstructionKind::Let
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
        InstructionValue::CallExpression { callee, args, optional } => {
            let callee_name = resolve_place(callee, inline_map);
            let call_op = if *optional { "?." } else { "" };
            let args_str: Vec<Cow<'_, str>> =
                args.iter().map(|a| resolve_place(a, inline_map)).collect();
            if is_stmt_only {
                output.push_str(&format!(
                    "{}{}{}({});\n",
                    indent,
                    callee_name,
                    call_op,
                    args_str.join(", ")
                ));
            } else {
                output.push_str(&format!(
                    "{}{}{} = {}{}({});\n",
                    indent,
                    decl_keyword,
                    lvalue_name,
                    callee_name,
                    call_op,
                    args_str.join(", ")
                ));
            }
        }
        InstructionValue::MethodCall { receiver, property, args, optional, optional_receiver } => {
            let receiver_name = resolve_place(receiver, inline_map);
            let member_op = if *optional_receiver { "?." } else { "." };
            let call_op = if *optional { "?." } else { "" };
            let args_str: Vec<Cow<'_, str>> =
                args.iter().map(|a| resolve_place(a, inline_map)).collect();
            if is_stmt_only {
                output.push_str(&format!(
                    "{}{}{}{}{}({});\n",
                    indent,
                    receiver_name,
                    member_op,
                    property,
                    call_op,
                    args_str.join(", ")
                ));
            } else {
                output.push_str(&format!(
                    "{}{}{} = {}{}{}{}({});\n",
                    indent,
                    decl_keyword,
                    lvalue_name,
                    receiver_name,
                    member_op,
                    property,
                    call_op,
                    args_str.join(", ")
                ));
            }
        }
        InstructionValue::PropertyLoad { object, property, optional } => {
            let access_op = if *optional { "?." } else { "." };
            output.push_str(&format!(
                "{}{}{} = {}{}{};\n",
                indent,
                decl_keyword,
                lvalue_name,
                resolve_place(object, inline_map),
                access_op,
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
            } else {
                // Build inline children string to determine if we can fit on one line
                let mut children_inline = String::new();
                for child in &resolved_children {
                    children_inline.push_str(&jsx_child_str(child));
                }
                // Use inline format when the total line length is reasonable,
                // matching upstream's behavior of keeping JSX on a single line.
                let total_len = indent.len() + decl_keyword.len() + lvalue_name.len()
                    + 3 /* " = " */ + 1 /* < */ + tag_name.len() + props_str.len()
                    + 1 /* > */ + children_inline.len() + 2 /* </ */ + tag_name.len()
                    + 2 /* >; */;
                if total_len <= 120 && !children_inline.contains('\n') {
                    // Inline: <Tag props>children</Tag>
                    output.push_str(&format!(
                        "{indent}{decl_keyword}{lvalue_name} = <{tag_name}{props_str}>{children_inline}</{tag_name}>;\n"
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
        }
        InstructionValue::JsxFragment { children } => {
            let resolved_children: Vec<Cow<'_, str>> =
                children.iter().map(|c| resolve_place(c, inline_map)).collect();

            if resolved_children.is_empty() {
                output.push_str(&format!("{indent}{decl_keyword}{lvalue_name} = <></>;\n"));
            } else {
                let mut children_inline = String::new();
                for child in &resolved_children {
                    children_inline.push_str(&jsx_child_str(child));
                }
                let total_len = indent.len() + decl_keyword.len() + lvalue_name.len()
                    + 3 /* " = " */ + 2 /* <> */ + children_inline.len() + 3 /* </>; */;
                if total_len <= 120 && !children_inline.contains('\n') {
                    output.push_str(&format!(
                        "{indent}{decl_keyword}{lvalue_name} = <>{children_inline}</>;\n"
                    ));
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
                                    format_object_key(name),
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
            if is_stmt_only {
                output.push_str(&format!(
                    "{}new {}({});\n",
                    indent,
                    callee_name,
                    args_str.join(", ")
                ));
            } else {
                output.push_str(&format!(
                    "{}{}{} = new {}({});\n",
                    indent,
                    decl_keyword,
                    lvalue_name,
                    callee_name,
                    args_str.join(", ")
                ));
            }
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
            // Emit default value checks for properties with defaults.
            // For `{ x = defaultVal } = obj`, generates:
            //   if (x === undefined) { x = defaultVal; }
            // This matches Babel's approach of `x = t1 === undefined ? default : t1`.
            codegen_destructure_defaults(lvalue_pattern, output, indent, inline_map);
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
        InstructionValue::ComputedLoad { object, property, .. } => {
            let prop_str = resolve_place(property, inline_map);
            if is_dotable_string_literal(&prop_str) {
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = {}.{};\n",
                    resolve_place(object, inline_map),
                    extract_dotable_id(&prop_str)
                ));
            } else {
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = {}[{}];\n",
                    resolve_place(object, inline_map),
                    prop_str
                ));
            }
        }
        InstructionValue::ComputedStore { object, property, value } => {
            let prop_str = resolve_place(property, inline_map);
            if is_dotable_string_literal(&prop_str) {
                output.push_str(&format!(
                    "{indent}{}.{} = {};\n",
                    resolve_place(object, inline_map),
                    extract_dotable_id(&prop_str),
                    resolve_place(value, inline_map)
                ));
            } else {
                output.push_str(&format!(
                    "{indent}{}[{}] = {};\n",
                    resolve_place(object, inline_map),
                    prop_str,
                    resolve_place(value, inline_map)
                ));
            }
        }
        InstructionValue::PropertyDelete { object, property } => {
            output.push_str(&format!(
                "{indent}{decl_keyword}{lvalue_name} = delete {}.{};\n",
                resolve_place(object, inline_map),
                property
            ));
        }
        InstructionValue::ComputedDelete { object, property } => {
            let prop_str = resolve_place(property, inline_map);
            if is_dotable_string_literal(&prop_str) {
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = delete {}.{};\n",
                    resolve_place(object, inline_map),
                    extract_dotable_id(&prop_str)
                ));
            } else {
                output.push_str(&format!(
                    "{indent}{decl_keyword}{lvalue_name} = delete {}[{}];\n",
                    resolve_place(object, inline_map),
                    prop_str
                ));
            }
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
            // Emit init instructions that precede GetIterator (e.g., property loads
            // to compute the collection expression), then use the collection name in
            // the for header. GetIterator/IteratorNext/DeclareLocal/StoreLocal are
            // consumed by the for header syntax.
            emit_for_of_preamble(init, output, indent, declared, tag_constants);
            let (loop_var, collection) = extract_for_of_parts(init);
            output.push_str(&format!("{indent_str}for (const {loop_var} of {collection}) {{\n"));
            codegen_block(test, output, cache_slot, indent + 1, declared, tag_constants);
            codegen_block(body, output, cache_slot, indent + 1, declared, tag_constants);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::ForIn { init, test, body, .. } => {
            emit_for_in_preamble(init, output, indent, declared, tag_constants);
            let (loop_var, collection) = extract_for_in_parts(init);
            output.push_str(&format!("{indent_str}for (const {loop_var} in {collection}) {{\n"));
            codegen_block(test, output, cache_slot, indent + 1, declared, tag_constants);
            codegen_block(body, output, cache_slot, indent + 1, declared, tag_constants);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Try { block, handler, .. } => {
            output.push_str(&format!("{indent_str}try {{\n"));
            codegen_block(block, output, cache_slot, indent + 1, declared, tag_constants);

            // Detect catch param: the HIR builder emits a DeclareLocal as the first
            // instruction in the handler block when the source has `catch (param)`.
            // When absent (bare `catch {}`), there is no DeclareLocal.
            let catch_param_name: Option<String> = handler.instructions.first().and_then(|first| {
                if let ReactiveInstruction::Instruction(instr) = first
                    && let InstructionValue::DeclareLocal { lvalue, .. } = &instr.value
                {
                    return Some(place_name(lvalue).to_string());
                }
                None
            });

            if let Some(ref name) = catch_param_name {
                output.push_str(&format!("{indent_str}}} catch ({name}) {{\n"));
                // Pre-declare so the DeclareLocal instruction becomes a no-op
                // when codegen_block processes the handler body.
                declared.insert(name.clone());
            } else {
                output.push_str(&format!("{indent_str}}} catch {{\n"));
            }
            let handler_start = output.len();
            codegen_block(handler, output, cache_slot, indent + 1, declared, tag_constants);
            // DIVERGENCE: Strip implicit `return undefined;` from catch handler.
            // When the catch handler body is just `return undefined;\n`, upstream
            // emits an empty catch block. This matches JS implicit return behavior.
            let handler_content = &output[handler_start..];
            let handler_trimmed = handler_content.trim();
            if handler_trimmed == "return undefined;" {
                output.truncate(handler_start);
            }
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Label { block, label, .. } => {
            output.push_str(&format!("{indent_str}bb{label}: {{\n"));
            codegen_block(block, output, cache_slot, indent + 1, declared, tag_constants);
            output.push_str(&format!("{indent_str}}}\n"));
        }
        ReactiveTerminal::Logical { operator, right, result, .. } => {
            // The left-side instructions (including `result = left_val`) were
            // already emitted. Wrap the right block in a conditional that
            // preserves short-circuit semantics.
            if let Some(result_place) = result {
                let result_name = resolve_place(result_place, inline_map);
                let condition = match operator {
                    crate::hir::types::LogicalOp::And => format!("{result_name}"),
                    crate::hir::types::LogicalOp::Or => format!("!{result_name}"),
                    crate::hir::types::LogicalOp::NullishCoalescing => {
                        format!("{result_name} == null")
                    }
                };
                output.push_str(&format!("{indent_str}if ({condition}) {{\n"));
                codegen_block(right, output, cache_slot, indent + 1, declared, tag_constants);
                output.push_str(&format!("{indent_str}}}\n"));
            } else {
                // No result place — just emit the right block unconditionally
                // (fallback, should not happen in practice)
                codegen_block(right, output, cache_slot, indent, declared, tag_constants);
            }
        }
        ReactiveTerminal::Continue { .. } => {
            output.push_str(&format!("{indent_str}continue;\n"));
        }
        ReactiveTerminal::Break { .. } => {
            output.push_str(&format!("{indent_str}break;\n"));
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
    scope_output_promotions: &FxHashMap<IdentifierId, String>,
) {
    let indent_str = "  ".repeat(indent);
    let deps = &scope.scope.dependencies;
    let slot_start = *cache_slot;

    // Sort declarations by source location for deterministic cache slot ordering.
    // Upstream's CodegenReactiveFunction uses compareScopeDeclaration() for this.
    let mut sorted_decls: Vec<_> = scope.scope.declarations.iter().collect();
    sorted_decls.sort_by_key(|(_, decl)| decl.identifier.loc.start);
    // Identify DeclareLocal temps that correspond to destructured variables.
    // These should be excluded from scope declarations and DeclareLocal hoisting.
    // DIVERGENCE: upstream never places hook destructures in scope bodies.
    // We fix this at codegen by:
    //   (a) hoisting the Destructure before the scope guard
    //   (b) pre-declaring pattern names so the Destructure emits bare assignment
    //   (c) removing DeclareLocal-based scope declarations for destructured vars
    //   (d) adding the actual destructured names as replacement scope declarations
    let destructured_declare_ids = find_destructured_declare_ids(scope);

    // Hoist DeclareLocal instructions for scope DECLARATIONS only.
    // Variables that are scope outputs (stored in cache, loaded in else branch)
    // need `let` declarations before the scope guard. Variables that are only
    // used inside the scope body should remain as `const` inside the if-block.
    // Skip DeclareLocals whose lvalue IDs are in destructured_declare_ids.
    let scope_decl_ids: FxHashSet<IdentifierId> =
        scope.scope.declarations.iter().map(|(id, _)| *id).collect();
    let empty_inline_map = InlineMap::default();
    for instr in &scope.instructions.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr
            && let InstructionValue::DeclareLocal { lvalue, .. }
            | InstructionValue::DeclareContext { lvalue, .. } = &instruction.value
        {
            // Skip DeclareLocal for destructured variables
            if destructured_declare_ids.contains(&instruction.lvalue.identifier.id) {
                continue;
            }
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
    let mut hoisted_destructure_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut destructure_pattern_names: Vec<String> = Vec::new();
    for instr in &scope.instructions.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr
            && let InstructionValue::Destructure { lvalue_pattern, .. } = &instruction.value
        {
            // Pre-declare the destructured names
            collect_pattern_names_with_declarations(lvalue_pattern, output, declared, &indent_str);
            // Emit the destructure before the scope guard
            codegen_instruction(
                instruction,
                output,
                &indent_str,
                declared,
                &empty_inline_map,
                tag_constants,
            );
            // Collect the pattern names as replacement declarations
            let mut names = Vec::new();
            collect_pattern_name_strings(lvalue_pattern, &mut names);
            destructure_pattern_names.extend(names);
            // Track this Destructure instruction's lvalue for skipping
            hoisted_destructure_ids.insert(instruction.lvalue.identifier.id);
        }
    }

    // Filter out scope declarations that correspond to DeclareLocal temps for
    // destructured variables (they're replaced by the actual pattern names).
    //
    // DIVERGENCE: Also filter out scope declarations whose variable is only declared
    // (via DeclareLocal) but never assigned a value within this scope body.
    // This prevents caching undefined for variables whose actual value comes from
    // a hook call (useMemo, useCallback, useRef) that runs AFTER this scope.
    // Pattern: ScopeA wraps callback+deps creation for useMemo, but also contains
    // a DeclareLocal for the NEXT useMemo's result variable. Without this filter,
    // the scope caches the uninitialized variable, causing "Cannot read properties
    // of undefined" errors at runtime.
    let value_producing_names = collect_value_producing_names(&scope.instructions);
    let sorted_decls: Vec<_> = sorted_decls
        .into_iter()
        .filter(|(id, _)| !destructured_declare_ids.contains(id))
        .filter(|(_, decl)| {
            let decl_name = identifier_display_name(&decl.identifier);
            value_producing_names.contains(decl_name.as_ref())
        })
        .collect();

    // Pre-declare scope output variables with `let` so the else-branch
    // (cache reload) can assign to them. Variables already declared by
    // DeclareLocal above are skipped.
    // When scope output promotions are active, use the promoted name instead
    // of the temp name (e.g., `let x;` instead of `let t7;`).
    for (id, decl) in &sorted_decls {
        let decl_name = effective_decl_display_name(*id, decl, scope_output_promotions);
        if !declared.contains(&decl_name) {
            declared.insert(decl_name.clone());
            output.push_str(&format!("{indent_str}let {decl_name};\n"));
        }
    }
    // Pre-declare the early return variable if it exists and isn't already declared
    if let Some(ref erv) = scope.scope.early_return_value {
        let ern = identifier_display_name(&erv.value.identifier);
        if !declared.contains(ern.as_ref()) {
            declared.insert(ern.to_string());
            output.push_str(&format!("{indent_str}let {ern};\n"));
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
    // Merge the hoisted Destructure and its DeclareLocal IDs for skipping
    let mut skip_ids = hoisted_destructure_ids.clone();
    skip_ids.extend(destructured_declare_ids.iter());

    // When the scope has an early return, we need to convert the trailing
    // `return X;` into `early_return_var = X;` so control flow continues
    // to the cache store instead of exiting the function.
    let early_return_name = scope
        .scope
        .early_return_value
        .as_ref()
        .map(|erv| identifier_display_name(&erv.value.identifier).to_string());

    // Build a temp→promoted name replacement map for scope body output.
    // When a scope output temp is promoted to a user variable name, the scope
    // body should use the promoted name (e.g., `x = [1, 2, 3]` instead of
    // `t7 = [1, 2, 3]`).
    let scope_body_replacements: Vec<(String, String)> = scope
        .scope
        .declarations
        .iter()
        .filter_map(|(id, decl)| {
            scope_output_promotions.get(id).map(|promoted_name| {
                let temp_name = identifier_display_name(&decl.identifier).to_string();
                (temp_name, promoted_name.clone())
            })
        })
        .collect();

    let body_start = output.len();
    if let Some(ref ern) = early_return_name {
        // Codegen the scope body into a temporary buffer
        codegen_block_skip_declares_and_ids(
            &scope.instructions,
            output,
            cache_slot,
            indent + 1,
            declared,
            &scope_decl_names,
            tag_constants,
            &skip_ids,
        );
        // Replace the last `return ...;` with `ern = ...;` in the generated body
        let body = output[body_start..].to_string();
        if let Some(return_pos) = body.rfind("return ") {
            // Find the semicolon that ends this return statement
            if let Some(semi_pos) = body[return_pos..].find(";\n") {
                let return_value = &body[return_pos + 7..return_pos + semi_pos];
                let replacement = format!("{ern} = {return_value}");
                let new_body = format!(
                    "{}{}{}",
                    &body[..return_pos],
                    replacement,
                    &body[return_pos + semi_pos..]
                );
                output.truncate(body_start);
                output.push_str(&new_body);
            }
        }
    } else {
        codegen_block_skip_declares_and_ids(
            &scope.instructions,
            output,
            cache_slot,
            indent + 1,
            declared,
            &scope_decl_names,
            tag_constants,
            &skip_ids,
        );
    }

    // Apply scope body replacements: replace temp names with promoted names
    // in the scope body output. This transforms e.g. `t7 = [1, 2, 3]` to
    // `x = [1, 2, 3]` when `t7` has been promoted to `x`.
    if !scope_body_replacements.is_empty() {
        let body = output[body_start..].to_string();
        let mut replaced = body;
        for (temp_name, promoted_name) in &scope_body_replacements {
            // Replace at word boundaries to avoid replacing `t7` inside `t70`
            // We use a simple approach: replace `temp_name` when preceded/followed
            // by non-alphanumeric characters. Since temp names are `tN`, this is safe.
            replaced = replace_identifier_in_output(&replaced, temp_name, promoted_name);
        }
        output.truncate(body_start);
        output.push_str(&replaced);
    }

    // Build the effective list of declaration names for cache store/load.
    // This includes the original (non-phantom) declarations plus any
    // destructured names that replace phantom temps.
    let mut effective_decl_names: Vec<String> = sorted_decls
        .iter()
        .map(|(id, decl)| effective_decl_display_name(*id, decl, scope_output_promotions))
        .collect();
    effective_decl_names.extend(destructure_pattern_names.iter().cloned());
    // Add early return value to effective declarations so it gets cached/reloaded
    if let Some(ref ern) = early_return_name
        && !effective_decl_names.contains(ern)
    {
        effective_decl_names.push(ern.clone());
    }
    let effective_decl_count = effective_decl_names.len() as u32;

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
        for (i, decl_name) in effective_decl_names.iter().enumerate() {
            output.push_str(&format!(
                "{}$[{}] = {};\n",
                inner_indent,
                slot_start + i as u32,
                decl_name
            ));
        }
        // Advance cache_slot past the declarations (sentinel slot is included)
        *cache_slot = slot_start + effective_decl_count.max(1);
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
        for (i, decl_name) in effective_decl_names.iter().enumerate() {
            output.push_str(&format!(
                "{}$[{}] = {};\n",
                inner_indent,
                decl_slot_start + i as u32,
                decl_name
            ));
        }
        *cache_slot = decl_slot_start + effective_decl_count;
    }

    // Compute the declaration slot start for the else-branch reload
    let decl_reload_start =
        if deps.is_empty() { slot_start } else { slot_start + deps.len() as u32 };

    // Only emit else block if there are declarations to load from cache
    if !effective_decl_names.is_empty() {
        output.push_str(&format!("{indent_str}}} else {{\n"));

        // Load cached declarations
        for (i, decl_name) in effective_decl_names.iter().enumerate() {
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

    // If this scope has an early return value, emit `return <value>;` after
    // the scope guard. The return value should have been cached as a scope
    // declaration, so it's available in both the if-branch (fresh) and
    // else-branch (cached) paths.
    if let Some(ref early_return) = scope.scope.early_return_value {
        let return_name = identifier_display_name(&early_return.value.identifier);
        output.push_str(&format!("{indent_str}return {return_name};\n"));
    }
}

/// Find scope declaration IDs that should be replaced when a scope contains
/// a Destructure instruction. These are DeclareLocal instructions whose
/// target variable is also a target of a Destructure pattern in the same scope.
///
/// Matching is done by declaration_id (linking SSA versions of the same
/// source variable) or by name when declaration_id is not available.
fn find_destructured_declare_ids(scope: &ReactiveScopeBlock) -> FxHashSet<IdentifierId> {
    // Collect declaration_ids and names from all Destructure patterns
    let mut destructure_decl_ids: FxHashSet<DeclarationId> = FxHashSet::default();
    let mut destructure_names: FxHashSet<String> = FxHashSet::default();
    for si in &scope.instructions.instructions {
        if let ReactiveInstruction::Instruction(instruction) = si
            && let InstructionValue::Destructure { lvalue_pattern, .. } = &instruction.value
        {
            collect_pattern_identifiers(
                lvalue_pattern,
                &mut destructure_decl_ids,
                &mut destructure_names,
            );
        }
    }
    if destructure_decl_ids.is_empty() && destructure_names.is_empty() {
        return FxHashSet::default();
    }

    // Find DeclareLocal instructions whose target matches a Destructure pattern variable
    let mut result = FxHashSet::default();
    for si in &scope.instructions.instructions {
        if let ReactiveInstruction::Instruction(instruction) = si
            && let InstructionValue::DeclareLocal { lvalue, .. } = &instruction.value
        {
            let matches = lvalue
                .identifier
                .declaration_id
                .is_some_and(|did| destructure_decl_ids.contains(&did))
                || lvalue.identifier.name.as_ref().is_some_and(|n| destructure_names.contains(n));
            if matches {
                result.insert(instruction.lvalue.identifier.id);
            }
        }
    }
    result
}

/// Collect the set of identifier IDs and display names that have value-producing
/// instructions (not just DeclareLocal/DeclareContext) within a scope body.
/// A "value-producing" instruction is one that computes or stores a value
/// (StoreLocal, FunctionExpression, ArrayExpression, CallExpression, etc.),
/// as opposed to DeclareLocal which merely declares a variable without assigning.
///
/// This is used to filter scope declarations: a variable that is only DeclareLocal'd
/// inside a scope but never assigned should NOT be cached as a scope output, because
/// its value is undefined at cache time. The actual assignment happens later (e.g.,
/// after a useMemo/useCallback call that follows the scope).
fn collect_value_producing_names(block: &ReactiveBlock) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    for instr in &block.instructions {
        match instr {
            ReactiveInstruction::Instruction(instruction) => {
                match &instruction.value {
                    // DeclareLocal/DeclareContext only declare — they don't produce values
                    InstructionValue::DeclareLocal { .. }
                    | InstructionValue::DeclareContext { .. } => {}
                    // StoreLocal/StoreContext produce values for the TARGET variable
                    InstructionValue::StoreLocal { lvalue, .. }
                    | InstructionValue::StoreContext { lvalue, .. } => {
                        // The store target is value-produced
                        let target_name = identifier_display_name(&lvalue.identifier);
                        names.insert(target_name.to_string());
                        // The instruction's own lvalue (temp) is also value-produced
                        let lvalue_name = identifier_display_name(&instruction.lvalue.identifier);
                        names.insert(lvalue_name.to_string());
                    }
                    // All other instructions produce a value in their lvalue
                    _ => {
                        let lvalue_name = identifier_display_name(&instruction.lvalue.identifier);
                        names.insert(lvalue_name.to_string());
                    }
                }
            }
            ReactiveInstruction::Scope(_) | ReactiveInstruction::Terminal(_) => {
                // Nested scopes and terminals also produce values but we don't
                // recurse — scope declarations reference top-level names.
            }
        }
    }
    names
}

/// Collect declaration_ids and names from all places in a Destructure pattern.
fn collect_pattern_identifiers(
    pattern: &crate::hir::types::DestructurePattern,
    decl_ids: &mut FxHashSet<DeclarationId>,
    names: &mut FxHashSet<String>,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};
    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                match &prop.value {
                    DestructureTarget::Place(place) => {
                        if let Some(did) = place.identifier.declaration_id {
                            decl_ids.insert(did);
                        }
                        if let Some(name) = &place.identifier.name {
                            names.insert(name.clone());
                        }
                    }
                    DestructureTarget::Pattern(nested) => {
                        collect_pattern_identifiers(nested, decl_ids, names);
                    }
                }
            }
            if let Some(rest_place) = rest {
                if let Some(did) = rest_place.identifier.declaration_id {
                    decl_ids.insert(did);
                }
                if let Some(name) = &rest_place.identifier.name {
                    names.insert(name.clone());
                }
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(target) => match target {
                        DestructureTarget::Place(place) => {
                            if let Some(did) = place.identifier.declaration_id {
                                decl_ids.insert(did);
                            }
                            if let Some(name) = &place.identifier.name {
                                names.insert(name.clone());
                            }
                        }
                        DestructureTarget::Pattern(nested) => {
                            collect_pattern_identifiers(nested, decl_ids, names);
                        }
                    },
                    DestructureArrayItem::Hole | DestructureArrayItem::Spread(_) => {}
                }
            }
            if let Some(rest_place) = rest {
                if let Some(did) = rest_place.identifier.declaration_id {
                    decl_ids.insert(did);
                }
                if let Some(name) = &rest_place.identifier.name {
                    names.insert(name.clone());
                }
            }
        }
    }
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

    // Strip implicit `return undefined;` added by the HIR builder for functions
    // with no explicit return value. Upstream omits this trailing statement,
    // relying on JavaScript's implicit undefined return for function expressions.
    // DIVERGENCE: Upstream distinguishes implicit vs explicit return in the AST;
    // we strip the trailing pattern as a post-processing step.
    let trailing = format!("{}return undefined;\n", "  ".repeat(indent));
    if output.ends_with(&trailing) {
        output.truncate(output.len() - trailing.len());
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
                // Count effective declarations: original declarations minus
                // DeclareLocal temps for destructured vars, plus their replacement names.
                let destructured_declare_ids = find_destructured_declare_ids(scope);
                let mut replacement_name_count: u32 = 0;
                for si in &scope.instructions.instructions {
                    if let ReactiveInstruction::Instruction(instruction) = si
                        && let InstructionValue::Destructure { lvalue_pattern, .. } =
                            &instruction.value
                    {
                        let mut names = Vec::new();
                        collect_pattern_name_strings(lvalue_pattern, &mut names);
                        replacement_name_count += names.len() as u32;
                    }
                }
                let non_phantom_decls = scope
                    .scope
                    .declarations
                    .iter()
                    .filter(|(id, _)| !destructured_declare_ids.contains(id))
                    .count() as u32;
                let effective_decls = non_phantom_decls + replacement_name_count;

                if deps.is_empty() {
                    // Sentinel scope: the sentinel check reuses the first
                    // declaration's slot. Total = max(declarations, 1).
                    count += effective_decls.max(1);
                } else {
                    // Reactive scope: deps + declarations as separate slots
                    count += deps.len() as u32 + effective_decls;
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
        ReactiveTerminal::Logical { right, .. } => count_cache_slots(right),
        ReactiveTerminal::Return { .. }
        | ReactiveTerminal::Throw { .. }
        | ReactiveTerminal::Continue { .. }
        | ReactiveTerminal::Break { .. } => 0,
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
///
/// When `gating` is provided, adds the gating function import alongside the
/// compiler-runtime import. The per-function gating ternaries should already
/// be applied in the `compiled_functions` entries by the caller.
pub fn apply_compilation(
    original_source: &str,
    compiled_functions: &[(oxc_span::Span, String)],
    gating: Option<&crate::entrypoint::options::GatingConfig>,
) -> String {
    if compiled_functions.is_empty() {
        return original_source.to_string();
    }

    let mut result = String::with_capacity(original_source.len() + 256);

    // Add compiler-runtime import at the top (JSX syntax is preserved, no jsx-runtime needed)
    result.push_str(&generate_import_statement());

    // Add gating function import if configured
    if let Some(gating) = gating {
        result.push_str(&gating.generate_import());
    }

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

    // When gating is active, strip the gating directive comments from the
    // source. The directives have been consumed (the gating import and
    // per-function ternaries are already applied), so they should not appear
    // in the output. Upstream's Babel plugin removes these annotations during
    // compilation; we do it here since our approach is source-edit-based.
    if gating.is_some() {
        let had_trailing_newline = source.ends_with('\n');
        source = source
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("// @gating") && !trimmed.starts_with("// @dynamicGating")
            })
            .collect::<Vec<_>>()
            .join("\n");
        if had_trailing_newline {
            source.push('\n');
        }
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
// ---------------------------------------------------------------------------
// ForOf / ForIn helpers
// ---------------------------------------------------------------------------

/// Emit init block instructions that precede the iterator protocol for a `ForOf` loop.
/// Instructions before `GetIterator` (e.g., property loads to compute the collection)
/// need to be emitted before the `for` header.
fn emit_for_of_preamble(
    init: &ReactiveBlock,
    output: &mut String,
    indent: usize,
    declared: &mut FxHashSet<String>,
    tag_constants: &TagConstantMap,
) {
    let indent_str = "  ".repeat(indent);
    // Use empty inline map: preamble instructions must emit all temps explicitly
    // because the for-header consumes GetIterator/IteratorNext without codegen.
    let inline_map = InlineMap::default();
    for instr in &init.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr {
            match &instruction.value {
                // Stop at GetIterator — everything from here is consumed by the for header
                InstructionValue::GetIterator { .. } => break,
                // Emit preceding instructions (e.g., property loads for the collection)
                _ => {
                    codegen_instruction(
                        instruction,
                        output,
                        &indent_str,
                        declared,
                        &inline_map,
                        tag_constants,
                    );
                }
            }
        }
    }
}

/// Emit init block instructions that precede the iterator protocol for a `ForIn` loop.
fn emit_for_in_preamble(
    init: &ReactiveBlock,
    output: &mut String,
    indent: usize,
    declared: &mut FxHashSet<String>,
    tag_constants: &TagConstantMap,
) {
    let indent_str = "  ".repeat(indent);
    let inline_map = InlineMap::default();
    for instr in &init.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr {
            match &instruction.value {
                // Stop at NextPropertyOf — everything from here is consumed by the for header
                InstructionValue::NextPropertyOf { .. } => break,
                _ => {
                    codegen_instruction(
                        instruction,
                        output,
                        &indent_str,
                        declared,
                        &inline_map,
                        tag_constants,
                    );
                }
            }
        }
    }
}

/// Extract the loop variable name and collection expression from a `ForIn` init block.
/// The init block contains `NextPropertyOf { value: collection }` followed by
/// `StoreLocal/DeclareLocal { lvalue: loop_var }`.
fn extract_for_in_parts(init: &ReactiveBlock) -> (String, String) {
    let mut collection_name = "_".to_string();
    let mut loop_var_name = "_".to_string();
    let mut collection_id = None;

    for instr in &init.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr {
            match &instruction.value {
                InstructionValue::NextPropertyOf { value } => {
                    let name = place_name(value);
                    if name.starts_with('t') && name[1..].chars().all(|c| c.is_ascii_digit()) {
                        collection_id = Some(value.identifier.id);
                    } else {
                        collection_name = name.into_owned();
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

    // Resolve temp collection name by finding the instruction that produced it
    if collection_name == "_"
        && let Some(cid) = collection_id
    {
        for instr in &init.instructions {
            if let ReactiveInstruction::Instruction(instruction) = instr
                && instruction.lvalue.identifier.id == cid
            {
                match &instruction.value {
                    InstructionValue::LoadLocal { place, .. }
                    | InstructionValue::LoadContext { place, .. } => {
                        collection_name = place_name(place).into_owned();
                    }
                    _ => {
                        collection_name = place_name(&instruction.lvalue).into_owned();
                    }
                }
                break;
            }
        }
    }

    (loop_var_name, collection_name)
}

/// Extract the loop variable name and collection expression from a `ForOf` init block.
/// The init block contains `GetIterator { collection }`, `IteratorNext { iterator }`,
/// then `DeclareLocal/StoreLocal { lvalue: loop_var }`. When the collection is a temp
/// (no name), we resolve it by looking for the LoadLocal/LoadContext that produced it.
fn extract_for_of_parts(init: &ReactiveBlock) -> (String, String) {
    let mut collection_name = "_".to_string();
    let mut loop_var_name = "_".to_string();
    // Track the GetIterator's collection place ID for temp resolution
    let mut collection_id = None;

    for instr in &init.instructions {
        if let ReactiveInstruction::Instruction(instruction) = instr {
            match &instruction.value {
                InstructionValue::GetIterator { collection } => {
                    let name = place_name(collection);
                    if name.starts_with('t') && name[1..].chars().all(|c| c.is_ascii_digit()) {
                        // Temp name — record ID for resolution
                        collection_id = Some(collection.identifier.id);
                    } else {
                        collection_name = name.into_owned();
                    }
                }
                InstructionValue::IteratorNext { iterator, .. } => {
                    // If we haven't found the collection via GetIterator yet,
                    // try to use the iterator's place name
                    if collection_name == "_" && collection_id.is_none() {
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

    // Resolve temp collection name by finding the instruction that produced it
    if collection_name == "_"
        && let Some(cid) = collection_id
    {
        for instr in &init.instructions {
            if let ReactiveInstruction::Instruction(instruction) = instr
                && instruction.lvalue.identifier.id == cid
            {
                match &instruction.value {
                    InstructionValue::LoadLocal { place, .. }
                    | InstructionValue::LoadContext { place, .. } => {
                        collection_name = place_name(place).into_owned();
                    }
                    _ => {
                        collection_name = place_name(&instruction.lvalue).into_owned();
                    }
                }
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

/// Get the effective display name for a scope declaration, applying scope
/// output promotions if available.
fn effective_decl_display_name(
    id: IdentifierId,
    decl: &crate::hir::types::ReactiveScopeDeclaration,
    promotions: &FxHashMap<IdentifierId, String>,
) -> String {
    if let Some(promoted) = promotions.get(&id) {
        promoted.clone()
    } else {
        identifier_display_name(&decl.identifier).to_string()
    }
}

/// Replace an identifier name in codegen output, respecting word boundaries.
///
/// Replaces occurrences of `old_name` with `new_name` only when `old_name`
/// appears as a standalone identifier (not part of a longer identifier).
/// This prevents replacing `t7` inside `t70` or `t7x`.
///
/// Word boundary characters include alphanumeric, `_`, and `$` (JS identifier chars).
fn replace_identifier_in_output(output: &str, old_name: &str, new_name: &str) -> String {
    if old_name.is_empty() || old_name == new_name {
        return output.to_string();
    }

    fn is_ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_' || c == '$'
    }

    let mut result = String::with_capacity(output.len());
    let mut search_start = 0;
    while let Some(pos) = output[search_start..].find(old_name) {
        let abs_pos = search_start + pos;
        // Check character before: must not be an identifier char
        let before_ok =
            abs_pos == 0 || !is_ident_char(output[..abs_pos].chars().next_back().unwrap_or(' '));
        // Check character after: must not be an identifier char
        let after_pos = abs_pos + old_name.len();
        let after_ok = after_pos >= output.len()
            || !is_ident_char(output[after_pos..].chars().next().unwrap_or(' '));

        if before_ok && after_ok {
            result.push_str(&output[search_start..abs_pos]);
            result.push_str(new_name);
            search_start = after_pos;
        } else {
            // Not a word boundary match — copy up to just past the match start
            // and continue searching after that
            result.push_str(&output[search_start..abs_pos + old_name.len()]);
            search_start = abs_pos + old_name.len();
        }
    }
    result.push_str(&output[search_start..]);
    result
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

/// Collect all variable names from a destructure pattern, emit `let name;`
/// declarations for any not yet declared, and add them to the declared set.
/// This is used to hoist destructure pattern names out of scope bodies so
/// the destructure inside the scope guard emits bare assignment instead of
/// block-scoped `let`.
fn collect_pattern_names_with_declarations(
    pattern: &crate::hir::types::DestructurePattern,
    output: &mut String,
    declared: &mut FxHashSet<String>,
    indent: &str,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};
    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                match &prop.value {
                    DestructureTarget::Place(place) => {
                        let name = place_name(place);
                        if !declared.contains(name.as_ref()) {
                            declared.insert(name.to_string());
                            output.push_str(&format!("{indent}let {name};\n"));
                        }
                    }
                    DestructureTarget::Pattern(nested) => {
                        collect_pattern_names_with_declarations(nested, output, declared, indent);
                    }
                }
            }
            if let Some(rest_place) = rest {
                let name = place_name(rest_place);
                if !declared.contains(name.as_ref()) {
                    declared.insert(name.to_string());
                    output.push_str(&format!("{indent}let {name};\n"));
                }
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(target) => match target {
                        DestructureTarget::Place(place) => {
                            let name = place_name(place);
                            if !declared.contains(name.as_ref()) {
                                declared.insert(name.to_string());
                                output.push_str(&format!("{indent}let {name};\n"));
                            }
                        }
                        DestructureTarget::Pattern(nested) => {
                            collect_pattern_names_with_declarations(
                                nested, output, declared, indent,
                            );
                        }
                    },
                    DestructureArrayItem::Hole | DestructureArrayItem::Spread(_) => {}
                }
            }
            if let Some(rest_place) = rest {
                let name = place_name(rest_place);
                if !declared.contains(name.as_ref()) {
                    declared.insert(name.to_string());
                    output.push_str(&format!("{indent}let {name};\n"));
                }
            }
        }
    }
}

/// Collect all variable names from a destructure pattern into a Vec of strings.
fn collect_pattern_name_strings(
    pattern: &crate::hir::types::DestructurePattern,
    names: &mut Vec<String>,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern, DestructureTarget};
    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                match &prop.value {
                    DestructureTarget::Place(place) => {
                        names.push(place_name(place).to_string());
                    }
                    DestructureTarget::Pattern(nested) => {
                        collect_pattern_name_strings(nested, names);
                    }
                }
            }
            if let Some(rest_place) = rest {
                names.push(place_name(rest_place).to_string());
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(target) => match target {
                        DestructureTarget::Place(place) => {
                            names.push(place_name(place).to_string());
                        }
                        DestructureTarget::Pattern(nested) => {
                            collect_pattern_name_strings(nested, names);
                        }
                    },
                    DestructureArrayItem::Hole | DestructureArrayItem::Spread(_) => {}
                }
            }
            if let Some(rest_place) = rest {
                names.push(place_name(rest_place).to_string());
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

/// Build inline map entries for default value temps in a destructure pattern.
/// Finds the instructions in the block that produce the default values and
/// generates their expression strings for inlining.
fn collect_default_value_inline_entries(
    pattern: &crate::hir::types::DestructurePattern,
    instructions: &[ReactiveInstruction],
    inline_map: &mut InlineMap,
    tag_constants: &TagConstantMap,
) {
    // Collect all default value temp IDs from the pattern
    let mut default_temp_ids: FxHashSet<String> = FxHashSet::default();
    collect_default_temp_names(pattern, &mut default_temp_ids);

    if default_temp_ids.is_empty() {
        return;
    }

    // Find instructions that produce these temps and generate inline expressions
    for ri in instructions {
        if let ReactiveInstruction::Instruction(instr) = ri {
            let temp_name = format!("t{}", instr.lvalue.identifier.id.0);
            if default_temp_ids.contains(&temp_name)
                && let Some(expr) = expr_string(&instr.value, inline_map, tag_constants)
            {
                inline_map.insert(temp_name, expr);
            }
        }
    }
}

/// Collect temp names from default value places in a destructure pattern.
fn collect_default_temp_names(
    pattern: &crate::hir::types::DestructurePattern,
    names: &mut FxHashSet<String>,
) {
    use crate::hir::types::DestructurePattern;
    match pattern {
        DestructurePattern::Object { properties, .. } => {
            for prop in properties {
                if let Some(ref default_place) = prop.default_value
                    && is_temp_place(default_place)
                {
                    names.insert(format!("t{}", default_place.identifier.id.0));
                }
                if let crate::hir::types::DestructureTarget::Pattern(nested) = &prop.value {
                    collect_default_temp_names(nested, names);
                }
            }
        }
        DestructurePattern::Array { items, .. } => {
            for item in items {
                if let crate::hir::types::DestructureArrayItem::Value(
                    crate::hir::types::DestructureTarget::Pattern(nested),
                ) = item
                {
                    collect_default_temp_names(nested, names);
                }
            }
        }
    }
}

/// Emit default value checks for destructure properties that have defaults.
/// For `{ x = defaultVal } = obj`, generates:
///   if (x === undefined) { x = defaultVal; }
///
/// This is called after the destructure statement is emitted.
fn codegen_destructure_defaults(
    pattern: &crate::hir::types::DestructurePattern,
    output: &mut String,
    indent: &str,
    inline_map: &InlineMap,
) {
    match pattern {
        crate::hir::types::DestructurePattern::Object { properties, .. } => {
            for prop in properties {
                if let Some(ref default_place) = prop.default_value
                    && let crate::hir::types::DestructureTarget::Place(target) = &prop.value
                {
                    let var_name = place_name(target);
                    let default_val = resolve_place(default_place, inline_map);
                    output.push_str(&format!(
                        "{indent}if ({var_name} === undefined) {{ {var_name} = {default_val}; }}\n"
                    ));
                }
            }
        }
        crate::hir::types::DestructurePattern::Array { .. } => {
            // Array destructuring defaults not yet supported
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
    let async_prefix = if rf.is_async { "async " } else { "" };
    let generator_star = if rf.is_generator { "*" } else { "" };
    if rf.is_arrow {
        ctx.write(&format!("{async_prefix}("));
    } else if let Some(ref name) = rf.id {
        ctx.write(&format!("{async_prefix}function{generator_star} {name}("));
    } else {
        ctx.write(&format!("{async_prefix}function{generator_star} ("));
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
                let empty_promotions = FxHashMap::default();
                codegen_scope(
                    scope_block,
                    &mut ctx.output,
                    cache_slot,
                    indent,
                    &mut declared,
                    tag_constants,
                    &empty_promotions,
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
