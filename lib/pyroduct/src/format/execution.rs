use std::collections::HashMap;

use crate::{CapturedError, PyroRow};


#[derive(Clone)]
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

#[derive(Clone)]
pub struct PyroSuccess {
    pub row: PyroRow<'static>,
    pub logs: PyroLogs,
}

#[derive(Clone)]
pub struct PyroFailure {
    pub result: Result<CapturedError, String>,
    pub logs: PyroLogs,
}
