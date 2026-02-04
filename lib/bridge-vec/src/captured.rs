use std::{backtrace::Backtrace, borrow::Cow};
use std::fmt;
use std::panic::Location;

use rkyv::rancor;
use thiserror::Error;

use crate::{BridgeVec, ErrorVec};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LibraryInfo<'a> {
    pub meta: Cow<'a, str>,
    pub name: Cow<'a, str>,
    pub version: Cow<'a, str>,
    pub authors: Cow<'a, str>,
    pub filename: Cow<'a, str>,
}

// Add this global static to store the identity of the currently running binary
static APP_IDENTITY: std::sync::OnceLock<LibraryInfo<'static>> = std::sync::OnceLock::new();

/// Called by the binary's `main()` to register its identity.
/// This ensures all CapturedErrors created in this process carry this tag.
pub fn register_app_identity(info: LibraryInfo<'static>) {
    // We ignore the result; if it's already set, we keep the original (first-one-wins)
    let _ = APP_IDENTITY.set(info);
}

/// Whether the error occurred locally or on the remote service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorOrigin {
    /// Error occurred in this process before/after the RPC call.
    Local,
    /// Error occurred in the remote service we called.
    Remote,
}

/// The central error type for BridgeVec operations.
#[derive(Error)]
pub enum BridgeError {
    /// The data is marked as a user-defined error (Status 1).
    #[error("user error")]
    UserError(ErrorVec),

    /// The data is marked as success but an error was expected.
    #[error("unexpected success data")]
    UserSuccess(BridgeVec),

    /// The remote code panicked and we captured it here
    #[error("Remote Code Panic: {0}")]
    CodePanic(Box<CapturedError>),

    /// A bridge/transport error with origin and kind.
    #[error("{origin} {kind}")]
    Bridge {
        origin: ErrorOrigin,
        kind: ErrorKind,
    },

    /// Unknown status code.
    #[error("unknown status code: {0}")]
    UnknownStatus(u8, BridgeVec),
}

impl BridgeError {
    /// Create a local error of the given kind.
    pub fn local(kind: ErrorKind) -> Self {
        BridgeError::Bridge {
            origin: ErrorOrigin::Local,
            kind,
        }
    }

    /// Create a remote error of the given kind.
    pub fn remote(kind: ErrorKind) -> Self {
        BridgeError::Bridge {
            origin: ErrorOrigin::Remote,
            kind,
        }
    }

    /// Returns true if this error originated locally.
    pub fn is_local(&self) -> bool {
        matches!(
            self,
            BridgeError::Bridge {
                origin: ErrorOrigin::Local,
                ..
            }
        )
    }

    /// Returns true if this error originated remotely.
    pub fn is_remote(&self) -> bool {
        matches!(
            self,
            BridgeError::Bridge {
                origin: ErrorOrigin::Remote,
                ..
            }
        )
    }

    /// Returns the error kind if this is a Bridge error.
    pub fn kind(&self) -> Option<&ErrorKind> {
        match self {
            BridgeError::Bridge { kind, .. } => Some(kind),
            _ => None,
        }
    }
}

impl From<std::io::Error> for BridgeError {
    fn from(err: std::io::Error) -> Self {
        Self::local(ErrorKind::Io(IoPayload::Error(err)))
    }
}

impl From<std::str::Utf8Error> for BridgeError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::local(ErrorKind::Utf8(Utf8Payload::Error(err)))
    }
}

impl fmt::Debug for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserError(_) => f.debug_tuple("UserError").finish(),
            Self::UserSuccess(_) => f.debug_tuple("UserSuccess").finish(),
            Self::CodePanic(arg0) => f.debug_tuple("CodePanic").field(arg0).finish(),
            Self::Bridge { origin, kind } => f
                .debug_struct("Bridge")
                .field("origin", origin)
                .field("kind", kind)
                .finish(),
            Self::UnknownStatus(arg0, _) => f.debug_tuple("UnknownStatus").field(arg0).finish(),
        }
    }
}

impl fmt::Display for ErrorOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorOrigin::Local => write!(f, "local"),
            ErrorOrigin::Remote => write!(f, "remote"),
        }
    }
}

#[derive(Debug)]
pub enum IoPayload {
    Error(std::io::Error),
    Captured(Box<CapturedError>),
}

impl fmt::Display for IoPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IoPayload::Error(e) => write!(f, "{}", e),
            IoPayload::Captured(e) => write!(f, "{}", e),
        }
    }
}

impl From<IoPayload> for Box<CapturedError> {
    fn from(value: IoPayload) -> Self {
        match value {
            IoPayload::Error(error) => CapturedError::new(format!("Io Error: {error}")).into(),
            IoPayload::Captured(captured_error) => captured_error,
        }
    }
}

impl From<&IoPayload> for Box<CapturedError> {
    fn from(value: &IoPayload) -> Self {
        match value {
            IoPayload::Error(error) => CapturedError::new(format!("Io Error: {error}")).into(),
            IoPayload::Captured(captured_error) => captured_error.clone(),
        }
    }
}

#[derive(Debug)]
pub enum Utf8Payload {
    Error(std::str::Utf8Error),
    Captured(Box<CapturedError>),
}

impl fmt::Display for Utf8Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Utf8Payload::Error(e) => write!(f, "{}", e),
            Utf8Payload::Captured(e) => write!(f, "{}", e),
        }
    }
}

impl From<Utf8Payload> for Box<CapturedError> {
    fn from(value: Utf8Payload) -> Self {
        match value {
            Utf8Payload::Error(error) => CapturedError::new(format!("Uft8 Error: {error}")).into(),
            Utf8Payload::Captured(captured_error) => captured_error,
        }
    }
}

impl From<&Utf8Payload> for Box<CapturedError> {
    fn from(value: &Utf8Payload) -> Self {
        match value {
            Utf8Payload::Error(error) => CapturedError::new(format!("Uft8 Error: {error}")).into(),
            Utf8Payload::Captured(captured_error) => captured_error.clone(),
        }
    }
}

/// The specific kind of bridge/transport error that occurred.
#[derive(Debug)]
pub enum ErrorKind {
    /// Failed to serialize data.
    Serialization(ErrorPayload),
    /// Failed to deserialize data.
    Deserialization(ErrorPayload),
    /// Failed to validate archived data.
    Validation(ErrorPayload),
    /// Generic transport failure.
    Transport(ErrorPayload),
    /// I/O error.
    Io(IoPayload),
    /// UTF-8 decoding error.
    Utf8(Utf8Payload),
    /// Pointer was null.
    NullPointer,
    /// Pointer was not properly aligned.
    MisalignedPointer,
    /// Magic header bytes were invalid.
    InvalidHeader,
    /// Layout/capacity calculation failed.
    LayoutError,
    /// Stream ended unexpectedly.
    UnexpectedEof,
}

impl ErrorKind {
    pub fn to_status(&self) -> ErrorStatus {
        match self {
            ErrorKind::Serialization(_) => ErrorStatus::Serialization,
            ErrorKind::Deserialization(_) => ErrorStatus::Deserialization,
            ErrorKind::Validation(_) => ErrorStatus::Validation,
            ErrorKind::Transport(_) => ErrorStatus::Transport,
            ErrorKind::Io(_) => ErrorStatus::Io,
            ErrorKind::Utf8(_) => ErrorStatus::Utf8,
            ErrorKind::NullPointer => ErrorStatus::NullPointer,
            ErrorKind::MisalignedPointer => ErrorStatus::MisalignedPointer,
            ErrorKind::InvalidHeader => ErrorStatus::InvalidHeader,
            ErrorKind::LayoutError => ErrorStatus::LayoutError,
            ErrorKind::UnexpectedEof => ErrorStatus::UnexpectedEof,
        }
    }
}

/// The specific kind of bridge/transport error that occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStatus {
    Serialization,
    Deserialization,
    Validation,
    Transport,
    Io,
    Utf8,
    NullPointer,
    MisalignedPointer,
    InvalidHeader,
    LayoutError,
    UnexpectedEof,
}

impl ErrorStatus {
    pub fn to_local(&self) -> DataStatus {
        match self {
            ErrorStatus::Serialization => DataStatus::LocalSerialization,
            ErrorStatus::Deserialization => DataStatus::LocalDeserialization,
            ErrorStatus::Validation => DataStatus::LocalValidation,
            ErrorStatus::Transport => DataStatus::LocalTransport,
            ErrorStatus::Io => DataStatus::LocalIo,
            ErrorStatus::Utf8 => DataStatus::LocalUtf8,
            ErrorStatus::NullPointer => DataStatus::LocalNullPointer,
            ErrorStatus::MisalignedPointer => DataStatus::LocalMisalignedPointer,
            ErrorStatus::InvalidHeader => DataStatus::LocalInvalidHeader,
            ErrorStatus::LayoutError => DataStatus::LocalLayoutError,
            ErrorStatus::UnexpectedEof => DataStatus::LocalUnexpectedEof,
        }
    }

    pub fn to_remote(&self) -> DataStatus {
        match self {
            ErrorStatus::Serialization => DataStatus::RemoteSerialization,
            ErrorStatus::Deserialization => DataStatus::RemoteDeserialization,
            ErrorStatus::Validation => DataStatus::RemoteValidation,
            ErrorStatus::Transport => DataStatus::RemoteTransport,
            ErrorStatus::Io => DataStatus::RemoteIo,
            ErrorStatus::Utf8 => DataStatus::RemoteUtf8,
            ErrorStatus::NullPointer => DataStatus::RemoteNullPointer,
            ErrorStatus::MisalignedPointer => DataStatus::RemoteMisalignedPointer,
            ErrorStatus::InvalidHeader => DataStatus::RemoteInvalidHeader,
            ErrorStatus::LayoutError => DataStatus::RemoteLayoutError,
            ErrorStatus::UnexpectedEof => DataStatus::RemoteUnexpectedEof,
        }
    }
}

/// Status codes located at Offset 0x0F in the header.
///
/// - **0-1**: User Logic (Success/Failure)
/// - **3**: Caught Remote Error (Panic/Crash)
/// - **4-99**: Reserved
/// - **100-149**: Reserved for Local/Proxy errors
/// - **150-199**: Remote Execution & Memory Safety errors
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataStatus {
    /// The payload is a valid `rkyv` archived `T`.
    ValidData = 0,
    /// The payload is a valid `rkyv` archived `E` (User Logic Error).
    UserError = 1,
    /// The remote code panicked or failed in an unhandled way.
    /// Payload is `CapturedError` as JSON.
    CodeError = 3,

    // --- Local Errors (100-149) ---
    // Used when the error occurs locally before leaving the bridge.
    LocalSerialization = 101,
    LocalDeserialization = 102,
    LocalValidation = 103,
    LocalTransport = 104,
    LocalIo = 105,
    LocalUtf8 = 106,
    LocalNullPointer = 107,
    LocalMisalignedPointer = 108,
    LocalInvalidHeader = 109,
    LocalLayoutError = 110,
    LocalUnexpectedEof = 111,

    // --- Remote Errors (150-199) ---
    // Used when the error occurs locally before leaving the bridge.
    RemoteSerialization = 151,
    RemoteDeserialization = 152,
    RemoteValidation = 153,
    RemoteTransport = 154,
    RemoteIo = 155,
    RemoteUtf8 = 156,
    RemoteNullPointer = 157,
    RemoteMisalignedPointer = 158,
    RemoteInvalidHeader = 159,
    RemoteLayoutError = 160,
    RemoteUnexpectedEof = 161,
}

/// Payload for errors - either a local rancor error or a captured remote error.
#[derive(Debug)]
pub enum ErrorPayload {
    /// Local rkyv/rancor error.
    Rancor(rancor::Error),
    /// Captured error with stack trace (from panics or remote).
    Captured(Box<CapturedError>),
    /// Simple string message.
    Message(String),
}

impl fmt::Display for ErrorPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorPayload::Rancor(e) => write!(f, "{}", e),
            ErrorPayload::Captured(e) => write!(f, "{}", e),
            ErrorPayload::Message(s) => write!(f, "{}", s),
        }
    }
}

impl From<ErrorPayload> for Box<CapturedError> {
    fn from(value: ErrorPayload) -> Self {
        match value {
            ErrorPayload::Rancor(error) => {
                CapturedError::new(format!("Rancor Error: {error}")).into()
            }
            ErrorPayload::Captured(captured_error) => captured_error,
            ErrorPayload::Message(error) => {
                CapturedError::new(format!("Error Message: {error}")).into()
            }
        }
    }
}

impl From<&ErrorPayload> for Box<CapturedError> {
    fn from(value: &ErrorPayload) -> Self {
        match value {
            ErrorPayload::Rancor(error) => {
                CapturedError::new(format!("Rancor Error: {error}")).into()
            }
            ErrorPayload::Captured(captured_error) => captured_error.clone(),
            ErrorPayload::Message(error) => {
                CapturedError::new(format!("Error Message: {error}")).into()
            }
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Serialization(e) => write!(f, "serialization error: {}", e),
            ErrorKind::Deserialization(e) => write!(f, "deserialization error: {}", e),
            ErrorKind::Validation(e) => write!(f, "validation error: {}", e),
            ErrorKind::Transport(e) => write!(f, "transport error: {}", e),
            ErrorKind::Io(e) => write!(f, "I/O error: {}", e),
            ErrorKind::Utf8(e) => write!(f, "UTF-8 error: {}", e),
            ErrorKind::NullPointer => write!(f, "null pointer"),
            ErrorKind::MisalignedPointer => write!(f, "misaligned pointer"),
            ErrorKind::InvalidHeader => write!(f, "invalid magic header"),
            ErrorKind::LayoutError => write!(f, "layout/capacity error"),
            ErrorKind::UnexpectedEof => write!(f, "unexpected end of stream"),
        }
    }
}

// Convenience constructors for common local errors
impl BridgeError {
    pub fn serialization(err: rancor::Error) -> Self {
        Self::local(ErrorKind::Serialization(ErrorPayload::Rancor(err)))
    }

    pub fn serialization_panic(err: Box<CapturedError>) -> Self {
        Self::local(ErrorKind::Serialization(ErrorPayload::Captured(err)))
    }

    pub fn deserialization(err: rancor::Error) -> Self {
        Self::local(ErrorKind::Deserialization(ErrorPayload::Rancor(err)))
    }

    pub fn deserialization_panic(err: Box<CapturedError>) -> Self {
        Self::local(ErrorKind::Deserialization(ErrorPayload::Captured(err)))
    }

    pub fn validation(err: rancor::Error) -> Self {
        Self::local(ErrorKind::Validation(ErrorPayload::Rancor(err)))
    }

    pub fn validation_panic(err: Box<CapturedError>) -> Self {
        Self::local(ErrorKind::Validation(ErrorPayload::Captured(err)))
    }

    pub fn transport(msg: String) -> Self {
        Self::local(ErrorKind::Transport(ErrorPayload::Message(msg)))
    }

    pub fn remote_io(err: Box<CapturedError>) -> Self {
        Self::remote(ErrorKind::Io(IoPayload::Captured(err)))
    }

    pub fn remote_utf8(err: Box<CapturedError>) -> Self {
        Self::remote(ErrorKind::Utf8(Utf8Payload::Captured(err)))
    }

    pub fn null_pointer() -> Self {
        Self::local(ErrorKind::NullPointer)
    }

    pub fn misaligned_pointer() -> Self {
        Self::local(ErrorKind::MisalignedPointer)
    }

    pub fn invalid_header() -> Self {
        Self::local(ErrorKind::InvalidHeader)
    }

    pub fn layout_error() -> Self {
        Self::local(ErrorKind::LayoutError)
    }

    pub fn unexpected_eof() -> Self {
        Self::local(ErrorKind::UnexpectedEof)
    }
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
    pub cause: Option<String>,

    /// Full stack trace, captured on demand
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library: Option<LibraryInfo<'static>>,
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
            library: APP_IDENTITY.get().cloned(),
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
    pub fn with_source<E: fmt::Display>(mut self, error: &E) -> Self {
        self.error = Some(error.to_string());
        self
    }

    pub(crate) fn encode(&self) -> BridgeVec {
        let mut vec = BridgeVec::with_capacity(predict_captured_error_size(&self));
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
        None => {},              // null
    }

    match &err.cause {
        Some(s) => size += s.len() + 2,
        None => {},
    }

    if let Some(lib) = &err.library {
        // Keys overhead: {"meta":"","name":"","version":"","authors":"","filename":""}
        // Approx 60 chars for keys/quotes/commas
        size += 60; 
        size += lib.meta.len();
        size += lib.name.len();
        size += lib.version.len();
        size += lib.authors.len();
        size += lib.filename.len();
    }

    size
}
