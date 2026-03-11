#![allow(dead_code)]
use crate::hir::types::HIR;

/// Optimize HIR for server-side rendering.
/// In SSR mode, memoization is unnecessary since each render produces
/// new output. This pass can simplify or remove memoization-related constructs.
pub fn optimize_for_ssr(hir: &mut HIR) {
    // In SSR mode:
    // 1. Remove reactive scope tracking (not needed for single renders)
    // 2. Simplify effect hooks (effects don't run on server)
    // 3. Keep the basic structure for correctness
    let _ = hir;
}
