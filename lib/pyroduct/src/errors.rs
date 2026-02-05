use std::fmt;
use bridge_vec::{BridgeError, captured::{LibraryInfo, library}};
use thiserror::Error;


/// User-facing error that wraps internal errors with simplified messaging
#[derive(Debug, Error)]
pub struct PyroductError {
    kind: ErrorKind,
    inner: InnerError,
}

impl fmt::Display for PyroductError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Compact: [Kind] InnerMessage
        write!(f, "[{:?}] {}", self.kind, self.inner)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f) // Reuses the Debug name (e.g., "NotFound")
    }
}

impl fmt::Display for InnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capability(e) => write!(f, "Capability error: {e}"),
            Self::Module(e) => write!(f, "Module error: {e}"),
            Self::Loading(lib, msg) => write!(f, "Failed to load {lib}: {msg}"),
            Self::Linking(lib, msg) => write!(f, "Linker failure for {lib}: {msg}"),
            Self::ModuleMemory(lib, msg) => write!(f, "Memory fault in {lib}: {msg}"),
            Self::Infrastructure(msg) => write!(f, "Host setup failed: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
        }
    }
}

#[derive(Debug)]
enum ErrorKind {
    NotFound,
    LogicPanic,
    IoError,
    LoadingError,
    LinkingError,
    Infrastructure,
    Unknown,
}

#[derive(Debug)]
enum InnerError {
    Capability(BridgeError),
    Module(BridgeError),
    Loading(LibraryInfo<'static>, String),
    Linking(LibraryInfo<'static>, String),

    ModuleMemory(LibraryInfo<'static>, String),
    Infrastructure(String), // Added for host setup failures
    NotFound(String),
}

impl PyroductError {
    /// Create a new user-facing error from a capability FfiError with explicit location
    pub fn missing_cap(name: &str) -> Self {
        Self {
            kind: ErrorKind::NotFound,
            inner: InnerError::NotFound(name.to_string()),
        }
    }

    /// Create a new user-facing error from a capability FfiError with automatic location tracking
    pub fn from_capability(inner: BridgeError) -> Self {
        Self {
            kind: ErrorKind::LogicPanic,
            inner: InnerError::Capability(inner),
        }
    }

    /// Create a new user-facing error for capability dynamic library loading/linking failures
    pub fn from_loading(ident: &LibraryInfo<'static>, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::LoadingError,
            inner: InnerError::Loading(ident.clone(), error.to_string()),
        }
    }

    /// Create a new user-facing error for capability dynamic library loading/linking failures
    pub fn from_linking(ident: &LibraryInfo<'static>, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::LinkingError,
            inner: InnerError::Linking(ident.clone(), error.to_string()),
        }
    }

    /// Create a new user-facing error from a module FfiError with automatic location tracking
    pub fn from_module(inner: BridgeError) -> Self {
        Self {
            kind: ErrorKind::LogicPanic,
            inner: InnerError::Module(inner),
        }
    }

    // --- New Extension Methods ---

    /// Create a new user-facing error for host infrastructure failures ("I fucked up")
    pub fn from_infrastructure(error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::Infrastructure,
            inner: InnerError::Infrastructure(error.to_string()),
        }
    }

    pub fn from_module_memory(module: &LibraryInfo<'static>, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::IoError,
            inner: InnerError::ModuleMemory(module.clone(), error.to_string()),
        }
    }

    /// Get the path to the capability or module that errored
    pub fn library(&self) -> Option<&LibraryInfo<'static>> {
        match &self.inner {
            InnerError::NotFound(_) => library(),
            InnerError::Infrastructure(_) => library(),
            InnerError::Capability(error) 
            | InnerError::Module(error) => error.library(),
            | InnerError::ModuleMemory(library_info, _) => Some(library_info),
            InnerError::Loading(library_info, _) => Some(library_info),
            InnerError::Linking(library_info, _) => Some(library_info),
        }
    }

    /// Get the underlying error details for diagnostics
    pub fn detailed_error(&self) -> String {
        match &self.inner {
            InnerError::NotFound(name) => {
                format!("NotFound: {:?}", name)
            }
            InnerError::Infrastructure(e) => {
                format!("Infrastructure failure: {}", e)
            }
            InnerError::Capability(path) => {
                format!("Capability: {:?}", path)
            }
            InnerError::Module(path) => {
                format!("Module: {:?}", path)
            }
            InnerError::ModuleMemory(path, e) => {
                format!("Module memory error {:?}: {}", path, e)
            }
            InnerError::Loading(library_info, e) => format!("Unable to load {}: {}", library_info, e),
            InnerError::Linking(library_info, e) => format!("Unable to link {}: {}", library_info, e),
        }
    }
}
