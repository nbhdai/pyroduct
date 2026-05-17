use std::collections::HashMap;

use crate::{CapturedError, PyroRow};

#[derive(Debug, Clone)]
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

#[derive(Clone)]
pub enum ExecutionRecord {
    Success {
        row_index: usize,
        input: PyroRow<'static>,
        success: PyroRow<'static>,
        logs: PyroLogs,
    },
    Failure {
        row_index: usize,
        input: PyroRow<'static>,
        failure: Result<CapturedError, String>,
        logs: PyroLogs,
    },
}

impl ExecutionRecord {
    pub fn row_index(&self) -> usize {
        match self {
            ExecutionRecord::Success { row_index, .. } => *row_index,
            ExecutionRecord::Failure { row_index, .. } => *row_index,
        }
    }

    pub fn row(&self) -> Option<&PyroRow<'static>> {
        match self {
            ExecutionRecord::Success { success, .. } => Some(success),
            ExecutionRecord::Failure { input, .. } => Some(input),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PyroSuccess {
    pub row: PyroRow<'static>,
    pub logs: PyroLogs,
}

#[derive(Debug, Clone)]
pub struct PyroFailure {
    pub result: Result<CapturedError, String>,
    pub logs: PyroLogs,
}
