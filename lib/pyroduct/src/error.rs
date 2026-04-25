use std::fmt;

use thiserror::Error;

use crate::{
    CapturedError,
    captured::{LibraryInfo, library},
    format::{ParseError, header::DataStatus, value::ValueError},
};

mod encoding;
mod json;

/// Whether the error occurred locally or on the remote service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorOrigin {
    /// Error occurred in this process before/after the RPC call.
    Local,
    /// Error occurred in the remote service we called.
    Remote,
}

/// The central error type for PyroVec operations.
#[derive(Error)]
pub enum PyroError {
    /// Attempt
    #[error("Incorrect parse, check and try again")]
    IncorrectParse(DataStatus),

    #[error("Bad header in an FFI context: {0}")]
    HeaderFfi(Box<CapturedError>),

    /// The remote code panicked and we captured it here
    #[error("Remote Code Panic: {0}")]
    CodePanic(Box<CapturedError>),

    /// A pyro/transport error with origin and kind.
    #[error("{origin} {kind}")]
    Pyro {
        origin: ErrorOrigin,
        kind: ErrorKind,
    },

    #[error("Bad header: {0}")]
    Header(#[from] ParseError),

    #[error("unknown status code: {0}")]
    NotFound(String),
}

impl PyroError {
    /// Create a local error of the given kind.
    pub fn local(kind: ErrorKind) -> Self {
        PyroError::Pyro {
            origin: ErrorOrigin::Local,
            kind,
        }
    }

    /// Create a remote error of the given kind.
    pub fn remote(kind: ErrorKind) -> Self {
        PyroError::Pyro {
            origin: ErrorOrigin::Remote,
            kind,
        }
    }

    /// Returns true if this error originated locally.
    pub fn is_local(&self) -> bool {
        matches!(
            self,
            PyroError::Pyro {
                origin: ErrorOrigin::Local,
                ..
            }
        )
    }

    /// Returns true if this error originated remotely.
    pub fn is_remote(&self) -> bool {
        matches!(
            self,
            PyroError::Pyro {
                origin: ErrorOrigin::Remote,
                ..
            }
        )
    }

    /// Returns true if this error originated remotely.
    pub fn library(&self) -> Option<&LibraryInfo<'static>> {
        match self {
            PyroError::IncorrectParse(_) => library(),
            PyroError::Header(_) => library(),
            PyroError::NotFound(_) => library(),
            PyroError::HeaderFfi(captured_error) => captured_error.library.as_ref(),
            PyroError::CodePanic(captured_error) => captured_error.library.as_ref(),
            PyroError::Pyro { kind, .. } => match kind {
                ErrorKind::Serialization(error_payload)
                | ErrorKind::Deserialization(error_payload)
                | ErrorKind::Validation(error_payload)
                | ErrorKind::Transport(error_payload)
                | ErrorKind::Io(error_payload)
                | ErrorKind::Utf8(error_payload) => error_payload.library.as_ref(),
                ErrorKind::InvalidHeader => library(),
                ErrorKind::LayoutError => library(),
                ErrorKind::UnexpectedEof => library(),
            },
        }
    }

    /// Returns the error kind if this is a Pyro error.
    pub fn kind(&self) -> Option<&ErrorKind> {
        match self {
            PyroError::Pyro { kind, .. } => Some(kind),
            _ => None,
        }
    }
}
// Todo: make this capture the stack and library and all that
impl From<std::io::Error> for PyroError {
    fn from(err: std::io::Error) -> Self {
        Self::local(ErrorKind::Io(CapturedError::new(err).into()))
    }
}

// Todo: make this capture the stack and library and all that
impl From<std::str::Utf8Error> for PyroError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::local(ErrorKind::Utf8(CapturedError::new(err).into()))
    }
}

impl fmt::Debug for PyroError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncorrectParse(value) => f.debug_tuple("IncorrectParse").field(value).finish(),
            Self::Header(h) => f.debug_tuple("Header").field(h).finish(),
            Self::HeaderFfi(captured) => f
                .debug_struct("HeaderFfi")
                .field("message", &captured.message)
                .field("library", &captured.library)
                .field("stack_trace", &captured.stack_trace)
                .field("file", &captured.file)
                .field("line", &captured.line)
                .field("column", &captured.column)
                .finish(),
            Self::CodePanic(arg0) => f.debug_tuple("CodePanic").field(arg0).finish(),
            Self::Pyro { origin, kind } => f
                .debug_struct("Pyro")
                .field("origin", origin)
                .field("kind", kind)
                .finish(),
            Self::NotFound(arg0) => f.debug_tuple("NotFound").field(arg0).finish(),
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

/// The specific kind of pyro/transport error that occurred.
#[derive(Debug)]
pub enum ErrorKind {
    /// Failed to serialize data.
    Serialization(Box<CapturedError>),
    /// Failed to deserialize data.
    Deserialization(Box<CapturedError>),
    /// Failed to validate archived data.
    Validation(Box<CapturedError>),
    /// Generic transport failure.
    Transport(Box<CapturedError>),
    /// I/O error.
    Io(Box<CapturedError>),
    /// UTF-8 decoding error.
    Utf8(Box<CapturedError>),
    /// Header was invalid
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
            ErrorKind::InvalidHeader => ErrorStatus::InvalidHeader,
            ErrorKind::LayoutError => ErrorStatus::LayoutError,
            ErrorKind::UnexpectedEof => ErrorStatus::UnexpectedEof,
        }
    }
}

/// The specific kind of pyro/transport error that occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStatus {
    Serialization,
    Deserialization,
    Validation,
    Transport,
    Io,
    Utf8,
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
            ErrorStatus::InvalidHeader => DataStatus::RemoteInvalidHeader,
            ErrorStatus::LayoutError => DataStatus::RemoteLayoutError,
            ErrorStatus::UnexpectedEof => DataStatus::RemoteUnexpectedEof,
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
            ErrorKind::InvalidHeader => write!(f, "invalid header"),
            ErrorKind::LayoutError => write!(f, "layout/capacity error"),
            ErrorKind::UnexpectedEof => write!(f, "unexpected end of stream"),
        }
    }
}

// Convenience constructors for common local errors
impl PyroError {
    pub fn serialization(err: impl Into<Box<CapturedError>>) -> Self {
        Self::local(ErrorKind::Serialization(err.into()))
    }

    pub fn deserialization(err: impl Into<Box<CapturedError>>) -> Self {
        Self::local(ErrorKind::Deserialization(err.into()))
    }

    pub fn validation(err: impl Into<Box<CapturedError>>) -> Self {
        Self::local(ErrorKind::Validation(err.into()))
    }

    pub fn transport(err: impl Into<Box<CapturedError>>) -> Self {
        Self::local(ErrorKind::Transport(err.into()))
    }

    pub fn remote_io(err: impl Into<Box<CapturedError>>) -> Self {
        Self::remote(ErrorKind::Io(err.into()))
    }

    pub fn remote_utf8(err: impl Into<Box<CapturedError>>) -> Self {
        Self::remote(ErrorKind::Utf8(err.into()))
    }

    pub fn layout_error() -> Self {
        Self::local(ErrorKind::LayoutError)
    }

    pub fn unexpected_eof() -> Self {
        Self::local(ErrorKind::UnexpectedEof)
    }

    pub fn null_pointer() -> Self {
        Self::Header(ParseError::NullPointer)
    }
}
