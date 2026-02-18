mod wasm_side;


pub use wasm_side::{Client, wasm_row_main};
pub type ModuleResult<T> = anyhow::Result<T>;

#[cfg(test)]
pub use wasm_side::_test_reinsert_input;

use crate::captured::CapturedError;

impl From<anyhow::Error> for CapturedError {
    fn from(err: anyhow::Error) -> Self {
        let mut captured = CapturedError::new(err.to_string());

        let sources: Vec<String> = err.chain().skip(1).map(|e| e.to_string()).collect();

        if !sources.is_empty() {
            captured.error = Some(sources.join(": "));
        }
        let backtrace = err.backtrace();
        let bt_str = backtrace.to_string();

        if !bt_str.is_empty() && bt_str != "Disabled Backtrace" {
            captured.stack_trace = Some(bt_str);
        }

        captured
    }
}

impl From<anyhow::Error> for Box<CapturedError> {
    fn from(err: anyhow::Error) -> Self {
        Box::new(err.into())
    }
}
