#![allow(dead_code)]
use crate::hir::types::HIR;

/// Outline nested function expressions to top level when possible.
/// This reduces closure overhead and improves memoization.
pub fn outline_functions(hir: &mut HIR) {
    // TODO: Identify function expressions that don't capture mutable state
    // and can be hoisted to the module level.
    let _ = hir;
}
