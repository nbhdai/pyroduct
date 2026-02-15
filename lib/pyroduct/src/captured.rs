// TODO: Make the "library" ALWAYS encode in the error messages and use a zerovec format.

use std::fmt::{self, Display};
use std::panic::Location;
use std::{backtrace::Backtrace, borrow::Cow};

use thiserror::Error;

use crate::PyroVec;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LibraryInfo<'a> {
    pub meta: Cow<'a, str>,
    pub name: Cow<'a, str>,
    pub version: Cow<'a, str>,
}

impl<'a> fmt::Display for LibraryInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Compact output format: name vversion (meta)
        write!(f, "lib: {} v{} ({})", self.name, self.version, self.meta)
    }
}

// Add this global static to store the identity of the currently running binary
static APP_IDENTITY: std::sync::OnceLock<(LibraryInfo<'static>, &'static str)> =
    std::sync::OnceLock::new();

/// Called by the binary's `main()` to register its identity.
/// This ensures all CapturedErrors created in this process carry this tag.
pub fn register_app_identity(info: LibraryInfo<'static>, encoded: &'static str) {
    // We ignore the result; if it's already set, we keep the original (first-one-wins)
    let _ = APP_IDENTITY.set((info, encoded));
}

pub fn library() -> Option<&'static LibraryInfo<'static>> {
    APP_IDENTITY.get().map(|l| &l.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn library_json(len: *mut usize) -> *const u8 {
    // 1. Retrieve the encoded JSON string from the global OnceLock
    // If it hasn't been set yet, we return an empty result.
    let json_str = match APP_IDENTITY.get() {
        Some((_, encoded)) => encoded,
        None => {
            if !len.is_null() {
                unsafe {
                    *len = 0;
                }
            }
            return std::ptr::null();
        }
    };

    // 2. Write the length to the C-provided pointer (if it's not null)
    if !len.is_null() {
        unsafe {
            *len = json_str.len();
        }
    }

    // 3. Return the pointer to the bytes
    json_str.as_ptr()
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Additional context or causal chain
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,

    /// Full stack trace, captured on demand
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library: Option<LibraryInfo<'static>>,
}

impl CapturedError {
    /// Creates a new CapturedError, automatically recording the file, line, and column
    /// of the caller.
    pub fn new(message: impl Display) -> Self {
        Self {
            message: message.to_string(),
            file: "unknown".to_string(),
            line: 0,
            column: 0,
            error: None,
            context: None,
            stack_trace: None,
            library: APP_IDENTITY.get().map(|l| l.0.clone()),
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
            }
            std::backtrace::BacktraceStatus::Disabled => {
                self.stack_trace =
                    Some("Backtrace captured but disabled (set RUST_BACKTRACE=1)".to_string());
            }
            _ => {
                self.stack_trace = Some("Backtrace unsupported on this platform".to_string());
            }
        }
        self
    }

    /// Attaches an underlying error (e.g., from a `Result::Err`).
    pub fn with_source<E: fmt::Display>(mut self, error: E) -> Self {
        self.error = Some(error.to_string());
        self
    }

    pub fn with_context<E: fmt::Display>(mut self, context: E) -> Self {
        self.context = Some(context.to_string());
        self
    }

    pub fn encode(&self) -> PyroVec {
        let mut vec = PyroVec::with_capacity(predict_captured_error_size(&self));
        serde_json::to_writer(&mut vec, self)
            .expect("CapturedError serialization should never fail");
        vec
    }
}

/// Predict the JSON size of a CapturedError for buffer preallocation.
///
/// This is a conservative estimate that may slightly overcount due to
/// escaped characters being rare in practice.
fn predict_captured_error_size(err: &CapturedError) -> usize {
    // Fixed JSON overhead: {"message":"","file":"","line":,"column":,"error":,"cause":}
    // Keys + colons + commas + braces + quotes ≈ 70 bytes
    // Adding a 10 byte buffer
    const FIXED_OVERHEAD: usize = 70 + 10;

    // u32 max is 10 digits
    const MAX_U32_DIGITS: usize = 10;

    let mut size = FIXED_OVERHEAD;
    size += err.message.len();
    size += err.file.len();
    size += MAX_U32_DIGITS * 2; // line + column

    match &err.error {
        Some(s) => size += s.len() + 2, // + quotes
        None => {}                      // null
    }

    match &err.context {
        Some(s) => size += s.len() + 2,
        None => {}
    }

    if let Some(lib) = &err.library {
        // Keys overhead: {"meta":"","name":"","version":"","authors":"","filename":""}
        // Approx 60 chars for keys/quotes/commas
        size += 60;
        size += lib.meta.len();
        size += lib.name.len();
        size += lib.version.len();
    }

    size
}
