pub mod pipeline;
pub mod phases;
pub mod workers;
pub mod type_env;
pub mod ast_cache;
pub mod grammar;
pub mod utils;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IngestError {
    #[error("Pipeline phase {phase} failed: {message}")]
    PhaseError { phase: String, message: String },
    #[error("Parse timeout for {path} after {timeout_secs}s")]
    ParseTimeout { path: String, timeout_secs: u64 },
    #[error("Tree-sitter error for {path}: {message}")]
    TreeSitterError { path: String, message: String },
    #[error(transparent)]
    Core(#[from] gitnexus_core::error::CoreError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
