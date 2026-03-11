#![allow(dead_code)]

/// Top-level plugin options controlling the compiler behavior.
#[derive(Debug, Clone)]
pub struct PluginOptions {
    pub compilation_mode: CompilationMode,
    pub output_mode: OutputMode,
    pub target: ReactTarget,
    pub gating: Option<GatingConfig>,
    pub panic_threshold: PanicThreshold,
    pub sources: Option<SourceFilter>,
}

impl Default for PluginOptions {
    fn default() -> Self {
        Self {
            compilation_mode: CompilationMode::Infer,
            output_mode: OutputMode::Client,
            target: ReactTarget::React19,
            gating: None,
            panic_threshold: PanicThreshold::CriticalErrors,
            sources: None,
        }
    }
}

/// Determines how the compiler finds functions to compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilationMode {
    /// Infer whether to compile based on heuristics (default)
    Infer,
    /// Only compile functions with explicit annotations ("use memo", "use no memo")
    Annotation,
    /// Compile all top-level functions that look like components/hooks
    Syntax,
    /// Compile everything
    All,
}

/// Controls what the compiler outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputMode {
    /// Normal client-side compilation with memoization
    Client,
    /// Server-side rendering mode (different memoization strategy)
    SSR,
    /// Lint-only mode: run analysis, collect errors, skip codegen
    Lint,
}

/// Target React version for generated code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReactTarget {
    React17,
    React18,
    React19,
}

/// Controls when the compiler bails out due to errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanicThreshold {
    /// Bail on any error
    AllErrors,
    /// Bail only on critical errors (invariant violations)
    CriticalErrors,
    /// Never bail, collect all errors
    None,
}

/// Configuration for gating compiled output behind a feature flag.
#[derive(Debug, Clone)]
pub struct GatingConfig {
    /// Import source for the gating function
    pub import_source: String,
    /// Function name to use as a gating check
    pub function_name: String,
}

/// Filter for which source files to compile.
#[derive(Debug, Clone)]
pub struct SourceFilter {
    /// Glob patterns for files to include
    pub include: Vec<String>,
    /// Glob patterns for files to exclude
    pub exclude: Vec<String>,
}
