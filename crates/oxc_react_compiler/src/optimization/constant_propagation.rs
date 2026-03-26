use crate::hir::types::{BinaryOp, HIR, IdentifierId, InstructionValue, Primitive, UnaryOp};
use rustc_hash::FxHashMap;

/// Propagate known constant values through the HIR and fold constant expressions.
///
/// This pass performs three transformations:
/// 1. Replace `LoadLocal` of known constant identifiers with `Primitive` values
/// 2. Fold `BinaryExpression` where both operands are constants
/// 3. Fold `UnaryExpression` where the operand is a constant
///
/// Returns the number of instructions changed (for iterative application).
///
/// This is a simple forward dataflow pass (not a full lattice-based analysis).
/// It does NOT propagate across function boundaries.
pub fn constant_propagation(hir: &mut HIR) -> usize {
    let mut changed = 0;

    // Phase 1: Collect constants (identifiers assigned exactly one constant value)
    let mut constants = collect_constants(hir);

    if constants.is_empty() {
        return 0;
    }

    // Phase 2: Replace LoadLocal with Primitive, fold BinaryExpression/UnaryExpression
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } => {
                    if let Some(constant) = constants.get(&place.identifier.id) {
                        instr.value = InstructionValue::Primitive { value: constant.clone() };
                        changed += 1;
                    }
                }
                InstructionValue::BinaryExpression { op, left, right } => {
                    if let (Some(lv), Some(rv)) =
                        (constants.get(&left.identifier.id), constants.get(&right.identifier.id))
                        && let Some(result) = fold_binary(*op, lv, rv)
                    {
                        // Record the folded result as a new constant
                        constants.insert(instr.lvalue.identifier.id, result.clone());
                        instr.value = InstructionValue::Primitive { value: result };
                        changed += 1;
                    }
                }
                InstructionValue::UnaryExpression { op, value } => {
                    if let Some(val) = constants.get(&value.identifier.id)
                        && let Some(result) = fold_unary(*op, val)
                    {
                        constants.insert(instr.lvalue.identifier.id, result.clone());
                        instr.value = InstructionValue::Primitive { value: result };
                        changed += 1;
                    }
                }
                _ => {}
            }
        }
    }

    changed
}

/// Collect identifiers that are assigned exactly one constant value across
/// all blocks. If an identifier is assigned more than once with different
/// values, or assigned a non-constant, it is excluded.
fn collect_constants(hir: &HIR) -> FxHashMap<IdentifierId, Primitive> {
    // None means "assigned but not a single constant" (poisoned).
    let mut constants: FxHashMap<IdentifierId, Option<Primitive>> = FxHashMap::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let id = instr.lvalue.identifier.id;
            match &instr.value {
                InstructionValue::Primitive { value } => {
                    match constants.get(&id) {
                        None => {
                            constants.insert(id, Some(value.clone()));
                        }
                        Some(Some(existing)) if *existing == *value => {
                            // Same constant, OK
                        }
                        _ => {
                            // Multiple different values -> not constant
                            constants.insert(id, None);
                        }
                    }
                }
                InstructionValue::StoreLocal { lvalue, value, .. } => {
                    // The lvalue target gets a non-primitive assignment (the value
                    // comes from another place, not a literal). Mark the lvalue's
                    // identifier as non-constant.
                    let target_id = lvalue.identifier.id;
                    constants.insert(target_id, None);
                    // Also mark the instruction's own lvalue
                    let _ = value;
                    constants.insert(id, None);
                }
                // Any other instruction that writes to the lvalue invalidates it
                // as a constant candidate.
                _ => {
                    constants.insert(id, None);
                }
            }
        }
    }

    // Phase 2: Propagate constants through phi nodes.
    // If all operands of a phi are the same constant, the phi result is that constant.
    // Iterate to fixed point since phi chains may depend on each other.
    loop {
        let mut changed = false;
        for (_, block) in &hir.blocks {
            for phi in &block.phis {
                let phi_id = phi.place.identifier.id;
                // Skip if already resolved
                if constants.contains_key(&phi_id) {
                    continue;
                }
                // Check if all operands map to the same constant
                let mut phi_const: Option<&Primitive> = None;
                let mut all_same = true;
                for (_, operand) in &phi.operands {
                    if let Some(Some(c)) = constants.get(&operand.identifier.id) {
                        match phi_const {
                            None => phi_const = Some(c),
                            Some(existing) if existing == c => {}
                            _ => {
                                all_same = false;
                                break;
                            }
                        }
                    } else {
                        // Operand is not a constant (or is poisoned)
                        all_same = false;
                        break;
                    }
                }
                if all_same && let Some(c) = phi_const {
                    constants.insert(phi_id, Some(c.clone()));
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    constants.into_iter().filter_map(|(id, val)| val.map(|v| (id, v))).collect()
}

/// Fold a binary expression with two constant operands.
/// Returns `None` if the operation cannot be folded (e.g., division by zero,
/// BigInt operations, or operations with non-numeric types that don't apply).
fn fold_binary(op: BinaryOp, left: &Primitive, right: &Primitive) -> Option<Primitive> {
    match op {
        // Arithmetic operators
        BinaryOp::Add => fold_add(left, right),
        BinaryOp::Sub => fold_arithmetic(left, right, |a, b| a - b),
        BinaryOp::Mul => fold_arithmetic(left, right, |a, b| a * b),
        BinaryOp::Div => {
            let (a, b) = to_numbers(left, right)?;
            // JavaScript division by zero returns Infinity/-Infinity/NaN, not an error.
            // We fold it to match upstream behavior.
            Some(Primitive::Number(a / b))
        }
        BinaryOp::Mod => {
            let (a, b) = to_numbers(left, right)?;
            Some(Primitive::Number(a % b))
        }
        BinaryOp::Exp => {
            let (a, b) = to_numbers(left, right)?;
            Some(Primitive::Number(a.powf(b)))
        }

        // Bitwise operators (operate on i32)
        BinaryOp::BitwiseAnd => {
            let (a, b) = to_i32s(left, right)?;
            Some(Primitive::Number(f64::from(a & b)))
        }
        BinaryOp::BitwiseOr => {
            let (a, b) = to_i32s(left, right)?;
            Some(Primitive::Number(f64::from(a | b)))
        }
        BinaryOp::BitwiseXor => {
            let (a, b) = to_i32s(left, right)?;
            Some(Primitive::Number(f64::from(a ^ b)))
        }
        BinaryOp::ShiftLeft => {
            let (a, b) = to_i32s(left, right)?;
            let shift = (b as u32) & 0x1f;
            Some(Primitive::Number(f64::from(a << shift)))
        }
        BinaryOp::ShiftRight => {
            let (a, b) = to_i32s(left, right)?;
            let shift = (b as u32) & 0x1f;
            Some(Primitive::Number(f64::from(a >> shift)))
        }
        BinaryOp::UnsignedShiftRight => {
            let (a, b) = to_i32s(left, right)?;
            let shift = (b as u32) & 0x1f;
            Some(Primitive::Number(f64::from((a as u32) >> shift)))
        }

        // Comparison operators
        BinaryOp::StrictEq => Some(Primitive::Boolean(strict_eq(left, right))),
        BinaryOp::StrictNotEq => Some(Primitive::Boolean(!strict_eq(left, right))),
        BinaryOp::EqEq => abstract_eq(left, right).map(Primitive::Boolean),
        BinaryOp::NotEq => abstract_eq(left, right).map(|v| Primitive::Boolean(!v)),
        BinaryOp::Lt => compare_values(left, right)
            .map(|ord| Primitive::Boolean(matches!(ord, std::cmp::Ordering::Less))),
        BinaryOp::LtEq => compare_values(left, right).map(|ord| {
            Primitive::Boolean(matches!(ord, std::cmp::Ordering::Less | std::cmp::Ordering::Equal))
        }),
        BinaryOp::Gt => compare_values(left, right)
            .map(|ord| Primitive::Boolean(matches!(ord, std::cmp::Ordering::Greater))),
        BinaryOp::GtEq => compare_values(left, right).map(|ord| {
            Primitive::Boolean(matches!(
                ord,
                std::cmp::Ordering::Greater | std::cmp::Ordering::Equal
            ))
        }),

        // Cannot fold at compile time
        BinaryOp::In | BinaryOp::InstanceOf | BinaryOp::NullishCoalescing => None,
    }
}

/// Fold a unary expression with a constant operand.
fn fold_unary(op: UnaryOp, value: &Primitive) -> Option<Primitive> {
    match op {
        UnaryOp::Minus => {
            let n = to_number(value)?;
            Some(Primitive::Number(-n))
        }
        UnaryOp::Plus => {
            let n = to_number(value)?;
            Some(Primitive::Number(n))
        }
        UnaryOp::Not => {
            let b = to_boolean(value);
            Some(Primitive::Boolean(!b))
        }
        UnaryOp::BitwiseNot => {
            let n = to_i32(value)?;
            Some(Primitive::Number(f64::from(!n)))
        }
        UnaryOp::TypeOf => {
            let result = match value {
                Primitive::Null => "object",
                Primitive::Undefined => "undefined",
                Primitive::Boolean(_) => "boolean",
                Primitive::Number(_) => "number",
                Primitive::String(_) => "string",
                Primitive::BigInt(_) => "bigint",
            };
            Some(Primitive::String(result.to_string()))
        }
        UnaryOp::Void => Some(Primitive::Undefined),
        UnaryOp::Delete => None, // Cannot fold delete
    }
}

/// Handle `+` which does string concatenation when either operand is a string.
fn fold_add(left: &Primitive, right: &Primitive) -> Option<Primitive> {
    // String concatenation: if either side is a string, concatenate
    match (left, right) {
        (Primitive::String(a), Primitive::String(b)) => Some(Primitive::String(format!("{a}{b}"))),
        (Primitive::String(a), other) => {
            Some(Primitive::String(format!("{a}{}", to_string(other))))
        }
        (other, Primitive::String(b)) => {
            Some(Primitive::String(format!("{}{b}", to_string(other))))
        }
        _ => fold_arithmetic(left, right, |a, b| a + b),
    }
}

/// Fold an arithmetic operation on two values coerced to numbers.
fn fold_arithmetic(
    left: &Primitive,
    right: &Primitive,
    op: impl FnOnce(f64, f64) -> f64,
) -> Option<Primitive> {
    let (a, b) = to_numbers(left, right)?;
    Some(Primitive::Number(op(a, b)))
}

/// Coerce a primitive to a JavaScript number (ToNumber).
fn to_number(val: &Primitive) -> Option<f64> {
    match val {
        Primitive::Number(n) => Some(*n),
        Primitive::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
        Primitive::Null => Some(0.0),
        Primitive::Undefined => Some(f64::NAN),
        Primitive::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() { Some(0.0) } else { trimmed.parse::<f64>().ok() }
        }
        Primitive::BigInt(_) => None, // BigInt cannot be coerced to number
    }
}

/// Coerce two primitives to numbers.
fn to_numbers(left: &Primitive, right: &Primitive) -> Option<(f64, f64)> {
    Some((to_number(left)?, to_number(right)?))
}

/// Coerce a primitive to i32 (ToInt32 in JS spec).
fn to_i32(val: &Primitive) -> Option<i32> {
    let n = to_number(val)?;
    if n.is_nan() || n.is_infinite() || n == 0.0 {
        Some(0)
    } else {
        // JavaScript ToInt32: truncate, then modulo 2^32, then sign
        Some(n as i32)
    }
}

/// Coerce two primitives to i32.
fn to_i32s(left: &Primitive, right: &Primitive) -> Option<(i32, i32)> {
    Some((to_i32(left)?, to_i32(right)?))
}

/// Coerce a primitive to boolean (ToBoolean in JS spec).
fn to_boolean(val: &Primitive) -> bool {
    match val {
        Primitive::Null | Primitive::Undefined => false,
        Primitive::Boolean(b) => *b,
        Primitive::Number(n) => *n != 0.0 && !n.is_nan(),
        Primitive::String(s) => !s.is_empty(),
        Primitive::BigInt(s) => s != "0" && !s.is_empty(),
    }
}

/// Coerce a primitive to string (ToString in JS spec).
fn to_string(val: &Primitive) -> String {
    match val {
        Primitive::Null => "null".to_string(),
        Primitive::Undefined => "undefined".to_string(),
        Primitive::Boolean(b) => b.to_string(),
        Primitive::Number(n) => format_js_number(*n),
        Primitive::String(s) => s.clone(),
        Primitive::BigInt(s) => s.clone(),
    }
}

/// Format a number the way JavaScript does.
fn format_js_number(n: f64) -> String {
    if n.is_nan() {
        "NaN".to_string()
    } else if n.is_infinite() {
        if n.is_sign_positive() { "Infinity" } else { "-Infinity" }.to_string()
    } else if n == 0.0 {
        "0".to_string()
    } else if n.fract() == 0.0 && n.abs() < 1e15 {
        // Integer-like numbers: print without decimal point
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Strict equality (===) between two primitives.
fn strict_eq(left: &Primitive, right: &Primitive) -> bool {
    match (left, right) {
        (Primitive::Null, Primitive::Null) => true,
        (Primitive::Undefined, Primitive::Undefined) => true,
        (Primitive::Boolean(a), Primitive::Boolean(b)) => a == b,
        (Primitive::Number(a), Primitive::Number(b)) => {
            // NaN !== NaN in JavaScript
            if a.is_nan() || b.is_nan() {
                return false;
            }
            a == b
        }
        (Primitive::String(a), Primitive::String(b)) => a == b,
        (Primitive::BigInt(a), Primitive::BigInt(b)) => a == b,
        _ => false, // Different types -> not strictly equal
    }
}

/// Abstract equality (==) between two primitives.
/// Returns `None` if the comparison would require runtime type coercion
/// that we cannot fully model (e.g., object-to-primitive).
fn abstract_eq(left: &Primitive, right: &Primitive) -> Option<bool> {
    // Same type -> use strict equality
    if std::mem::discriminant(left) == std::mem::discriminant(right) {
        return Some(strict_eq(left, right));
    }
    // null == undefined (and vice versa)
    match (left, right) {
        (Primitive::Null, Primitive::Undefined) | (Primitive::Undefined, Primitive::Null) => {
            Some(true)
        }
        // null/undefined != anything else
        (Primitive::Null | Primitive::Undefined, _)
        | (_, Primitive::Null | Primitive::Undefined) => Some(false),
        // Number/Boolean comparisons: coerce to number
        _ => {
            let (a, b) = to_numbers(left, right)?;
            if a.is_nan() || b.is_nan() { Some(false) } else { Some(a == b) }
        }
    }
}

/// Compare two primitive values for ordering.
/// Returns `None` if the comparison is not meaningful (e.g., NaN involved).
fn compare_values(left: &Primitive, right: &Primitive) -> Option<std::cmp::Ordering> {
    let (a, b) = to_numbers(left, right)?;
    if a.is_nan() || b.is_nan() {
        return None;
    }
    a.partial_cmp(&b)
}
