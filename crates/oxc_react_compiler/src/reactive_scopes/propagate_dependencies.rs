#![allow(dead_code)]

use crate::hir::types::{HIR, IdentifierId, InstructionValue, ReactiveScopeDependency, ScopeId};
use rustc_hash::{FxHashMap, FxHashSet};

/// Propagate scope dependencies through the HIR.
///
/// For each reactive scope, determine which external values it depends on.
/// These become the "deps" that are checked at runtime to decide whether
/// to recompute the scope's output.
pub fn propagate_scope_dependencies_hir(hir: &mut HIR) {
    // Phase 1: Build a map of scope_id -> set of identifier ids declared in that scope
    let mut scope_declarations: FxHashMap<ScopeId, FxHashSet<IdentifierId>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(ref scope) = instr.lvalue.identifier.scope {
                scope_declarations.entry(scope.id).or_default().insert(instr.lvalue.identifier.id);
            }
        }
    }

    // Phase 2: For each instruction in a scope, find operands from outside the scope
    let mut scope_deps: FxHashMap<ScopeId, Vec<ReactiveScopeDependency>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let scope_id = match &instr.lvalue.identifier.scope {
                Some(scope) => scope.id,
                None => continue,
            };

            let declared = scope_declarations.get(&scope_id).cloned().unwrap_or_default();

            // Collect operands from the instruction value
            let operands = collect_operand_places(&instr.value);
            for place in operands {
                let op_id = place.identifier.id;
                // If this operand is not declared within the same scope, it's a dependency
                if !declared.contains(&op_id) {
                    // Check if already added
                    let deps = scope_deps.entry(scope_id).or_default();
                    let already_added = deps.iter().any(|d| d.identifier.id == op_id);
                    if !already_added {
                        deps.push(ReactiveScopeDependency {
                            identifier: place.identifier.clone(),
                            reactive: place.reactive,
                            path: Vec::new(),
                        });
                    }
                }
            }

            // Handle property loads: build dependency paths
            if let InstructionValue::PropertyLoad { object, property } = &instr.value {
                let obj_id = object.identifier.id;
                if !declared.contains(&obj_id) {
                    let deps = scope_deps.entry(scope_id).or_default();
                    // Check if we already have a dep for this object and can extend its path
                    let existing = deps.iter_mut().find(|d| d.identifier.id == obj_id);
                    if let Some(dep) = existing {
                        dep.path.push(crate::hir::types::DependencyPathEntry {
                            property: property.clone(),
                            optional: false,
                        });
                    }
                }
            }
        }
    }

    // Phase 3: Write the dependencies back onto the scopes
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(ref mut scope) = instr.lvalue.identifier.scope
                && let Some(deps) = scope_deps.remove(&scope.id) {
                    // Only include reactive dependencies
                    scope.dependencies = deps.into_iter().filter(|d| d.reactive).collect();
                }
        }
    }
}

/// Collect all places referenced as operands in an instruction value.
fn collect_operand_places(value: &InstructionValue) -> Vec<&crate::hir::types::Place> {
    let mut places = Vec::new();

    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            places.push(place);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            places.push(lvalue);
            places.push(value);
        }
        InstructionValue::DeclareLocal { lvalue, .. }
        | InstructionValue::DeclareContext { lvalue } => {
            places.push(lvalue);
        }
        InstructionValue::Destructure { value, .. } => {
            places.push(value);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            places.push(left);
            places.push(right);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            places.push(value);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            places.push(lvalue);
        }
        InstructionValue::CallExpression { callee, args } => {
            places.push(callee);
            for arg in args {
                places.push(arg);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            places.push(receiver);
            for arg in args {
                places.push(arg);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            places.push(callee);
            for arg in args {
                places.push(arg);
            }
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            places.push(object);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            places.push(object);
            places.push(value);
        }
        InstructionValue::ComputedLoad { object, property } => {
            places.push(object);
            places.push(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            places.push(object);
            places.push(property);
            places.push(value);
        }
        InstructionValue::ComputedDelete { object, property } => {
            places.push(object);
            places.push(property);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                places.push(&prop.value);
                if let crate::hir::types::ObjectPropertyKey::Computed(place) = &prop.key {
                    places.push(place);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Expression(p)
                    | crate::hir::types::ArrayElement::Spread(p) => {
                        places.push(p);
                    }
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            places.push(tag);
            for attr in props {
                places.push(&attr.value);
            }
            for child in children {
                places.push(child);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                places.push(child);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                places.push(sub);
            }
        }
        InstructionValue::Await { value }
        | InstructionValue::GetIterator { collection: value }
        | InstructionValue::NextPropertyOf { value }
        | InstructionValue::TypeCastExpression { value, .. } => {
            places.push(value);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            places.push(iterator);
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            places.push(tag);
            for sub in &value.subexpressions {
                places.push(sub);
            }
        }
        InstructionValue::StoreGlobal { value, .. } => {
            places.push(value);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            places.push(decl);
            for dep in deps {
                places.push(dep);
            }
        }
        // No operands
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. } => {}
    }

    places
}
