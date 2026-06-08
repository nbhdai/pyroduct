use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{CapturedError, PyroRow};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PyroLogs {
    pub module_logs: Vec<String>,
    #[serde(
        serialize_with = "serialize_cap_logs",
        deserialize_with = "deserialize_cap_logs"
    )]
    pub capability_logs: HashMap<(String, String), Vec<String>>,
}

impl PyroLogs {
    pub fn empty() -> Self {
        PyroLogs {
            module_logs: Vec::new(),
            capability_logs: HashMap::new(),
        }
    }
}

pub fn serialize_cap_logs<S>(
    logs: &HashMap<(String, String), Vec<String>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let list: Vec<(&(String, String), &Vec<String>)> = logs.iter().collect();
    list.serialize(serializer)
}

pub fn deserialize_cap_logs<'de, D>(
    deserializer: D,
) -> Result<HashMap<(String, String), Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let list = Vec::<((String, String), Vec<String>)>::deserialize(deserializer)?;
    Ok(list.into_iter().collect())
}

// =============================================================================
// ExecutionRecord
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PyroSuccess {
    pub row_index: u32,
    pub row: PyroRow<'static>,
    pub logs: PyroLogs,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PyroFailure {
    pub row_index: u32,
    pub result: Result<CapturedError, String>,
    pub logs: PyroLogs,
}

/// The type of session response returned by `PyroInstance::call_session()`.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum SessionResult {
    /// The session should continue. Contains the output row.
    Continue {
        result: PyroRow<'static>,
        session_id: u32,
        logs: PyroLogs,
    },
    /// The session is ending normally. Contains the final output row.
    End {
        result: PyroRow<'static>,
        session_id: u32,
        logs: PyroLogs,
    },
    /// The session has been terminated. No output row.
    Terminate { session_id: u32, logs: PyroLogs },
}

/// A trait that allows obtaining the index from any execution response.
pub trait ResponseIndex {
    fn row_index(&self) -> u32;
}

impl ResponseIndex for PyroSuccess {
    fn row_index(&self) -> u32 {
        self.row_index
    }
}

impl ResponseIndex for PyroFailure {
    fn row_index(&self) -> u32 {
        self.row_index
    }
}

impl ResponseIndex for SessionResult {
    fn row_index(&self) -> u32 {
        match self {
            SessionResult::Continue { session_id, .. } => *session_id,
            SessionResult::End { session_id, .. } => *session_id,
            SessionResult::Terminate { session_id, .. } => *session_id,
        }
    }
}

impl<S: ResponseIndex, F: ResponseIndex> ResponseIndex for Result<S, F> {
    fn row_index(&self) -> u32 {
        match self {
            Ok(s) => s.row_index(),
            Err(f) => f.row_index(),
        }
    }
}
