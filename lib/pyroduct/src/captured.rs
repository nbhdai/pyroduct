// TODO: Make the "library" ALWAYS encode in the error messages and use a zerovec format.

use std::fmt::{self, Display};
use std::panic::Location;
use std::{backtrace::Backtrace, borrow::Cow};

use thiserror::Error;

use crate::format::PyroVec;
use crate::format::header::PyroHeaderMut;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// # Safety
///
/// Len needs to be valid for the full call, returns a pointer to the data.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn library_json(len: *mut usize) -> *const u8 {
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Error)]
#[error("Error at {file}:{line}:{column} - {message}{}", 
    .error.as_ref().map(|e| format!(" (Error: {e})")).unwrap_or_default()
)]
pub struct CapturedErrorInner {
    pub message: String,
    pub file: String,
    pub line: u32,
    pub column: u32,

    /// The stringified source error (e.g. "permission denied")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Full stack trace, captured on demand
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library: Option<LibraryInfo<'static>>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Error)]
#[serde(transparent)]
#[error("{0}")]
pub struct CapturedError(pub Box<CapturedErrorInner>);

impl std::ops::Deref for CapturedError {
    type Target = CapturedErrorInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for CapturedError {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<CapturedErrorInner> for CapturedError {
    fn from(inner: CapturedErrorInner) -> Self {
        Self(Box::new(inner))
    }
}

impl CapturedError {
    /// Creates a new CapturedError, automatically recording the file, line, and column
    /// of the caller.
    pub fn new(message: impl Display) -> Self {
        Self(Box::new(CapturedErrorInner {
            message: message.to_string(),
            file: "unknown".to_string(),
            line: 0,
            column: 0,
            error: None,
            stack_trace: None,
            library: APP_IDENTITY.get().map(|l| l.0.clone()),
        }))
    }

    /// Useful when wrapping an error that already has location data
    /// (e.g., from a parser or a lexer).
    pub fn with_location(mut self, location: &Location<'_>) -> Self {
        self.0.file = location.file().to_string();
        self.0.line = location.line();
        self.0.column = location.column();
        self
    }

    /// Captures the current stack trace and attaches it to the error.
    ///
    /// Note: This relies on `std::backtrace`. For best results, run with `RUST_BACKTRACE=1`.
    pub fn with_backtrace(mut self, backtrace: Backtrace) -> Self {
        // We force capture to string immediately because Backtrace is not Serializable
        match backtrace.status() {
            std::backtrace::BacktraceStatus::Captured => {
                self.0.stack_trace = Some(format!("{}", backtrace));
            }
            std::backtrace::BacktraceStatus::Disabled => {
                self.0.stack_trace =
                    Some("Backtrace captured but disabled (set RUST_BACKTRACE=1)".to_string());
            }
            _ => {
                self.0.stack_trace = Some("Backtrace unsupported on this platform".to_string());
            }
        }
        self
    }

    /// Attaches an underlying error (e.g., from a `Result::Err`).
    pub fn with_source<E: fmt::Display>(mut self, error: E) -> Self {
        self.0.error = Some(error.to_string());
        self
    }

    pub fn encode(&self) -> PyroVec {
        let mut vec = PyroVec::with_capacity(predict_captured_error_size(&self.0));
        serde_json::to_writer(&mut vec, &self.0)
            .expect("CapturedError serialization should never fail");
        vec.set_status(crate::format::header::DataStatus::CodeError);
        vec
    }
}

/// Predict the JSON size of a CapturedError for buffer preallocation.
///
/// This is a conservative estimate that may slightly overcount due to
/// escaped characters being rare in practice.
fn predict_captured_error_size(err: &CapturedErrorInner) -> usize {
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

    if let Some(s) = &err.error {
        size += s.len() + 2
    } // + quotes

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

/// Bails out with a `CapturedError`, automatically capturing the location and backtrace.
///
/// # Examples
///
/// ```rust,ignore
/// bail!("Something went wrong: {}", id);
/// bail!(io_err, "Failed to read file");
/// ```
#[macro_export]
macro_rules! capture {
    ($msg:literal $(,)?) => {
        $crate::CapturedError::new($msg)
            .with_location(::std::panic::Location::caller())
            .with_backtrace(::std::backtrace::Backtrace::capture())
    };
    ($err:expr, $msg:literal $(,)?) => {
        $crate::CapturedError::new($msg)
            .with_source($err)
            .with_location(::std::panic::Location::caller())
            .with_backtrace(::std::backtrace::Backtrace::capture())
    };
    ($err:expr, $fmt:literal, $($arg:tt)*) => {
        $crate::CapturedError::new(format!($fmt, $($arg)*))
            .with_source($err)
            .with_location(::std::panic::Location::caller())
            .with_backtrace(::std::backtrace::Backtrace::capture())
    };
    ($fmt:literal, $($arg:tt)*) => {
        $crate::CapturedError::new(format!($fmt, $($arg)*))
            .with_location(::std::panic::Location::caller())
            .with_backtrace(::std::backtrace::Backtrace::capture())
    };
}

/// Bails out with a `CapturedError`, automatically capturing the location and backtrace.
///
/// # Examples
///
/// ```rust,ignore
/// bail!("Something went wrong: {}", id);
/// bail!(io_err, "Failed to read file");
/// ```
#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::CapturedError::new($msg)
            .with_location(::std::panic::Location::caller())
            .with_backtrace(::std::backtrace::Backtrace::capture()))
    };
    ($err:expr, $msg:literal $(,)?) => {
        return Err($crate::CapturedError::new($msg)
            .with_source($err)
            .with_location(::std::panic::Location::caller())
            .with_backtrace(::std::backtrace::Backtrace::capture()))
    };
    ($err:expr, $fmt:literal, $($arg:tt)*) => {
        return Err($crate::CapturedError::new(format!($fmt, $($arg)*))
            .with_source($err)
            .with_location(::std::panic::Location::caller())
            .with_backtrace(::std::backtrace::Backtrace::capture()))
    };
    ($fmt:literal, $($arg:tt)*) => {
        return Err($crate::CapturedError::new(format!($fmt, $($arg)*))
            .with_location(::std::panic::Location::caller())
            .with_backtrace(::std::backtrace::Backtrace::capture()))
    };
}

/// Extension trait to convert `Result<T, E>` and `Option<T>` into `Result<T, CapturedError>` with context.
#[allow(clippy::result_large_err)]
pub trait Capture<T> {
    /// Alias for `context`.
    #[track_caller]
    fn capture<C>(self, context: C) -> Result<T, CapturedError>
    where
        C: Display;

    /// Alias for `with_context`.
    #[track_caller]
    fn with_capture<C, F>(self, f: F) -> Result<T, CapturedError>
    where
        C: Display,
        F: FnOnce() -> C;
}

impl<T, E> Capture<T> for Result<T, E>
where
    E: std::error::Error,
{
    #[track_caller]
    fn capture<C>(self, context: C) -> Result<T, CapturedError>
    where
        C: Display,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(CapturedError::new(context)
                .with_source(e)
                .with_location(std::panic::Location::caller())
                .with_backtrace(std::backtrace::Backtrace::capture())),
        }
    }

    #[track_caller]
    fn with_capture<C, F>(self, f: F) -> Result<T, CapturedError>
    where
        C: Display,
        F: FnOnce() -> C,
    {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(CapturedError::new(f())
                .with_source(e)
                .with_location(std::panic::Location::caller())
                .with_backtrace(std::backtrace::Backtrace::capture())),
        }
    }
}

impl<T> Capture<T> for Option<T> {
    #[track_caller]
    fn capture<C>(self, context: C) -> Result<T, CapturedError>
    where
        C: Display,
    {
        match self {
            Some(t) => Ok(t),
            None => Err(CapturedError::new(context)
                .with_location(std::panic::Location::caller())
                .with_backtrace(std::backtrace::Backtrace::capture())),
        }
    }

    #[track_caller]
    fn with_capture<C, F>(self, f: F) -> Result<T, CapturedError>
    where
        C: Display,
        F: FnOnce() -> C,
    {
        match self {
            Some(t) => Ok(t),
            None => Err(CapturedError::new(f())
                .with_location(std::panic::Location::caller())
                .with_backtrace(std::backtrace::Backtrace::capture())),
        }
    }
}
