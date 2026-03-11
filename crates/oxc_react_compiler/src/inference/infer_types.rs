#![allow(dead_code)]

use crate::hir::types::{BinaryOp, HIR, InstructionValue, Primitive, PrimitiveType, Type, UnaryOp};

/// Infer types for all identifiers in the HIR.
///
/// This is a forward dataflow pass that propagates type information:
/// - Primitive literals -> Primitive type
/// - Binary/unary ops -> result type based on operator
/// - Property loads -> Object type
/// - Function expressions -> Function type
/// - Call expressions -> Poly (unknown return type, refined by shape system)
pub fn infer_types(hir: &mut HIR) {
    for (_, block) in hir.blocks.iter_mut() {
        for instr in &mut block.instructions {
            let inferred = infer_instruction_type(&instr.value);
            instr.lvalue.identifier.type_ = inferred;
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
