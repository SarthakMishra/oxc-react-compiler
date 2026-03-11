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
pub use entrypoint::options::PluginOptions;
pub use entrypoint::program::{CompileResult, compile_program, compile_program_with_source_map};
