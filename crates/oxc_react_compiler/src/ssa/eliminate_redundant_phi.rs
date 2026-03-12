use crate::hir::types::{HIR, IdentifierId, Phi, Place};
use rustc_hash::FxHashMap;

// DIVERGENCE: Upstream does not have a separate redundant-phi elimination pass;
// its SSA construction avoids trivial phis at insertion time. Our EnterSSA
// uses the standard Cytron-et-al algorithm which may insert more phis than
// necessary, so we run this cleanup pass immediately after to remove them.
/// Remove phi nodes that are trivially redundant.
///
/// A phi is redundant if:
/// - All operands are the same value (or the phi itself)
/// - `phi(x, x, x)` -> replace with `x`
/// - `phi(x, phi_self)` -> replace with `x`
///
/// This is iterative: removing one phi may make others redundant.
pub fn eliminate_redundant_phi(hir: &mut HIR) {
    let mut changed = true;
    while changed {
        changed = false;
        let mut replacements: FxHashMap<IdentifierId, IdentifierId> = FxHashMap::default();

        // Find redundant phis
        for (_, block) in &hir.blocks {
            for phi in &block.phis {
                if let Some(replacement) = is_redundant_phi(phi) {
                    replacements.insert(phi.place.identifier.id, replacement);
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }

        // Apply replacements: update all identifier references
        apply_replacements(hir, &replacements);

        // Remove the redundant phi nodes
        for (_, block) in &mut hir.blocks {
            block.phis.retain(|phi| !replacements.contains_key(&phi.place.identifier.id));
        }
    }
}

/// Check if a phi node is redundant. Returns the replacement IdentifierId if so.
fn is_redundant_phi(phi: &Phi) -> Option<IdentifierId> {
    let phi_id = phi.place.identifier.id;
    let mut unique_operand: Option<IdentifierId> = None;

    for (_, operand_place) in &phi.operands {
        let op_id = operand_place.identifier.id;
        // Skip self-references
        if op_id == phi_id {
            continue;
        }
        match unique_operand {
            None => unique_operand = Some(op_id),
            Some(existing) => {
                if existing != op_id {
                    // Multiple distinct operands - not redundant
                    return None;
                }
            }
        }
    }

    // If we found exactly one unique operand (or all were self-references with one real value)
    unique_operand
}

/// Apply identifier replacements throughout the HIR
fn apply_replacements(hir: &mut HIR, replacements: &FxHashMap<IdentifierId, IdentifierId>) {
    if replacements.is_empty() {
        return;
    }

    // Resolve transitive replacements
    let resolved = resolve_transitive(replacements);

    for (_, block) in &mut hir.blocks {
        // Update instructions
        for instr in &mut block.instructions {
            replace_in_place(&mut instr.lvalue, &resolved);
            replace_in_instruction_value(&mut instr.value, &resolved);
        }
        // Update phi operands
        for phi in &mut block.phis {
            for (_, operand) in &mut phi.operands {
                replace_in_place(operand, &resolved);
            }
        }
        // Update terminal
        replace_in_terminal(&mut block.terminal, &resolved);
    }
}

/// Resolve transitive replacements: if a -> b -> c, then a -> c
fn resolve_transitive(
    replacements: &FxHashMap<IdentifierId, IdentifierId>,
) -> FxHashMap<IdentifierId, IdentifierId> {
    let mut resolved = replacements.clone();
    let mut changed = true;
    while changed {
        changed = false;
        let snapshot: Vec<(IdentifierId, IdentifierId)> =
            resolved.iter().map(|(&k, &v)| (k, v)).collect();
        for (key, value) in snapshot {
            if let Some(&further) = resolved.get(&value)
                && further != value
            {
                resolved.insert(key, further);
                changed = true;
            }
        }
    }
    resolved
}

fn replace_in_place(place: &mut Place, replacements: &FxHashMap<IdentifierId, IdentifierId>) {
    if let Some(&replacement) = replacements.get(&place.identifier.id) {
        place.identifier.id = replacement;
    }
}

fn replace_in_instruction_value(
    value: &mut crate::hir::types::InstructionValue,
    replacements: &FxHashMap<IdentifierId, IdentifierId>,
) {
    use crate::hir::types::InstructionValue;
    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            replace_in_place(place, replacements);
        }
        InstructionValue::StoreLocal { lvalue, value, .. } => {
            replace_in_place(lvalue, replacements);
            replace_in_place(value, replacements);
        }
        InstructionValue::StoreContext { lvalue, value } => {
            replace_in_place(lvalue, replacements);
            replace_in_place(value, replacements);
        }
        InstructionValue::DeclareLocal { lvalue, .. } => {
            replace_in_place(lvalue, replacements);
        }
        InstructionValue::DeclareContext { lvalue } => {
            replace_in_place(lvalue, replacements);
        }
        InstructionValue::Destructure { lvalue_pattern, value } => {
            replace_in_place(value, replacements);
            replace_in_destructure_pattern(lvalue_pattern, replacements);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            replace_in_place(left, replacements);
            replace_in_place(right, replacements);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            replace_in_place(value, replacements);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            replace_in_place(lvalue, replacements);
        }
        InstructionValue::CallExpression { callee, args } => {
            replace_in_place(callee, replacements);
            for arg in args {
                replace_in_place(arg, replacements);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            replace_in_place(receiver, replacements);
            for arg in args {
                replace_in_place(arg, replacements);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            replace_in_place(callee, replacements);
            for arg in args {
                replace_in_place(arg, replacements);
            }
        }
        InstructionValue::PropertyLoad { object, .. } => {
            replace_in_place(object, replacements);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            replace_in_place(object, replacements);
            replace_in_place(value, replacements);
        }
        InstructionValue::ComputedLoad { object, property } => {
            replace_in_place(object, replacements);
            replace_in_place(property, replacements);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            replace_in_place(object, replacements);
            replace_in_place(property, replacements);
            replace_in_place(value, replacements);
        }
        InstructionValue::PropertyDelete { object, .. } => {
            replace_in_place(object, replacements);
        }
        InstructionValue::ComputedDelete { object, property } => {
            replace_in_place(object, replacements);
            replace_in_place(property, replacements);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                replace_in_place(&mut prop.value, replacements);
                if let crate::hir::types::ObjectPropertyKey::Computed(p) = &mut prop.key {
                    replace_in_place(p, replacements);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Spread(p)
                    | crate::hir::types::ArrayElement::Expression(p) => {
                        replace_in_place(p, replacements);
                    }
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            replace_in_place(tag, replacements);
            for attr in props {
                replace_in_place(&mut attr.value, replacements);
            }
            for child in children {
                replace_in_place(child, replacements);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                replace_in_place(child, replacements);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                replace_in_place(sub, replacements);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            replace_in_place(tag, replacements);
            for sub in &mut value.subexpressions {
                replace_in_place(sub, replacements);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => {
            replace_in_place(value, replacements);
        }
        InstructionValue::Await { value } => {
            replace_in_place(value, replacements);
        }
        InstructionValue::GetIterator { collection } => {
            replace_in_place(collection, replacements);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            replace_in_place(iterator, replacements);
        }
        InstructionValue::NextPropertyOf { value } => {
            replace_in_place(value, replacements);
        }
        InstructionValue::TypeCastExpression { value, .. } => {
            replace_in_place(value, replacements);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            replace_in_place(decl, replacements);
            for dep in deps {
                replace_in_place(dep, replacements);
            }
        }
        InstructionValue::FunctionExpression { lowered_func, .. } => {
            // Recurse into nested function bodies
            replace_in_hir_function(lowered_func, replacements);
        }
        InstructionValue::ObjectMethod { lowered_func } => {
            replace_in_hir_function(lowered_func, replacements);
        }
        // No places in these variants
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

fn replace_in_destructure_pattern(
    pattern: &mut crate::hir::types::DestructurePattern,
    replacements: &FxHashMap<IdentifierId, IdentifierId>,
) {
    use crate::hir::types::{DestructureArrayItem, DestructurePattern};
    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                replace_in_destructure_target(&mut prop.value, replacements);
            }
            if let Some(rest_place) = rest {
                replace_in_place(rest_place, replacements);
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Value(target) => {
                        replace_in_destructure_target(target, replacements);
                    }
                    DestructureArrayItem::Spread(place) => {
                        replace_in_place(place, replacements);
                    }
                    DestructureArrayItem::Hole => {}
                }
            }
            if let Some(rest_place) = rest {
                replace_in_place(rest_place, replacements);
            }
        }
    }
}

fn replace_in_destructure_target(
    target: &mut crate::hir::types::DestructureTarget,
    replacements: &FxHashMap<IdentifierId, IdentifierId>,
) {
    use crate::hir::types::DestructureTarget;
    match target {
        DestructureTarget::Place(place) => {
            replace_in_place(place, replacements);
        }
        DestructureTarget::Pattern(pattern) => {
            replace_in_destructure_pattern(pattern, replacements);
        }
    }
}

fn replace_in_hir_function(
    func: &mut crate::hir::types::HIRFunction,
    replacements: &FxHashMap<IdentifierId, IdentifierId>,
) {
    // Replace in params
    for param in &mut func.params {
        match param {
            crate::hir::types::Param::Identifier(place)
            | crate::hir::types::Param::Spread(place) => {
                replace_in_place(place, replacements);
            }
        }
    }
    // Replace in returns
    replace_in_place(&mut func.returns, replacements);
    // Replace in context
    for ctx in &mut func.context {
        replace_in_place(ctx, replacements);
    }
    // Replace in body
    for (_, block) in &mut func.body.blocks {
        for instr in &mut block.instructions {
            replace_in_place(&mut instr.lvalue, replacements);
            replace_in_instruction_value(&mut instr.value, replacements);
        }
        for phi in &mut block.phis {
            replace_in_place(&mut phi.place, replacements);
            for (_, operand) in &mut phi.operands {
                replace_in_place(operand, replacements);
            }
        }
        replace_in_terminal(&mut block.terminal, replacements);
    }
}

fn replace_in_terminal(
    terminal: &mut crate::hir::types::Terminal,
    replacements: &FxHashMap<IdentifierId, IdentifierId>,
) {
    use crate::hir::types::Terminal;
    match terminal {
        Terminal::If { test, .. }
        | Terminal::Branch { test, .. }
        | Terminal::Ternary { test, .. }
        | Terminal::Optional { test, .. } => {
            replace_in_place(test, replacements);
        }
        Terminal::Switch { test, cases, .. } => {
            replace_in_place(test, replacements);
            for case in cases {
                if let Some(t) = &mut case.test {
                    replace_in_place(t, replacements);
                }
            }
        }
        Terminal::Return { value } | Terminal::Throw { value } => {
            replace_in_place(value, replacements);
        }
        // Terminals that only contain BlockIds, no Places
        Terminal::Goto { .. }
        | Terminal::For { .. }
        | Terminal::ForOf { .. }
        | Terminal::ForIn { .. }
        | Terminal::DoWhile { .. }
        | Terminal::While { .. }
        | Terminal::Logical { .. }
        | Terminal::Sequence { .. }
        | Terminal::Label { .. }
        | Terminal::MaybeThrow { .. }
        | Terminal::Try { .. }
        | Terminal::Scope { .. }
        | Terminal::PrunedScope { .. }
        | Terminal::Unreachable => {}
    }
}
