
use crate::hir::types::{
    Effect, HIR, Identifier, IdentifierId, Instruction, InstructionId, InstructionValue,
    MutableRange, Place, Type,
};

/// Optimize method calls on props to avoid unnecessary memoization.
///
/// Detects `props.onClick()` patterns and converts:
///   `MethodCall { receiver: props, property: "onClick", args }`
/// into:
///   `PropertyLoad { object: props, property: "onClick" }` → new temp
///   `CallExpression { callee: new_temp, args }`
///
/// This allows the property load and the call to be in different reactive
/// scopes, improving memoization granularity.
pub fn optimize_props_method_calls(hir: &mut HIR) {
    // Find the next available instruction ID and identifier ID.
    let mut next_instr_id = hir
        .blocks
        .iter()
        .flat_map(|(_, b)| b.instructions.iter())
        .map(|i| i.id.0)
        .max()
        .unwrap_or(0)
        + 1;
    let mut next_ident_id = hir
        .blocks
        .iter()
        .flat_map(|(_, b)| b.instructions.iter())
        .map(|i| i.lvalue.identifier.id.0)
        .max()
        .unwrap_or(0)
        + 1;

    for (_, block) in &mut hir.blocks {
        let mut insertions: Vec<(usize, Instruction)> = Vec::new();

        for (idx, instr) in block.instructions.iter_mut().enumerate() {
            if let InstructionValue::MethodCall { ref receiver, ref property, .. } = instr.value {
                // Check if the receiver is a props parameter (first param, or named "props").
                let is_props = receiver
                    .identifier
                    .name
                    .as_deref()
                    .is_some_and(|n| n == "props" || n.starts_with("_t"))
                    && matches!(receiver.identifier.type_, Type::Object);

                if !is_props {
                    continue;
                }

                // Create a new temporary for the property load result.
                let temp_id = IdentifierId(next_ident_id);
                next_ident_id += 1;
                let load_instr_id = InstructionId(next_instr_id);
                next_instr_id += 1;

                let temp_ident = Identifier {
                    id: temp_id,
                    declaration_id: None,
                    name: None,
                    mutable_range: MutableRange {
                        start: load_instr_id,
                        end: InstructionId(load_instr_id.0 + 1),
                    },
                    scope: None,
                    type_: Type::Function,
                    loc: instr.loc,
                };

                let temp_place = Place {
                    identifier: temp_ident.clone(),
                    effect: Effect::Read,
                    reactive: false,
                    loc: instr.loc,
                };

                // Create the PropertyLoad instruction.
                let load_instr = Instruction {
                    id: load_instr_id,
                    lvalue: temp_place.clone(),
                    value: InstructionValue::PropertyLoad {
                        object: receiver.clone(),
                        property: property.clone(),
                    },
                    loc: instr.loc,
                    effects: None,
                };

                // Replace the MethodCall with a CallExpression using the loaded property.
                if let InstructionValue::MethodCall { args, .. } = std::mem::replace(
                    &mut instr.value,
                    InstructionValue::Primitive { value: crate::hir::types::Primitive::Undefined },
                ) {
                    instr.value = InstructionValue::CallExpression { callee: temp_place, args };
                }

                insertions.push((idx, load_instr));
            }
        }

        // Insert the PropertyLoad instructions before their corresponding calls.
        // Process in reverse order to keep indices valid.
        for (idx, instr) in insertions.into_iter().rev() {
            block.instructions.insert(idx, instr);
        }
    }
}
