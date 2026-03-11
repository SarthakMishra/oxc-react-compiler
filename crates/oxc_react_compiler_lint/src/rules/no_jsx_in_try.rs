#![allow(dead_code)]
//! # no-jsx-in-try
//!
//! Disallows JSX expressions inside `try` blocks. React error boundaries should
//! be used instead for error handling around JSX.

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;
use oxc_span::Span;

/// Check for JSX elements inside try blocks.
pub fn check_no_jsx_in_try(program: &Program<'_>) -> Vec<OxcDiagnostic> {
    let mut visitor = NoJsxInTryVisitor { diagnostics: Vec::new(), try_depth: 0 };
    visitor.visit_program(program);
    visitor.diagnostics
}

struct NoJsxInTryVisitor {
    diagnostics: Vec<OxcDiagnostic>,
    /// How many nested `try` blocks we are currently inside.
    try_depth: u32,
}

impl NoJsxInTryVisitor {
    fn report(&mut self, span: Span) {
        self.diagnostics.push(
            OxcDiagnostic::warn(
                "JSX expressions should not be used inside try blocks. Use error boundaries instead.",
            )
            .with_label(span),
        );
    }
}

impl<'a> Visit<'a> for NoJsxInTryVisitor {
    fn visit_try_statement(&mut self, it: &TryStatement<'a>) {
        // Only the `block` (try body) is problematic. The catch/finally are fine.
        self.try_depth += 1;
        self.visit_block_statement(&it.block);
        self.try_depth -= 1;

        // Visit catch and finally normally (JSX is fine there).
        if let Some(handler) = &it.handler {
            self.visit_catch_clause(handler);
        }
        if let Some(finalizer) = &it.finalizer {
            self.visit_block_statement(finalizer);
        }
    }

    fn visit_jsx_element(&mut self, it: &JSXElement<'a>) {
        if self.try_depth > 0 {
            self.report(it.span);
            // Don't walk children — one diagnostic per element is enough.
            return;
        }
        walk::walk_jsx_element(self, it);
    }

    fn visit_jsx_fragment(&mut self, it: &JSXFragment<'a>) {
        if self.try_depth > 0 {
            self.report(it.span);
            return;
        }
        walk::walk_jsx_fragment(self, it);
    }
}
