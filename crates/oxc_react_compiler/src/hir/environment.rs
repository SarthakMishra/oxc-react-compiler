#![allow(dead_code)]

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

    // Analysis behavior flags
    pub enable_assume_hooks_follow_rules_of_react: bool,
    pub enable_transitively_freeze_function_expressions: bool,
    pub enable_optional_dependencies: bool,
    pub enable_treat_ref_like_identifiers_as_refs: bool,

    // Dev/HMR
    pub enable_reset_cache_on_source_file_changes: bool,
    pub enable_emit_hook_guards: bool,

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
            enable_assume_hooks_follow_rules_of_react: false,
            enable_transitively_freeze_function_expressions: true,
            enable_optional_dependencies: true,
            enable_treat_ref_like_identifiers_as_refs: false,
            enable_reset_cache_on_source_file_changes: false,
            enable_emit_hook_guards: false,
            custom_macros: Vec::new(),
            custom_hooks: FxHashMap::default(),
        }
    }
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
        Self {
            config,
            id_generator: IdGenerator::new(),
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new(EnvironmentConfig::default())
    }
}
