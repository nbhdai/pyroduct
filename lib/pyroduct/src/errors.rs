// crates/pyroduct/src/errors.rs

use std::{
    fmt,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct FfiPanic {
    pub message: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug)]
pub enum Phase {
    /// Detected during capability init
    Init,
    /// Detected during capability reset
    Reset,
    /// Detected during capability state retrieval
    State,
    /// Detected during capability input retrieval
    Input,
    /// Detected during capability input retrieval
    Output,
    /// Detected during capability client retrieval
    Client,
    /// Detected during serializing input of wasm capability call
    CapabilityInput,
    /// Detected during serializing client of wasm capability call
    CapabilityClient,
    /// Detected during serializing client of wasm capability call
    CapabilityOutput,
    /// Detected during wasm main call
    Call,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Phase::Init => write!(f, "Init()"),
            Phase::Reset => write!(f, "Reset()"),

            Phase::State => write!(f, "State()"),
            Phase::Input => write!(f, "Input()"),
            Phase::Client => write!(f, "Client()"),
            Phase::Output => write!(f, "Output()"),

            Phase::CapabilityInput => write!(f, "CapabilityInput()"),
            Phase::CapabilityClient => write!(f, "CapabilityClient()"),
            Phase::CapabilityOutput => write!(f, "CapabilityOutput()"),

            Phase::Call => write!(f, "Call()"),
        }
    }
}

/// Error type for FFI operations between wasm or capabilities
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, Error)]
pub enum FfiError {
    /// Null pointer was provided
    #[error("FFI error: null pointer provided")]
    NullPointer(Phase),

    /// Zero length was provided
    #[error("FFI error: zero length provided")]
    ZeroLength(Phase),

    /// Failed to validate archived bytes
    #[error("FFI validation error: {0} during {1}")]
    ValidationFailed(String, Phase),

    /// Failed to deserialize data
    #[error("FFI deserialization error: {0} during {1}")]
    DeserializationFailed(String, Phase),

    #[error("FFI deserialization panic: {0:?} during {1}")]
    DeserializationPanicked(FfiPanic, Phase),

    /// Failed to convert the call data to a row
    #[error("Input failed ToRow: {0}")]
    ToRowFailed(String),

    /// Failed to serialize data
    #[error("FFI serialization error: {0} during {1}")]
    SerializationFailed(String, Phase),

    #[error("FFI serialization panic: {0:?} during {1}")]
    SerializationPanicked(FfiPanic, Phase),

    #[error("User Logic panic: {0:?}")]
    ModuleLogicPanicked(FfiPanic),

    #[error("Capability Logic panic: {0:?}")]
    CapabilityLogicPanicked(FfiPanic),

    #[error("Future polled after completion")]
    FuturePolledAfterCompletion,

    #[error("Unknown FFI Tag: {0}")]
    UnknownTag(u8),

    #[error("Host Side capability fail, check the error slot")]
    HostSideCapability,
}

/// User-facing error that wraps internal errors with simplified messaging
#[derive(Debug, Error)]
pub struct PyroductError {
    name: String,
    kind: ErrorKind,
    inner: InnerError,
    /// Location where the error was created (host-side)
    location: Option<ErrorLocation>,
}

#[derive(Debug, Clone)]
pub struct ErrorLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl fmt::Display for ErrorLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

#[derive(Debug)]
enum ErrorKind {
    LogicPanic,
    IoError,
    LinkingError,
    Unknown,
}

#[derive(Debug)]
enum InnerError {
    Capability(PathBuf, FfiError),
    Module(PathBuf, FfiError),
    CapabilityLinking(PathBuf, String),
    ModuleLinking(PathBuf, String),
    ModuleUnknown(PathBuf, String),
    // New Extensions
    ModuleSerialization(PathBuf, String),
    ModuleMemory(PathBuf, String),
    ModuleValidation(PathBuf, String),
    ModuleExecution(PathBuf, String),
}

impl PyroductError {
    /// Create a new user-facing error from a capability FfiError with automatic location tracking
    pub fn from_capability(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        inner: FfiError,
    ) -> Self {
        Self::from_capability_with_location(name, path, inner, None)
    }

    /// Create a new user-facing error from a capability FfiError with explicit location
    pub fn from_capability_with_location(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        inner: FfiError,
        location: Option<ErrorLocation>,
    ) -> Self {
        let kind = Self::classify_error(&inner);
        Self {
            name: name.into(),
            kind,
            inner: InnerError::Capability(path.into(), inner),
            location,
        }
    }

    /// Create a new user-facing error from a module FfiError with automatic location tracking
    pub fn from_module(name: impl Into<String>, path: impl Into<PathBuf>, inner: FfiError) -> Self {
        Self::from_module_with_location(name, path, inner, None)
    }

    /// Create a new user-facing error from a module FfiError with automatic location tracking
    pub fn from_module_unknown(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            name: name.into(),
            kind: ErrorKind::Unknown,
            inner: InnerError::ModuleUnknown(path.into(), error.to_string()),
            location: None,
        }
    }

    /// Create a new user-facing error from a module FfiError with explicit location
    pub fn from_module_with_location(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        inner: FfiError,
        location: Option<ErrorLocation>,
    ) -> Self {
        let kind = Self::classify_error(&inner);
        Self {
            name: name.into(),
            kind,
            inner: InnerError::Module(path.into(), inner),
            location,
        }
    }

    /// Create a new user-facing error for capability dynamic library loading/linking failures
    pub fn from_capability_linking(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            name: name.into(),
            kind: ErrorKind::LinkingError,
            inner: InnerError::CapabilityLinking(path.into(), error.to_string()),
            location: None,
        }
    }

    /// Create a new user-facing error for module loading/linking failures
    pub fn from_module_linking(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            name: name.into(),
            kind: ErrorKind::LinkingError,
            inner: InnerError::ModuleLinking(path.into(), error.to_string()),
            location: None,
        }
    }

    // --- New Extension Methods ---

    pub fn from_module_serialization(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            name: name.into(),
            kind: ErrorKind::IoError,
            inner: InnerError::ModuleSerialization(path.into(), error.to_string()),
            location: None,
        }
    }

    pub fn from_module_memory(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            name: name.into(),
            kind: ErrorKind::IoError,
            inner: InnerError::ModuleMemory(path.into(), error.to_string()),
            location: None,
        }
    }

    pub fn from_module_validation(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            name: name.into(),
            kind: ErrorKind::IoError,
            inner: InnerError::ModuleValidation(path.into(), error.to_string()),
            location: None,
        }
    }

    pub fn from_module_execution(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        error: impl fmt::Display,
    ) -> Self {
        Self {
            name: name.into(),
            kind: ErrorKind::Unknown,
            inner: InnerError::ModuleExecution(path.into(), error.to_string()),
            location: None,
        }
    }

    // ----------------------------

    fn classify_error(error: &FfiError) -> ErrorKind {
        match error {
            // Logic panics - both user logic and wasm-side panics
            FfiError::ModuleLogicPanicked(_)
            | FfiError::CapabilityLogicPanicked(_)
            | FfiError::DeserializationPanicked(_, _)
            | FfiError::SerializationPanicked(_, _) => ErrorKind::LogicPanic,

            // I/O errors - serialization, deserialization, validation
            FfiError::HostSideCapability
            | FfiError::NullPointer(_)
            | FfiError::ZeroLength(_)
            | FfiError::ValidationFailed(_, _)
            | FfiError::DeserializationFailed(_, _)
            | FfiError::SerializationFailed(_, _)
            | FfiError::ToRowFailed(_)
            | FfiError::FuturePolledAfterCompletion
            | FfiError::UnknownTag(_) => ErrorKind::IoError,
        }
    }

    /// Get the path to the capability or module that errored
    pub fn path(&self) -> &Path {
        match &self.inner {
            InnerError::Capability(path, _)
            | InnerError::Module(path, _)
            | InnerError::CapabilityLinking(path, _)
            | InnerError::ModuleLinking(path, _)
            | InnerError::ModuleUnknown(path, _)
            | InnerError::ModuleSerialization(path, _)
            | InnerError::ModuleMemory(path, _)
            | InnerError::ModuleValidation(path, _)
            | InnerError::ModuleExecution(path, _) => path.as_path(),
        }
    }

    /// Get the underlying error details for diagnostics
    pub fn detailed_error(&self) -> String {
        match &self.inner {
            InnerError::Capability(path, e) => {
                format!("Capability at {:?}: {:?}", path, e)
            }
            InnerError::Module(path, e) => {
                format!("Module at {:?}: {:?}", path, e)
            }
            InnerError::CapabilityLinking(path, e) => {
                format!("Capability linking at {:?}: {}", path, e)
            }
            InnerError::ModuleLinking(path, e) => {
                format!("Module linking at {:?}: {}", path, e)
            }
            InnerError::ModuleUnknown(path, e) => {
                format!("Module call failed {:?}, unknown: {}", path, e)
            }
            InnerError::ModuleSerialization(path, e) => {
                format!("Module input serialization failed {:?}: {}", path, e)
            }
            InnerError::ModuleMemory(path, e) => {
                format!("Module memory error {:?}: {}", path, e)
            }
            InnerError::ModuleValidation(path, e) => {
                format!("Module return validation failed {:?}: {}", path, e)
            }
            InnerError::ModuleExecution(path, e) => {
                format!("Module execution returned error {:?}: {}", path, e)
            }
        }
    }

    /// Get the host-side location where this error was created
    pub fn host_location(&self) -> Option<&ErrorLocation> {
        self.location.as_ref()
    }
}

impl fmt::Display for PyroductError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::LogicPanic => {
                write!(f, "{} experienced a logic panic", self.name)?;
                write!(f, ": {}", self.inner)?;
            }
            ErrorKind::IoError => {
                write!(f, "{} experienced an I/O error", self.name)?;
                if let Some(loc) = &self.location {
                    write!(f, " (detected at {})", loc)?;
                }
                write!(f, ": {}", self.inner)?;
            }
            ErrorKind::LinkingError => {
                write!(f, "{} experienced a linking error", self.name)?;
                if let Some(loc) = &self.location {
                    write!(f, " (detected at {})", loc)?;
                }
                write!(f, ": {}", self.inner)?;
            }
            ErrorKind::Unknown => {
                write!(f, "{} failed with an unknown error", self.name)?;
                if let Some(loc) = &self.location {
                    write!(f, " (detected at {})", loc)?;
                }
                write!(f, ": {}", self.inner)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for InnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InnerError::Capability(path, e) => {
                write!(f, "{} (capability: {})", e, path.display())
            }
            InnerError::Module(path, e) => {
                write!(f, "{} (module: {})", e, path.display())
            }
            InnerError::CapabilityLinking(path, e) => {
                write!(f, "{} (capability: {})", e, path.display())
            }
            InnerError::ModuleLinking(path, e) => {
                write!(f, "{} (module: {})", e, path.display())
            }
            InnerError::ModuleUnknown(path, e) => {
                write!(f, "{} (module: {})", e, path.display())
            }
            InnerError::ModuleSerialization(path, e) => {
                write!(f, "{} (module: {})", e, path.display())
            }
            InnerError::ModuleMemory(path, e) => {
                write!(f, "{} (module: {})", e, path.display())
            }
            InnerError::ModuleValidation(path, e) => {
                write!(f, "{} (module: {})", e, path.display())
            }
            InnerError::ModuleExecution(path, e) => {
                write!(f, "{} (module: {})", e, path.display())
            }
        }
    }
}

// Convenience conversions
impl FfiError {
    pub fn to_capability_error(
        self,
        name: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> PyroductError {
        PyroductError::from_capability(name, path, self)
    }

    pub fn to_module_error(
        self,
        name: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> PyroductError {
        PyroductError::from_module(name, path, self)
    }
}
