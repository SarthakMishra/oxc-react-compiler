
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

/// Fine-grained diagnostic kind for filtering in lint rules (Tier 2).
///
/// Each variant maps to a specific validation pass, enabling lint rules to
/// filter diagnostics by kind rather than fragile string matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticKind {
    /// Hooks called conditionally or in wrong order (validate_hooks_usage)
    HooksViolation,
    /// Mutation of frozen/immutable values (validate_no_freezing_known_mutable_functions)
    ImmutabilityViolation,
    /// Compiler memoization doesn't preserve manual useMemo/useCallback (validate_preserved_manual_memoization)
    MemoizationPreservation,
    /// Missing or extraneous deps in useMemo/useCallback (validate_exhaustive_dependencies for memo hooks)
    MemoDependency,
    /// Missing or extraneous deps in useEffect/useLayoutEffect (validate_exhaustive_dependencies for effect hooks)
    EffectDependency,
    /// Ref.current accessed during render (validate_no_ref_access_in_render)
    RefAccessInRender,
    /// setState called during render (validate_no_set_state_in_render)
    SetStateInRender,
    /// setState called directly in effect body (validate_no_set_state_in_effects)
    SetStateInEffects,
    /// JSX inside try block (validate_no_jsx_in_try)
    JsxInTry,
    /// Capitalized function calls that aren't components (validate_no_capitalized_calls)
    CapitalizedCalls,
    /// Context variable reassignment (validate_context_variable_lvalues)
    ContextVariableLvalues,
    /// Static component detection (validate_static_components)
    StaticComponents,
    /// Derived computations in effects (validate_no_derived_computations_in_effects)
    DerivedComputationsInEffects,
    /// Locals reassigned after render (validate_locals_not_reassigned_after_render)
    LocalsReassignedAfterRender,
    /// useMemo/useCallback validation (validate_use_memo)
    UseMemoValidation,
    /// Impure function called during render (validate_no_impure_functions_in_render)
    ImpureFunctionInRender,
    /// Blocklisted import used (validate_blocklisted_imports)
    BlocklistedImport,
    /// useMemo returns void (validate_no_void_use_memo)
    VoidUseMemo,
    /// Break target validation failure (assert_well_formed_break_targets)
    MalformedBreakTarget,
    /// Internal invariant violation (assert_valid_mutable_ranges)
    InvariantViolation,
    /// General/unclassified diagnostic
    Other,
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

/// A compiler error with source location, category, diagnostic kind, and optional detail.
#[derive(Debug)]
pub struct CompilerError {
    pub category: ErrorCategory,
    pub kind: DiagnosticKind,
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

    /// Creates an `OxcDiagnostic` from this error without consuming it.
    pub fn to_diagnostic(&self) -> OxcDiagnostic {
        let diag = match self.severity() {
            ErrorSeverity::Error => OxcDiagnostic::error(self.message.clone()),
            ErrorSeverity::Warning | ErrorSeverity::Todo => {
                OxcDiagnostic::warn(self.message.clone())
            }
        };
        diag.with_label(self.span)
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

    /// Creates an `InvalidReact` error with a specific diagnostic kind.
    pub fn invalid_react_with_kind(
        span: Span,
        message: impl Into<String>,
        kind: DiagnosticKind,
    ) -> Self {
        Self {
            category: ErrorCategory::InvalidReact,
            kind,
            span,
            message: message.into(),
            detail: None,
        }
    }

    /// Creates an `InvalidReact` error with `DiagnosticKind::Other`.
    pub fn invalid_react(span: Span, message: impl Into<String>) -> Self {
        Self::invalid_react_with_kind(span, message, DiagnosticKind::Other)
    }

    /// Creates an `InvalidJS` error with a specific diagnostic kind.
    pub fn invalid_js_with_kind(
        span: Span,
        message: impl Into<String>,
        kind: DiagnosticKind,
    ) -> Self {
        Self {
            category: ErrorCategory::InvalidJS,
            kind,
            span,
            message: message.into(),
            detail: None,
        }
    }

    /// Creates an `InvalidJS` error with `DiagnosticKind::Other`.
    pub fn invalid_js(span: Span, message: impl Into<String>) -> Self {
        Self::invalid_js_with_kind(span, message, DiagnosticKind::Other)
    }

    /// Creates a `Todo` error for unimplemented features.
    pub fn todo(span: Span, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::Todo,
            kind: DiagnosticKind::Other,
            span,
            message: message.into(),
            detail: None,
        }
    }

    /// Creates an `InvariantViolation` error for internal compiler bugs.
    pub fn invariant(span: Span, message: impl Into<String>) -> Self {
        Self {
            category: ErrorCategory::InvariantViolation,
            kind: DiagnosticKind::InvariantViolation,
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
        self.errors.iter().any(|e| e.category == ErrorCategory::InvariantViolation)
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
        self.errors.into_iter().map(CompilerError::into_diagnostic).collect()
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

    /// Returns a reference to the collected errors.
    pub fn errors(&self) -> &[CompilerError] {
        &self.errors
    }

    /// Appends all errors from another collector into this one.
    pub fn extend(&mut self, other: &mut ErrorCollector) {
        self.errors.append(&mut other.errors);
    }

    /// Filters errors by diagnostic kind and converts them to diagnostics.
    pub fn diagnostics_by_kind(&self, kind: DiagnosticKind) -> Vec<OxcDiagnostic> {
        self.errors.iter().filter(|e| e.kind == kind).map(CompilerError::to_diagnostic).collect()
    }

    /// Filters errors by multiple diagnostic kinds and converts them to diagnostics.
    pub fn diagnostics_by_kinds(&self, kinds: &[DiagnosticKind]) -> Vec<OxcDiagnostic> {
        self.errors
            .iter()
            .filter(|e| kinds.contains(&e.kind))
            .map(CompilerError::to_diagnostic)
            .collect()
    }
}
