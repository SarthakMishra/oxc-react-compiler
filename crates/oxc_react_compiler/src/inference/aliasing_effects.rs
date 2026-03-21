use rustc_hash::FxHashMap;

use crate::hir::types::{
    AliasingEffect, ArrayElement, DestructureArrayItem, DestructurePattern, DestructureTarget,
    FunctionSignature, IdentifierId, InstructionValue, Place, SourceLocation, ValueKind,
    ValueReason,
};

/// Compute the aliasing effects for a single instruction.
///
/// This is the core of the effect system — it determines how each instruction
/// affects the abstract heap model. The effects are later used by
/// `infer_mutation_aliasing_effects` for fixpoint iteration.
///
/// `fn_signatures` maps callee IdentifierIds to their known function signatures,
/// enabling precise per-parameter effects instead of conservative defaults.
#[expect(clippy::implicit_hasher)]
pub fn compute_instruction_effects(
    value: &InstructionValue,
    lvalue: &Place,
    loc: SourceLocation,
    fn_signatures: &FxHashMap<IdentifierId, FunctionSignature>,
) -> Vec<AliasingEffect> {
    let mut effects = Vec::new();

    match value {
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::TemplateLiteral { .. } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Primitive,
                reason: ValueReason::KnownValue,
            });
        }

        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            effects.push(AliasingEffect::Alias { from: place.clone(), into: lvalue.clone() });
        }

        InstructionValue::StoreLocal { lvalue: store_lvalue, value, .. } => {
            effects
                .push(AliasingEffect::Assign { from: value.clone(), into: store_lvalue.clone() });
            effects.push(AliasingEffect::Assign { from: value.clone(), into: lvalue.clone() });
        }

        InstructionValue::StoreContext { lvalue: store_lvalue, value } => {
            effects
                .push(AliasingEffect::Assign { from: value.clone(), into: store_lvalue.clone() });
            effects
                .push(AliasingEffect::Capture { from: value.clone(), into: store_lvalue.clone() });
            effects.push(AliasingEffect::Assign { from: value.clone(), into: lvalue.clone() });
        }

        InstructionValue::ObjectExpression { properties } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Mutable,
                reason: ValueReason::KnownValue,
            });
            for prop in properties {
                effects.push(AliasingEffect::Capture {
                    from: prop.value.clone(),
                    into: lvalue.clone(),
                });
            }
        }

        InstructionValue::ArrayExpression { elements } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Mutable,
                reason: ValueReason::KnownValue,
            });
            for el in elements {
                match el {
                    ArrayElement::Expression(p) | ArrayElement::Spread(p) => {
                        effects.push(AliasingEffect::Capture {
                            from: p.clone(),
                            into: lvalue.clone(),
                        });
                    }
                    ArrayElement::Hole => {}
                }
            }
        }

        InstructionValue::CallExpression { callee, args, .. } => {
            let sig = fn_signatures.get(&callee.identifier.id).cloned();
            effects.push(AliasingEffect::Apply {
                receiver: callee.clone(),
                function: callee.clone(),
                mutates_function: false,
                args: args.clone(),
                into: lvalue.clone(),
                signature: sig,
                loc,
            });
        }

        InstructionValue::MethodCall { receiver, args, .. } => {
            // DIVERGENCE: MethodCall signature lookup is not supported yet.
            // The receiver is the object, not the method — looking up receiver.id
            // would incorrectly apply the object's signature to a method call.
            effects.push(AliasingEffect::Apply {
                receiver: receiver.clone(),
                function: receiver.clone(),
                mutates_function: false,
                args: args.clone(),
                into: lvalue.clone(),
                signature: None,
                loc,
            });
            // The receiver itself may be mutated by the method call.
            // refine_effects filters this out when the receiver is Frozen.
            effects.push(AliasingEffect::MutateTransitiveConditionally { value: receiver.clone() });
        }

        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::ComputedLoad { object, .. } => {
            effects.push(AliasingEffect::CreateFrom { from: object.clone(), into: lvalue.clone() });
        }

        InstructionValue::PropertyStore { object, value, .. }
        | InstructionValue::ComputedStore { object, value, .. } => {
            effects.push(AliasingEffect::Mutate { value: object.clone(), reason: None });
            effects.push(AliasingEffect::Capture { from: value.clone(), into: object.clone() });
        }

        InstructionValue::PropertyDelete { object, .. }
        | InstructionValue::ComputedDelete { object, .. } => {
            effects.push(AliasingEffect::Mutate { value: object.clone(), reason: None });
        }

        InstructionValue::FunctionExpression { lowered_func, .. } => {
            let captures: Vec<Place> = lowered_func.context.clone();
            effects.push(AliasingEffect::CreateFunction {
                captures,
                function: lvalue.clone(),
                into: lvalue.clone(),
            });
        }

        InstructionValue::JsxExpression { tag, props, children } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Frozen,
                reason: ValueReason::KnownValue,
            });
            effects
                .push(AliasingEffect::ImmutableCapture { from: tag.clone(), into: lvalue.clone() });
            for attr in props {
                effects.push(AliasingEffect::Freeze {
                    value: attr.value.clone(),
                    reason: ValueReason::JsxCaptured,
                });
                // Capture prop into JSX element (matches upstream: Freeze + Capture)
                effects.push(AliasingEffect::Capture {
                    from: attr.value.clone(),
                    into: lvalue.clone(),
                });
            }
            for child in children {
                effects.push(AliasingEffect::Freeze {
                    value: child.clone(),
                    reason: ValueReason::JsxCaptured,
                });
                // Capture child into JSX element (matches upstream: Freeze + Capture)
                effects.push(AliasingEffect::Capture { from: child.clone(), into: lvalue.clone() });
            }
        }

        InstructionValue::JsxFragment { children } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Frozen,
                reason: ValueReason::KnownValue,
            });
            for child in children {
                effects.push(AliasingEffect::Freeze {
                    value: child.clone(),
                    reason: ValueReason::JsxCaptured,
                });
                // Capture child into fragment (matches upstream: Freeze + Capture)
                effects.push(AliasingEffect::Capture { from: child.clone(), into: lvalue.clone() });
            }
        }

        InstructionValue::LoadGlobal { .. } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Global,
                reason: ValueReason::KnownValue,
            });
        }

        InstructionValue::Await { value } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Mutable,
                reason: ValueReason::Other,
            });
            effects.push(AliasingEffect::MutateTransitiveConditionally { value: value.clone() });
            effects.push(AliasingEffect::Capture { from: value.clone(), into: lvalue.clone() });
        }

        InstructionValue::NewExpression { callee, args } => {
            let sig = fn_signatures.get(&callee.identifier.id).cloned();
            effects.push(AliasingEffect::Apply {
                receiver: callee.clone(),
                function: callee.clone(),
                mutates_function: false,
                args: args.clone(),
                into: lvalue.clone(),
                signature: sig,
                loc,
            });
        }

        InstructionValue::DeclareLocal { .. } | InstructionValue::DeclareContext { .. } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Mutable,
                reason: ValueReason::Other,
            });
        }

        InstructionValue::Destructure { lvalue_pattern, value } => {
            // Emit per-pattern-item effects matching upstream behavior:
            // - Each identifier target: CreateFrom { from: value, into: item_place }
            // - Each spread target: Create(Mutable) + Capture { from: value, into: spread_place }
            // - Instruction lvalue: Assign { from: value, into: lvalue }
            collect_destructure_pattern_effects(lvalue_pattern, value, &mut effects);
            effects.push(AliasingEffect::Assign { from: value.clone(), into: lvalue.clone() });
        }

        InstructionValue::PrefixUpdate { lvalue: target, .. }
        | InstructionValue::PostfixUpdate { lvalue: target, .. } => {
            effects.push(AliasingEffect::Mutate { value: target.clone(), reason: None });
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Primitive,
                reason: ValueReason::KnownValue,
            });
        }

        InstructionValue::StoreGlobal { value: val, .. } => {
            effects.push(AliasingEffect::MutateGlobal {
                place: val.clone(),
                error: "Cannot mutate global variables during render".to_string(),
            });
        }

        InstructionValue::GetIterator { collection } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Mutable,
                reason: ValueReason::Other,
            });
            effects.push(AliasingEffect::Alias { from: collection.clone(), into: lvalue.clone() });
            effects
                .push(AliasingEffect::MutateTransitiveConditionally { value: collection.clone() });
        }

        InstructionValue::IteratorNext { iterator, .. } => {
            effects
                .push(AliasingEffect::CreateFrom { from: iterator.clone(), into: lvalue.clone() });
            effects.push(AliasingEffect::MutateConditionally { value: iterator.clone() });
        }

        InstructionValue::NextPropertyOf { value } => {
            effects.push(AliasingEffect::CreateFrom { from: value.clone(), into: lvalue.clone() });
        }

        InstructionValue::TypeCastExpression { value, .. } => {
            effects.push(AliasingEffect::Alias { from: value.clone(), into: lvalue.clone() });
        }

        InstructionValue::TaggedTemplateExpression { tag, .. } => {
            let sig = fn_signatures.get(&tag.identifier.id).cloned();
            effects.push(AliasingEffect::Apply {
                receiver: tag.clone(),
                function: tag.clone(),
                mutates_function: false,
                args: vec![],
                into: lvalue.clone(),
                signature: sig,
                loc,
            });
        }

        InstructionValue::ObjectMethod { .. } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Mutable,
                reason: ValueReason::KnownValue,
            });
        }

        InstructionValue::BinaryExpression { .. } | InstructionValue::UnaryExpression { .. } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Primitive,
                reason: ValueReason::KnownValue,
            });
        }

        InstructionValue::StartMemoize { .. } | InstructionValue::FinishMemoize { .. } => {
            // Memoization markers do not produce aliasing effects
        }

        InstructionValue::UnsupportedNode { .. } => {
            effects.push(AliasingEffect::Create {
                into: lvalue.clone(),
                value: ValueKind::Mutable,
                reason: ValueReason::Other,
            });
        }
    }

    effects
}

/// Recursively collect per-item aliasing effects from a destructure pattern.
///
/// Upstream emits a `CreateFrom` for each identifier target and
/// `Create(Mutable) + Capture` for spread targets. This replaces the old
/// single `CreateFrom` that only covered the instruction lvalue.
fn collect_destructure_pattern_effects(
    pattern: &DestructurePattern,
    value: &Place,
    effects: &mut Vec<AliasingEffect>,
) {
    match pattern {
        DestructurePattern::Object { properties, rest } => {
            for prop in properties {
                collect_destructure_target_effects(&prop.value, value, effects);
            }
            if let Some(rest_place) = rest {
                // Spread in object destructure: Create(Mutable) + Capture
                effects.push(AliasingEffect::Create {
                    into: rest_place.clone(),
                    value: ValueKind::Mutable,
                    reason: ValueReason::Other,
                });
                effects.push(AliasingEffect::Capture {
                    from: value.clone(),
                    into: rest_place.clone(),
                });
            }
        }
        DestructurePattern::Array { items, rest } => {
            for item in items {
                match item {
                    DestructureArrayItem::Hole => {}
                    DestructureArrayItem::Value(target) => {
                        collect_destructure_target_effects(target, value, effects);
                    }
                    DestructureArrayItem::Spread(spread_place) => {
                        // Spread in array destructure: Create(Mutable) + Capture
                        effects.push(AliasingEffect::Create {
                            into: spread_place.clone(),
                            value: ValueKind::Mutable,
                            reason: ValueReason::Other,
                        });
                        effects.push(AliasingEffect::Capture {
                            from: value.clone(),
                            into: spread_place.clone(),
                        });
                    }
                }
            }
            if let Some(rest_place) = rest {
                effects.push(AliasingEffect::Create {
                    into: rest_place.clone(),
                    value: ValueKind::Mutable,
                    reason: ValueReason::Other,
                });
                effects.push(AliasingEffect::Capture {
                    from: value.clone(),
                    into: rest_place.clone(),
                });
            }
        }
    }
}

/// Process a single destructure target — either a place or a nested pattern.
fn collect_destructure_target_effects(
    target: &DestructureTarget,
    value: &Place,
    effects: &mut Vec<AliasingEffect>,
) {
    match target {
        DestructureTarget::Place(place) => {
            effects.push(AliasingEffect::CreateFrom { from: value.clone(), into: place.clone() });
        }
        DestructureTarget::Pattern(nested_pattern) => {
            collect_destructure_pattern_effects(nested_pattern, value, effects);
        }
    }
}
