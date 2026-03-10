pub mod pipeline;
pub mod wasm_execute;

pub use pipeline::{PipelineConfig, PipelineFactory};
pub use wasm_execute::{Failure, Pipeline, PipelinePool};
use thiserror::Error;

use crate::PyroError;
use crate::module::{WasmError, capability::CapabilityError};

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error(transparent)]
    Pyro(#[from] PyroError),

    #[error(transparent)]
    Wasm(#[from] WasmError),

    #[error(transparent)]
    Capability(#[from] CapabilityError),

    #[error("{0}")]
    Config(String),
}

pub type PipelineResult<T> = Result<T, PipelineError>;