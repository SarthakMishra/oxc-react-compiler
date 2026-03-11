#![allow(dead_code)]
use crate::hir::types::HIR;

/// Outline JSX expressions into separate variables.
/// This is an optional optimization that improves memoization granularity
/// by making JSX elements individually memoizable.
pub fn outline_jsx(hir: &mut HIR) {
    // TODO: Walk instructions, find JsxExpression that are used inline,
    // extract them into separate instructions with their own temporaries.
    let _ = hir;
}
