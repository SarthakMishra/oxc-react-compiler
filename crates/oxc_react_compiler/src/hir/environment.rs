use super::types::IdGenerator;
use rustc_hash::FxHashMap;

/// Configuration for how the compiler analyzes and transforms code.
/// Maps to upstream `EnvironmentConfig` with ~30 flags.
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    // Memoization behavior
    pub enable_preserve_existing_memoization_guarantees: bool,
    pub validate_preserve_existing_memoization_guarantees: bool,

    // Outlining (optional transforms)
    pub enable_function_outlining: bool,
    pub enable_jsx_outlining: bool,

    // Validation toggles
    pub validate_hooks_usage: bool,
    pub validate_ref_access_during_render: bool,
    pub validate_no_set_state_in_render: bool,
    pub validate_no_set_state_in_effects: bool,
    pub validate_no_derived_computations_in_effects: bool,
    pub validate_no_jsx_in_try_statements: bool,
    pub validate_no_capitalized_calls: bool,
    pub validate_exhaustive_memo_dependencies: bool,
    pub validate_exhaustive_effect_dependencies: bool,

    // Debug / internal flags
    pub assert_valid_mutable_ranges: bool,
    pub enable_name_anonymous_functions: bool,

    // Analysis behavior flags
    pub enable_assume_hooks_follow_rules_of_react: bool,
    pub enable_transitively_freeze_function_expressions: bool,
    pub enable_optional_dependencies: bool,
    pub enable_treat_ref_like_identifiers_as_refs: bool,
    /// When true, treat `setX` naming pattern as state setters even without type info.
    pub enable_treat_set_identifiers_as_state_setters: bool,
    /// When true, allow setState calls inside effects when value comes from ref.current.
    pub enable_allow_set_state_from_refs_in_effects: bool,
    /// When true, show verbose diagnostics for setState-in-effect violations.
    pub enable_verbose_no_set_state_in_effect: bool,

    // Output mode
    pub enable_ssr: bool,

    // Dev/HMR
    pub enable_reset_cache_on_source_file_changes: bool,
    pub enable_emit_hook_guards: bool,
    /// External function config for hook guards (import source + function name).
    pub emit_hook_guards_external_function: Option<ExternalFunctionConfig>,

    // Validation passes
    /// Mode for exhaustive effect dependency validation (off/all/missing-only/extra-only).
    pub validate_exhaustive_effect_dependencies_mode: ExhaustiveDepsMode,
    /// Validate that no impure functions (console.log, Math.random, etc.) are called in render.
    pub validate_no_impure_functions_in_render: bool,
    /// List of import sources that should be blocked from compiled code.
    pub blocklisted_imports: Vec<String>,

    // Extensibility
    pub custom_macros: Vec<String>,
    pub custom_hooks: FxHashMap<String, CustomHookConfig>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            enable_preserve_existing_memoization_guarantees: false,
            validate_preserve_existing_memoization_guarantees: false,
            enable_function_outlining: false,
            enable_jsx_outlining: false,
            validate_hooks_usage: true,
            validate_ref_access_during_render: true,
            validate_no_set_state_in_render: true,
            validate_no_set_state_in_effects: false,
            validate_no_derived_computations_in_effects: false,
            validate_no_jsx_in_try_statements: false,
            validate_no_capitalized_calls: true,
            validate_exhaustive_memo_dependencies: false,
            validate_exhaustive_effect_dependencies: false,
            assert_valid_mutable_ranges: false,
            enable_name_anonymous_functions: true,
            enable_assume_hooks_follow_rules_of_react: false,
            enable_transitively_freeze_function_expressions: true,
            enable_optional_dependencies: true,
            enable_treat_ref_like_identifiers_as_refs: false,
            enable_treat_set_identifiers_as_state_setters: true,
            enable_allow_set_state_from_refs_in_effects: false,
            enable_verbose_no_set_state_in_effect: false,
            enable_ssr: false,
            enable_reset_cache_on_source_file_changes: false,
            enable_emit_hook_guards: false,
            emit_hook_guards_external_function: None,
            validate_exhaustive_effect_dependencies_mode: ExhaustiveDepsMode::Off,
            validate_no_impure_functions_in_render: false,
            blocklisted_imports: Vec::new(),
            custom_macros: Vec::new(),
            custom_hooks: FxHashMap::default(),
        }
    }
}

impl EnvironmentConfig {
    /// Returns a config with all validation passes enabled.
    /// Useful for testing to ensure every validation pass runs.
    pub fn all_validations_enabled() -> Self {
        Self {
            validate_hooks_usage: true,
            validate_ref_access_during_render: true,
            validate_no_set_state_in_render: true,
            validate_no_set_state_in_effects: true,
            validate_no_derived_computations_in_effects: true,
            validate_no_jsx_in_try_statements: true,
            validate_no_capitalized_calls: true,
            validate_exhaustive_memo_dependencies: true,
            validate_exhaustive_effect_dependencies: true,
            enable_preserve_existing_memoization_guarantees: true,
            validate_preserve_existing_memoization_guarantees: true,
            ..Self::default()
        }
    }
}

/// Mode for exhaustive effect dependency validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExhaustiveDepsMode {
    /// Disabled (no checking)
    #[default]
    Off,
    /// Report all issues (both missing and extra)
    All,
    /// Only report missing dependencies
    MissingOnly,
    /// Only report extraneous dependencies
    ExtraOnly,
}

/// External function configuration for hook guards or gating.
#[derive(Debug, Clone)]
pub struct ExternalFunctionConfig {
    /// Import source (e.g. "react-compiler-runtime")
    pub source: String,
    /// Function name to import
    pub function_name: String,
}

/// Configuration for a custom hook's behavior.
#[derive(Debug, Clone)]
pub struct CustomHookConfig {
    /// Name of the hook
    pub name: String,
    /// If the hook's return value is mutable
    pub return_is_mutable: bool,
    /// If the hook's arguments are captured
    pub args_are_captured: bool,
    /// Indices of arguments that are effect callbacks
    pub effect_arg_indices: Vec<usize>,
}

/// The Environment holds all compiler state for analyzing a single function.
#[derive(Debug)]
pub struct Environment {
    pub config: EnvironmentConfig,
    pub id_generator: IdGenerator,
}

impl Environment {
    pub fn new(config: EnvironmentConfig) -> Self {
        Self { config, id_generator: IdGenerator::new() }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new(EnvironmentConfig::default())
    }
}
