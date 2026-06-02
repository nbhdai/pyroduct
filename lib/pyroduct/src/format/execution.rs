use std::collections::HashMap;

use crate::{CapturedError, PyroRow};

#[derive(Debug, Clone, PartialEq)]
pub struct PyroLogs {
    pub module_logs: Vec<String>,
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

// =============================================================================
// ExecutionRecord
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct PyroSuccess {
    pub row_index: u32,
    pub row: PyroRow<'static>,
    pub logs: PyroLogs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PyroFailure {
    pub row_index: u32,
    pub result: Result<CapturedError, String>,
    pub logs: PyroLogs,
}

/// The type of session response returned by `PyroInstance::call_session()`.
#[derive(Debug, PartialEq)]
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

