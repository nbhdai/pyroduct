pub mod pipeline;
pub mod session;
pub mod session_diff;
pub mod normal;
pub mod data;

#[cfg(feature = "host")]
pub mod sql;

pub use pipeline::{PipelineConfig, PipelineFactory};
use thiserror::Error;
pub use normal::{Failure, Pipeline, PipelinePool, ExecutionRecord};

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
