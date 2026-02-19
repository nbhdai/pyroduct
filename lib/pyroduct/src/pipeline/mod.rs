pub mod pipeline;
pub mod wasm_execute;
pub mod wasm;

pub use pipeline::{CapabilityConfig, ModuleConfig, PipelineConfig, PipelineDef};
pub use wasm_execute::{Failure, Pipeline, PipelinePool};

use thiserror::Error;

use crate::PyroError;
use crate::ffi::host::capability::CapabilityLoading;
use crate::pipeline::wasm::WasmError;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error(transparent)]
    Pyro(#[from] PyroError),

    #[error(transparent)]
    Wasm(#[from] WasmError),

    #[error(transparent)]
    Capability(#[from] CapabilityLoading),

    #[error("{0}")]
    Config(String),
}

pub type PipelineResult<T> = Result<T, PipelineError>;