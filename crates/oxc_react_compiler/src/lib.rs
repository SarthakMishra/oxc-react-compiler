// Crate-level lint configuration: suppress noisy style lints that don't
// affect correctness. This is proof-of-concept scaffold code.
#![allow(
    clippy::match_same_arms,
    clippy::format_push_string,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_pass_by_value,
    clippy::wildcard_in_or_patterns,
    clippy::match_single_binding,
    clippy::redundant_else,
    clippy::manual_let_else,
    clippy::unnecessary_wraps,
    clippy::result_unit_err,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::struct_field_names,
    clippy::items_after_statements,
    clippy::match_wildcard_for_single_variants,
    clippy::needless_continue,
    clippy::ptr_as_ptr,
    clippy::ref_as_ptr,
    clippy::nonminimal_bool,
    clippy::unused_self,
    clippy::undocumented_unsafe_blocks,
    clippy::collection_is_never_read,
    clippy::while_let_loop,
    clippy::only_used_in_recursion,
    clippy::option_option,
    clippy::if_same_then_else,
    clippy::single_match,
    clippy::needless_pass_by_ref_mut,
    clippy::comparison_chain
)]

pub mod debug_dump;
pub mod entrypoint;
pub mod error;
pub mod hir;
pub mod inference;
pub mod optimization;
pub mod reactive_scopes;
pub mod ssa;
pub mod utils;
pub mod validation;

// Public API
pub use entrypoint::options::{
    CompilationMode, GatingConfig, OutputMode, PanicThreshold, PluginOptions,
};
pub use entrypoint::program::{
    CompileResult, compile_program, compile_program_with_config, compile_program_with_source_map,
};
pub use hir::environment::EnvironmentConfig;
