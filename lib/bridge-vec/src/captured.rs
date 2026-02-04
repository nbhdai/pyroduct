use std::backtrace::Backtrace;
use std::panic::Location;
use std::fmt;

use thiserror::Error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Error)]
#[error("Panic at {file}:{line}:{column} - {message}{}", 
    .error.as_ref().map(|e| format!(" (Error: {e})")).unwrap_or_default()
)]
pub struct CapturedError {
    pub message: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    
    /// The stringified source error (e.g. "permission denied")
    pub error: Option<String>, 
    
    /// Additional context or causal chain
    pub cause: Option<String>, 
    
    /// Full stack trace, captured on demand
    pub stack_trace: Option<String>,
}

impl CapturedError {
    /// Creates a new CapturedError, automatically recording the file, line, and column
    /// of the caller.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            file: "unknown".to_string(),
            line: 0,
            column: 0,
            error: None,
            cause: None,
            stack_trace: None,
        }
    }

    /// Useful when wrapping an error that already has location data 
    /// (e.g., from a parser or a lexer).
    pub fn with_location(mut self, location: &Location<'_>) -> Self {
        self.file = location.file().to_string();
        self.line = location.line();
        self.column = location.column();
        self
    }

    /// Captures the current stack trace and attaches it to the error.
    /// 
    /// Note: This relies on `std::backtrace`. For best results, run with `RUST_BACKTRACE=1`.
    pub fn with_backtrace(mut self, backtrace: Backtrace) -> Self {
        // We force capture to string immediately because Backtrace is not Serializable
        match backtrace.status() {
            std::backtrace::BacktraceStatus::Captured => {
                 self.stack_trace = Some(format!("{}", backtrace));
            },
            std::backtrace::BacktraceStatus::Disabled => {
                 self.stack_trace = Some("Backtrace captured but disabled (set RUST_BACKTRACE=1)".to_string());
            },
            _ => {
                 self.stack_trace = Some("Backtrace unsupported on this platform".to_string());
            }
        }
        self
    }

    /// Attaches an underlying error (e.g., from a `Result::Err`).
    pub fn with_source<E: fmt::Display>(mut self, error: &E) -> Self {
        self.error = Some(error.to_string());
        self
    }
}
