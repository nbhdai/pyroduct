pub mod callback;
pub mod data;
pub mod factory;
pub mod interconnect;
pub mod normal;
pub mod server;
pub mod session;
pub mod session_diff;

#[cfg(feature = "sql")]
pub mod sql;

pub use callback::Callback;
pub use factory::{PipelineConfig, PipelineFactory};
pub use interconnect::PlaybookCollection;
pub use normal::{ExecutionRecord, Failure, Pipeline};
pub use server::{PipelineServer, ServerExecutionRecord, ServerPipeline};
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
