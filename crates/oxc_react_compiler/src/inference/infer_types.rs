use rustc_hash::{FxHashMap, FxHashSet};

use crate::hir::types::{
    BinaryOp, DestructureArrayItem, DestructurePattern, DestructureTarget, HIR, IdentifierId,
    InstructionValue, Primitive, PrimitiveType, Type, UnaryOp,
};

/// Infer types for all identifiers in the HIR.
///
/// This is a forward dataflow pass that propagates type information:
/// - Primitive literals -> Primitive type
/// - Binary/unary ops -> result type based on operator
/// - Property loads -> Object type
/// - Function expressions -> Function type
/// - Call expressions -> Poly (unknown return type, refined by shape system)
/// - useRef() calls -> Ref type
/// - useState()/useReducer() calls -> marks destructured setter as SetState
pub fn infer_types(hir: &mut HIR) {
    // Track identifiers that hold hook return values for destructuring propagation
    let mut ref_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut state_tuple_ids: FxHashSet<IdentifierId> = FxHashSet::default();

    // Build id-to-name map to resolve callee names through LoadGlobal/LoadLocal.
    // After SSA, CallExpression callees are temporaries with no name — we need
    // to trace back through the load instruction to find the original name.
    let mut id_to_name: FxHashMap<IdentifierId, String> = FxHashMap::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadGlobal { binding } => {
                    id_to_name.insert(instr.lvalue.identifier.id, binding.name.clone());
                }
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if let Some(name) = &place.identifier.name {
                        id_to_name.insert(instr.lvalue.identifier.id, name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    // Pass 1: Infer instruction types and identify hook call returns
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            let inferred = infer_instruction_type(&instr.value);
            instr.lvalue.identifier.type_ = inferred;

            // Track hook return value identifiers.
            // Resolve callee name through LoadGlobal/LoadLocal for cases where
            // the callee place is a nameless temporary.
            if let InstructionValue::CallExpression { callee, .. } = &instr.value {
                let callee_name = callee
                    .identifier
                    .name
                    .as_deref()
                    .or_else(|| id_to_name.get(&callee.identifier.id).map(String::as_str));

                if let Some(name) = callee_name {
                    if name == "useRef" {
                        instr.lvalue.identifier.type_ = Type::Ref;
                        ref_ids.insert(instr.lvalue.identifier.id);
                    } else if matches!(
                        name,
                        "useState"
                            | "useReducer"
                            | "useTransition"
                            | "useOptimistic"
                            | "useActionState"
                    ) {
                        state_tuple_ids.insert(instr.lvalue.identifier.id);
                    }
                }
            }
        }
    }

    // Pass 2: Propagate hook types through destructuring
    // When useState/useReducer returns are destructured as [state, setter],
    // mark the second element as SetState. When useRef return is destructured
    // (uncommon but possible), propagate Ref type.
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let InstructionValue::Destructure { value, lvalue_pattern } = &instr.value {
                if state_tuple_ids.contains(&value.identifier.id) {
                    // Mark second element of array destructure as SetState
                    if let DestructurePattern::Array { items, .. } = lvalue_pattern
                        && let Some(DestructureArrayItem::Value(DestructureTarget::Place(p))) =
                            items.get(1)
                    {
                        // We can't mutate through the immutable reference, so collect
                        // the ID to mark in a third pass
                        state_tuple_ids.remove(&value.identifier.id);
                        ref_ids.remove(&p.identifier.id); // avoid collision
                        // Store the setter ID in state_tuple_ids for pass 3
                        state_tuple_ids.insert(p.identifier.id);
                    }
                } else if ref_ids.contains(&value.identifier.id) {
                    // useRef destructuring is uncommon but handle it
                    ref_ids.remove(&value.identifier.id);
                }
            }
        }
    }

    // Pass 3: Apply SetState type to destructured setter identifiers.
    // NOTE: After SSA renaming, the destructure pattern Place IDs don't match
    // instruction lvalue IDs. This pass applies types where it can, but the
    // validator (validate_no_set_state_in_render) does its own destructure
    // detection as a more reliable fallback.
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if state_tuple_ids.contains(&instr.lvalue.identifier.id) {
                instr.lvalue.identifier.type_ = Type::SetState;
            }
            if ref_ids.contains(&instr.lvalue.identifier.id) {
                instr.lvalue.identifier.type_ = Type::Ref;
            }
        }
    }
}

fn infer_instruction_type(value: &InstructionValue) -> Type {
    match value {
        InstructionValue::Primitive { value } => match value {
            Primitive::Null => Type::Primitive(PrimitiveType::Null),
            Primitive::Undefined => Type::Primitive(PrimitiveType::Undefined),
            Primitive::Boolean(_) => Type::Primitive(PrimitiveType::Boolean),
            Primitive::Number(_) => Type::Primitive(PrimitiveType::Number),
            Primitive::String(_) => Type::Primitive(PrimitiveType::String),
            Primitive::BigInt(_) => Type::Primitive(PrimitiveType::BigInt),
        },
        InstructionValue::JSXText { .. } => Type::Primitive(PrimitiveType::String),
        InstructionValue::TemplateLiteral { .. } => Type::Primitive(PrimitiveType::String),
        InstructionValue::RegExpLiteral { .. } => Type::Object,

        InstructionValue::BinaryExpression { op, .. } => match op {
            BinaryOp::Add => Type::Poly, // could be string or number
            BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Exp
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
            | BinaryOp::UnsignedShiftRight => Type::Primitive(PrimitiveType::Number),
            BinaryOp::EqEq
            | BinaryOp::NotEq
            | BinaryOp::StrictEq
            | BinaryOp::StrictNotEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq
            | BinaryOp::In
            | BinaryOp::InstanceOf => Type::Primitive(PrimitiveType::Boolean),
            BinaryOp::NullishCoalescing => Type::Poly,
        },
        InstructionValue::UnaryExpression { op, .. } => match op {
            UnaryOp::Not => Type::Primitive(PrimitiveType::Boolean),
            UnaryOp::TypeOf => Type::Primitive(PrimitiveType::String),
            UnaryOp::Void => Type::Primitive(PrimitiveType::Undefined),
            UnaryOp::Delete => Type::Primitive(PrimitiveType::Boolean),
            UnaryOp::Minus | UnaryOp::Plus | UnaryOp::BitwiseNot => {
                Type::Primitive(PrimitiveType::Number)
            }
        },

        InstructionValue::ObjectExpression { .. } => Type::Object,
        InstructionValue::ArrayExpression { .. } => Type::Object,
        InstructionValue::JsxExpression { .. } | InstructionValue::JsxFragment { .. } => {
            Type::Object
        }

        InstructionValue::FunctionExpression { .. } | InstructionValue::ObjectMethod { .. } => {
            Type::Function
        }

        // For most other instructions, we don't know the type without more context
        _ => Type::Poly,
    }
}
