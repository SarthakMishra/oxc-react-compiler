#![allow(dead_code)]

use rustc_hash::FxHashMap;

use crate::hir::types::{AliasingEffect, Effect, HIR, IdentifierId, InstructionId, MutableRange};

/// Compute mutable ranges for all identifiers.
///
/// Uses the effects computed by `infer_mutation_aliasing_effects` to determine
/// the instruction range during which each value is being mutated.
///
/// - `start`: instruction that creates the value
/// - `end`: last instruction that mutates the value (transitively through aliases)
pub fn infer_mutation_aliasing_ranges(hir: &mut HIR) {
    // Step 1: Build a map of each identifier to its creation point and all mutation sites.
    let mut creation_map: FxHashMap<IdentifierId, InstructionId> = FxHashMap::default();
    let mut mutation_map: FxHashMap<IdentifierId, Vec<InstructionId>> = FxHashMap::default();
    let mut alias_map: FxHashMap<IdentifierId, Vec<IdentifierId>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let instr_id = instr.id;

            // The lvalue's identifier is created at this instruction.
            let lvalue_id = instr.lvalue.identifier.id;
            creation_map.entry(lvalue_id).or_insert(instr_id);

            // Process effects to find mutations and aliases.
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    match effect {
                        AliasingEffect::Create { into, .. }
                        | AliasingEffect::CreateFrom { into, .. }
                        | AliasingEffect::CreateFunction { into, .. } => {
                            creation_map.entry(into.identifier.id).or_insert(instr_id);
                        }
                        AliasingEffect::Mutate { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateTransitiveConditionally { value } => {
                            mutation_map.entry(value.identifier.id).or_default().push(instr_id);
                        }
                        AliasingEffect::Alias { from, into }
                        | AliasingEffect::Assign { from, into }
                        | AliasingEffect::MaybeAlias { from, into } => {
                            alias_map
                                .entry(from.identifier.id)
                                .or_default()
                                .push(into.identifier.id);
                            alias_map
                                .entry(into.identifier.id)
                                .or_default()
                                .push(from.identifier.id);
                        }
                        AliasingEffect::Capture { from, into } => {
                            // Capture creates an indirect alias for mutation propagation.
                            alias_map
                                .entry(into.identifier.id)
                                .or_default()
                                .push(from.identifier.id);
                        }
                        _ => {}
                    }
                }
            }

            // Also check the lvalue's effect.
            if matches!(
                instr.lvalue.effect,
                Effect::Mutate | Effect::ConditionallyMutate | Effect::Store
            ) {
                mutation_map.entry(lvalue_id).or_default().push(instr_id);
            }
        }
    }

    // Step 2: Collect last-use sites for each identifier.
    // The mutable range should extend to the last point where the value is read,
    // not just the last mutation. This is critical for reactive scope inference.
    let mut last_use_map: FxHashMap<IdentifierId, InstructionId> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let instr_id = instr.id;
            // Collect all operand IDs and mark their last use
            let operand_ids = collect_operand_ids(&instr.value);
            for op_id in operand_ids {
                let entry = last_use_map.entry(op_id).or_insert(instr_id);
                if instr_id.0 > entry.0 {
                    *entry = instr_id;
                }
            }
        }
        // Terminal uses
        match &block.terminal {
            crate::hir::types::Terminal::Return { value }
            | crate::hir::types::Terminal::Throw { value } => {
                // Use a high instruction ID for terminal uses
                let terminal_id =
                    InstructionId(block.instructions.last().map_or(0, |i| i.id.0) + 1);
                let entry = last_use_map.entry(value.identifier.id).or_insert(terminal_id);
                if terminal_id.0 > entry.0 {
                    *entry = terminal_id;
                }
            }
            crate::hir::types::Terminal::If { test, .. }
            | crate::hir::types::Terminal::Branch { test, .. } => {
                let terminal_id =
                    InstructionId(block.instructions.last().map_or(0, |i| i.id.0) + 1);
                let entry = last_use_map.entry(test.identifier.id).or_insert(terminal_id);
                if terminal_id.0 > entry.0 {
                    *entry = terminal_id;
                }
            }
            _ => {}
        }
    }

    // Step 3: Propagate mutation sites through aliases transitively.
    let mut all_ids: Vec<IdentifierId> = creation_map.keys().copied().collect();
    all_ids.sort();

    let mut transitive_mutations: FxHashMap<IdentifierId, InstructionId> = FxHashMap::default();

    for &id in &all_ids {
        let mut last_mutation = InstructionId(0);

        // Direct mutations.
        if let Some(mutations) = mutation_map.get(&id) {
            for &m in mutations {
                if m.0 > last_mutation.0 {
                    last_mutation = m;
                }
            }
        }

        // Mutations through aliases (one level of transitivity).
        if let Some(aliases) = alias_map.get(&id) {
            for &alias_id in aliases {
                if let Some(mutations) = mutation_map.get(&alias_id) {
                    for &m in mutations {
                        if m.0 > last_mutation.0 {
                            last_mutation = m;
                        }
                    }
                }
            }
        }

        if last_mutation.0 > 0 {
            transitive_mutations.insert(id, last_mutation);
        }
    }

    // Step 4: Write mutable ranges back to identifiers.
    // Range end = max(last_mutation + 1, last_use + 1, start + 1)
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            let id = instr.lvalue.identifier.id;
            let start = creation_map.get(&id).copied().unwrap_or(instr.id);

            let mut end = InstructionId(start.0 + 1);

            // Extend to last mutation
            if let Some(&last_mut) = transitive_mutations.get(&id) {
                let mutation_end = InstructionId(last_mut.0 + 1);
                if mutation_end.0 > end.0 {
                    end = mutation_end;
                }
            }

            // Extend to last use
            if let Some(&last_use) = last_use_map.get(&id) {
                let use_end = InstructionId(last_use.0 + 1);
                if use_end.0 > end.0 {
                    end = use_end;
                }
            }

            instr.lvalue.identifier.mutable_range = MutableRange { start, end };
        }
    }
}

/// Collect operand identifier IDs from an instruction value.
fn collect_operand_ids(value: &crate::hir::types::InstructionValue) -> Vec<IdentifierId> {
    use crate::hir::types::InstructionValue;
    let mut ids = Vec::new();
    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            ids.push(place.identifier.id);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            ids.push(lvalue.identifier.id);
            ids.push(value.identifier.id);
        }
        InstructionValue::DeclareLocal { lvalue, .. }
        | InstructionValue::DeclareContext { lvalue } => {
            ids.push(lvalue.identifier.id);
        }
        InstructionValue::Destructure { value, .. } => {
            ids.push(value.identifier.id);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            ids.push(left.identifier.id);
            ids.push(right.identifier.id);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            ids.push(value.identifier.id);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            ids.push(lvalue.identifier.id);
        }
        InstructionValue::CallExpression { callee, args }
        | InstructionValue::NewExpression { callee, args } => {
            ids.push(callee.identifier.id);
            for arg in args {
                ids.push(arg.identifier.id);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            ids.push(receiver.identifier.id);
            for arg in args {
                ids.push(arg.identifier.id);
            }
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            ids.push(object.identifier.id);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            ids.push(object.identifier.id);
            ids.push(value.identifier.id);
        }
        InstructionValue::ComputedLoad { object, property }
        | InstructionValue::ComputedDelete { object, property } => {
            ids.push(object.identifier.id);
            ids.push(property.identifier.id);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            ids.push(object.identifier.id);
            ids.push(property.identifier.id);
            ids.push(value.identifier.id);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                ids.push(prop.value.identifier.id);
                if let crate::hir::types::ObjectPropertyKey::Computed(p) = &prop.key {
                    ids.push(p.identifier.id);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Expression(p)
                    | crate::hir::types::ArrayElement::Spread(p) => ids.push(p.identifier.id),
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            ids.push(tag.identifier.id);
            for attr in props {
                ids.push(attr.value.identifier.id);
            }
            for child in children {
                ids.push(child.identifier.id);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                ids.push(child.identifier.id);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                ids.push(sub.identifier.id);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            ids.push(tag.identifier.id);
            for sub in &value.subexpressions {
                ids.push(sub.identifier.id);
            }
        }
        InstructionValue::Await { value }
        | InstructionValue::StoreGlobal { value, .. }
        | InstructionValue::NextPropertyOf { value }
        | InstructionValue::TypeCastExpression { value, .. } => {
            ids.push(value.identifier.id);
        }
        InstructionValue::GetIterator { collection } => {
            ids.push(collection.identifier.id);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            ids.push(iterator.identifier.id);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            ids.push(decl.identifier.id);
            for dep in deps {
                ids.push(dep.identifier.id);
            }
        }
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. } => {}
    }
    ids
}
