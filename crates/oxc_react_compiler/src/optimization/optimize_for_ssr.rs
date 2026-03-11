#![allow(dead_code)]
use crate::hir::types::HIR;

/// Optimize HIR for server-side rendering.
///
/// In SSR mode, memoization is unnecessary since each render produces fresh output.
/// This pass removes reactive scope annotations from identifiers so that the
/// reactive scope construction passes (33-46) produce no scopes, and codegen
/// emits simpler code without `useMemoCache`.
pub fn optimize_for_ssr(hir: &mut HIR) {
    // Strip scope annotations from all identifiers. Without scopes,
    // the downstream passes (infer_reactive_scope_variables, build_reactive_scope_terminals)
    // become no-ops, and codegen emits plain function bodies.
    for (_, block) in hir.blocks.iter_mut() {
        for instr in &mut block.instructions {
            instr.lvalue.identifier.scope = None;
        }
    }
}
