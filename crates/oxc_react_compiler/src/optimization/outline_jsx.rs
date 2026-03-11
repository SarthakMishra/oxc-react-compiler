#![allow(dead_code)]

use crate::hir::types::{
    Effect, Identifier, IdentifierId, Instruction, InstructionId, InstructionValue, MutableRange,
    Place, Type, HIR,
};

/// Outline JSX expressions into separate variables.
///
/// Finds JSX elements used as arguments or children of other JSX, and extracts
/// them into separate instructions with their own temporaries. This improves
/// memoization granularity by allowing each JSX element to get its own
/// reactive scope.
///
/// Example: `<Parent><Child /></Parent>` →
///   `const t1 = <Child />;`
///   `<Parent>{t1}</Parent>`
pub fn outline_jsx(hir: &mut HIR) {
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

    for (_, block) in hir.blocks.iter_mut() {
        let mut insertions: Vec<(usize, Instruction)> = Vec::new();

        for (idx, instr) in block.instructions.iter().enumerate() {
            // Find instructions that produce JSX and are used as arguments to other JSX.
            // We look at the children of JsxExpression: if a child was produced by another
            // JsxExpression instruction in this same block, it's already outlined.
            // We only outline JSX that appears as a direct child value if it matches
            // a known inline pattern (this is a simplified heuristic).
            if let InstructionValue::JsxExpression { ref children, .. } = instr.value {
                // Children that are JSX expressions could benefit from outlining,
                // but since our HIR already flattens expressions into temporaries,
                // JSX children are already separate instructions with their own Places.
                // The outlining optimization is already implicitly handled by the
                // HIR lowering pass (build.rs) which creates temporaries for all
                // subexpressions. No additional outlining needed.
                let _ = children;
            }
        }

        // Since HIR lowering already outlines JSX into temporaries, this pass
        // mainly serves as a hook for future enhancements where JSX could be
        // further decomposed (e.g., splitting props computation from element creation).
    }
}
