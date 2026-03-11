#![allow(dead_code)]

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

/// Severity level derived from the error category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Todo,
}

impl ErrorCategory {
    /// Returns the severity level for this category.
    pub fn severity(self) -> ErrorSeverity {
        match self {
            ErrorCategory::InvalidReact | ErrorCategory::InvalidJS => ErrorSeverity::Error,
            ErrorCategory::Todo => ErrorSeverity::Todo,
            ErrorCategory::InvariantViolation => ErrorSeverity::Error,
        }
    }
}

/// Determines when the compiler should bail out of compilation.
/// Standalone copy to avoid circular dependency with options.rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanicThreshold {
    /// Bail on any error
    AllErrors,
    /// Bail only on invariant violations
    CriticalErrors,
    /// Never bail
    None,
}

/// A compiler error with source location, category, and optional detail.
#[derive(Debug)]
pub struct CompilerError {
    pub category: ErrorCategory,
    pub span: Span,
    pub message: String,
    pub detail: Option<String>,
}

impl CompilerError {
    /// Returns the severity of this error, derived from its category.
    pub fn severity(&self) -> ErrorSeverity {
        self.category.severity()
    }

    /// Returns a string error code for diagnostic reporting.
    pub fn code(&self) -> &'static str {
        match self.category {
            ErrorCategory::InvalidReact => "react-compiler(invalid-react)",
            ErrorCategory::InvalidJS => "react-compiler(invalid-js)",
            ErrorCategory::Todo => "react-compiler(todo)",
            ErrorCategory::InvariantViolation => "react-compiler(invariant)",
        }
    }

    /// Converts this error into an `OxcDiagnostic`, using error or warn
    /// based on severity.
    pub fn into_diagnostic(self) -> OxcDiagnostic {
        let diag = match self.severity() {
            ErrorSeverity::Error => OxcDiagnostic::error(self.message),
            ErrorSeverity::Warning | ErrorSeverity::Todo => OxcDiagnostic::warn(self.message),
        };
        diag.with_label(self.span)
    }

    /// Creates an `InvalidReact` error.
    pub fn invalid_react(span: Span, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::InvalidReact,
            span,
            message: message.into(),
            detail: None,
        }
    }

    /// Creates an `InvalidJS` error.
    pub fn invalid_js(span: Span, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::InvalidJS,
            span,
            message: message.into(),
            detail: None,
        }
    }

    /// Creates a `Todo` error for unimplemented features.
    pub fn todo(span: Span, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::Todo,
            span,
            message: message.into(),
            detail: None,
        }
    }

    /// Creates an `InvariantViolation` error for internal compiler bugs.
    pub fn invariant(span: Span, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::InvariantViolation,
            span,
            message: message.into(),
            detail: None,
        }
    }
}

/// Accumulates errors during a compiler pass and decides whether to bail.
#[derive(Debug, Default)]
pub struct ErrorCollector {
    errors: Vec<CompilerError>,
}

impl ErrorCollector {
    /// Pushes an error into the collector.
    pub fn push(&mut self, error: CompilerError) {
        self.errors.push(error);
    }

    /// Returns `true` if any errors have been collected.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns `true` if any critical (invariant violation) errors exist.
    pub fn has_critical_errors(&self) -> bool {
        self.errors
            .iter()
            .any(|e| e.category == ErrorCategory::InvariantViolation)
    }

    /// Returns `true` if compilation should bail given the threshold.
    pub fn should_bail(&self, threshold: PanicThreshold) -> bool {
        match threshold {
            PanicThreshold::AllErrors => self.has_errors(),
            PanicThreshold::CriticalErrors => self.has_critical_errors(),
            PanicThreshold::None => false,
        }
    }

    /// Converts all collected errors into diagnostics.
    pub fn into_diagnostics(self) -> Vec<OxcDiagnostic> {
        self.errors
            .into_iter()
            .map(CompilerError::into_diagnostic)
            .collect()
    }

    /// Drains all errors out of the collector and returns them.
    pub fn drain(&mut self) -> Vec<CompilerError> {
        std::mem::take(&mut self.errors)
    }

    /// Returns the number of collected errors.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Returns `true` if no errors have been collected.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}
