use pyro_artifacts::{
    artifacts::Playbook,
    cache::{CacheManager, LoadedPlaybook},
};
use serde::{Deserialize, Serialize};

use crate::{
    CapturedError, PyroError,
    format::log_wal::LogWal,
    module::{PyroFactory, WasmError},
    pipeline::{Pipeline, session::SessionPipeline, session_diff::SessionDiffPipeline},
};

use super::PipelineError;

// =============================================================================
// Config (deserialized from TOML / JSON)
// =============================================================================

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PipelineConfig {
    pub playbook: Playbook,
    #[serde(default = "default_wal_capacity")]
    pub wal_capacity: usize,
    #[serde(default = "default_success_retention")]
    pub success_log_retention_secs: u64,
    #[serde(default = "default_error_retention")]
    pub error_log_retention_secs: u64,
    pub log_dir: std::path::PathBuf,
    pub input_dir: std::path::PathBuf,
    pub output_dir: std::path::PathBuf,
}

fn default_wal_capacity() -> usize {
    1000
}
fn default_success_retention() -> u64 {
    3600
}
fn default_error_retention() -> u64 {
    86400 * 7
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LoadedPipelineConfig {
    pub playbook: LoadedPlaybook,
    pub wal_capacity: usize,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
}

impl PipelineConfig {
    pub async fn load(self, cache: &CacheManager) -> Result<LoadedPipelineConfig, WasmError> {
        let loaded_playbook = cache
            .load_playbook(self.playbook, self.log_dir, self.input_dir, self.output_dir)
            .await?;
        Ok(LoadedPipelineConfig {
            playbook: loaded_playbook,
            wal_capacity: self.wal_capacity,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
        })
    }
}

impl LoadedPipelineConfig {
    pub fn factory(&self) -> Result<PipelineFactory, PipelineError> {
        Ok(PipelineFactory {
            factory: PyroFactory::from_playbook(&self.playbook).map_err(PipelineError::Wasm)?,
            wal_capacity: self.wal_capacity,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            log_dir: self.playbook.log_dir.clone(),
            input_dir: self.playbook.input_dir.clone(),
            output_dir: self.playbook.output_dir.clone(),
        })
    }
}

// =============================================================================
// Runtime structures
// =============================================================================

/// A loaded pipeline definition for a single playbook.
pub struct PipelineFactory {
    pub factory: PyroFactory,
    pub log_dir: std::path::PathBuf,
    pub input_dir: std::path::PathBuf,
    pub output_dir: std::path::PathBuf,
    pub wal_capacity: usize,
    pub success_log_retention_secs: u64,
    pub error_log_retention_secs: u64,
}

// =============================================================================
// Loading
// =============================================================================

impl PipelineFactory {
    /// Build a pipeline from a fully-loaded `PipelineFactory`.
    ///
    /// Compiles and instantiates the single wasm module, configuring it with WAL.
    pub async fn build(&self) -> Result<Pipeline, PipelineError> {
        tracing::debug!("Building wasm module for playbook");
        let instance = self.factory.instantiate().await?;
        let input_schema = self.factory.spec().func.input.clone();
        let output_schema = self.factory.spec().func.output.clone();

        Ok(Pipeline {
            step: instance,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            log_manager: LogWal::open(self.log_dir.clone(), self.wal_capacity)
                .await
                .map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to make the log wal").with_source(io),
                    )
                })?,
            input_manager: super::data::DataManager::new(self.input_dir.clone(), input_schema),
            output_manager: super::data::DataManager::new(self.output_dir.clone(), output_schema),
        })
    }

    /// Build a session pipeline from a fully-loaded `PipelineFactory`.
    ///
    /// Compiles and instantiates the single wasm module, configuring it with WAL.
    pub async fn build_session(&self) -> Result<SessionPipeline, PipelineError> {
        tracing::debug!("Building wasm module for playbook");
        let instance = self.factory.instantiate().await?;
        let input_schema = self.factory.spec().func.input.clone();
        let output_schema_from_func = self.factory.spec().func.output.clone();

        // Standard session list elements will have either an input row (with the "input" field)
        // or an output row (with the output fields from the guest function).
        // To support both while satisfying the validation schema, we define a unified group
        // schema containing the last input field (which represents "input") and all output fields,
        // with all of them marked as nullable (nullable = true).
        // If the Wasm output field has an empty Group type (which occurs when SessionResponse<T> ok type
        // is resolved as an unknown struct/enum), we fallback to the type of the last input field.
        let last_input_field = input_schema.fields.last().cloned();
        let last_input_type = last_input_field
            .as_ref()
            .map(|f| f.data_type.clone())
            .unwrap_or(crate::format::value::PyroType::Null);

        let mut group_fields = Vec::new();
        if let Some(mut f) = last_input_field {
            f.nullable = true;
            group_fields.push(f);
        }
        for mut out_field in output_schema_from_func.fields.iter().cloned() {
            out_field.nullable = true;
            if let crate::format::value::PyroType::Group(ref fields) = out_field.data_type {
                if fields.is_empty() {
                    out_field.data_type = last_input_type.clone();
                }
            }
            group_fields.push(out_field);
        }

        let session_type = crate::format::value::PyroType::List(
            Box::new(crate::format::value::PyroType::Group(
                std::borrow::Cow::Owned(group_fields),
            )),
            false,
        );
        let output_schema =
            crate::format::value::PyroSchema::new(vec![crate::format::value::PyroField::new(
                "session",
                session_type,
                false,
            )]);

        Ok(SessionPipeline {
            step: instance,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            log_manager: LogWal::open(self.log_dir.clone(), self.wal_capacity)
                .await
                .map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to make the log wal").with_source(io),
                    )
                })?,
            output_manager: super::data::DataManager::new(self.output_dir.clone(), output_schema),
            log_dir: self.log_dir.clone(),
            output_dir: self.output_dir.clone(),
            wal_capacity: self.wal_capacity,
            active_sessions: std::collections::HashMap::new(),
        })
    }

    /// Build a session pipeline from a fully-loaded `PipelineFactory`.
    ///
    /// Compiles and instantiates the single wasm module, configuring it with WAL.
    pub async fn build_session_diff(&self) -> Result<SessionDiffPipeline, PipelineError> {
        tracing::debug!("Building wasm module for playbook");
        let instance = self.factory.instantiate().await?;
        let input_schema = self.factory.spec().func.input.clone();
        let output_schema = self.factory.spec().func.output.clone();

        // In Session Diff, the "inputs" list group type only contains the "input" field
        // (which is the last field of input_schema.fields), marked as nullable.
        let mut input_group_fields = Vec::new();
        if let Some(mut last_input_field) = input_schema.fields.last().cloned() {
            last_input_field.nullable = true;
            input_group_fields.push(last_input_field);
        }

        let last_input_type = input_schema
            .fields
            .last()
            .map(|f| f.data_type.clone())
            .unwrap_or(crate::format::value::PyroType::Null);
        let mut output_group_fields = Vec::new();
        for mut out_field in output_schema.fields.iter().cloned() {
            if let crate::format::value::PyroType::Group(ref fields) = out_field.data_type {
                if fields.is_empty() {
                    out_field.data_type = last_input_type.clone();
                }
            }
            output_group_fields.push(out_field);
        }

        let inputs_type = crate::format::value::PyroType::List(
            Box::new(crate::format::value::PyroType::Group(
                std::borrow::Cow::Owned(input_group_fields),
            )),
            false,
        );
        let outputs_type = crate::format::value::PyroType::List(
            Box::new(crate::format::value::PyroType::Group(
                std::borrow::Cow::Owned(output_group_fields),
            )),
            false,
        );
        let overall_output_schema = crate::format::value::PyroSchema::new(vec![
            crate::format::value::PyroField::new("inputs", inputs_type, false),
            crate::format::value::PyroField::new("outputs", outputs_type, false),
        ]);

        Ok(SessionDiffPipeline {
            step: instance,
            success_log_retention_secs: self.success_log_retention_secs,
            error_log_retention_secs: self.error_log_retention_secs,
            log_manager: LogWal::open(self.log_dir.clone(), self.wal_capacity)
                .await
                .map_err(|io| {
                    PyroError::local_io(
                        CapturedError::new("Unable to make the log wal").with_source(io),
                    )
                })?,
            output_manager: super::data::DataManager::new(
                self.output_dir.clone(),
                overall_output_schema,
            ),
            log_dir: self.log_dir.clone(),
            output_dir: self.output_dir.clone(),
            wal_capacity: self.wal_capacity,
            active_sessions: std::collections::HashMap::new(),
        })
    }
}
