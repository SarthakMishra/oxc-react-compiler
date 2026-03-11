use oxc_diagnostics::OxcDiagnostic;
use oxc_span::Span;

/// Categories of compiler errors, matching upstream React Compiler error taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Invalid input that violates React rules
    InvalidReact,
    /// Invalid input that violates JavaScript semantics
    InvalidJS,
    /// Valid input that the compiler doesn't yet support
    Todo,
    /// Internal compiler invariant violation
    InvariantViolation,
}

/// A compiler error with source location and category.
#[derive(Debug)]
pub struct CompilerError {
    pub category: ErrorCategory,
    pub span: Span,
    pub message: String,
}

impl CompilerError {
    pub fn into_diagnostic(self) -> OxcDiagnostic {
        OxcDiagnostic::warn(self.message).with_label(self.span)
    }
}
