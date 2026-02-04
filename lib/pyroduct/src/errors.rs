// crates/pyroduct/src/errors.rs

use std::{fmt, path::Path};
use bridge_vec::BridgeError;
use thiserror::Error;

use crate::{CapIdentity, ModIdentity};

/// User-facing error that wraps internal errors with simplified messaging
#[derive(Debug, Error)]
pub struct PyroductError {
    kind: ErrorKind,
    inner: InnerError,
}

#[derive(Debug)]
enum ErrorKind {
    NotFound,
    LogicPanic,
    IoError,
    LoadingError,
    LinkingError,
    Infrastructure, // Added for "I fucked up" scenarios
    Unknown,
}

#[derive(Debug)]
enum InnerError {
    Capability(CapIdentity, BridgeError),
    Module(ModIdentity, BridgeError),
    CapabilityLoading(CapIdentity, String),
    CapabilityLinking(CapIdentity, String),
    ModuleLoading(ModIdentity, String),
    ModuleLinking(ModIdentity, String),
    ModuleUnknown(ModIdentity, String),
    // New Extensions
    ModuleSerialization(ModIdentity, String),
    ModuleMemory(ModIdentity, String),
    ModuleValidation(ModIdentity, String),
    ModuleExecution(ModIdentity, String),
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
    pub fn from_capability(ident: &CapIdentity, inner: BridgeError) -> Self {
        Self {
            kind: ErrorKind::LogicPanic,
            inner: InnerError::Capability(ident.clone(), inner),
        }
    }

    /// Create a new user-facing error for capability dynamic library loading/linking failures
    pub fn from_capability_loading(ident: &CapIdentity, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::LoadingError,
            inner: InnerError::CapabilityLoading(ident.clone(), error.to_string()),
        }
    }

    /// Create a new user-facing error for capability dynamic library loading/linking failures
    pub fn from_capability_linking(ident: &CapIdentity, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::LinkingError,
            inner: InnerError::CapabilityLinking(ident.clone(), error.to_string()),
        }
    }

    /// Create a new user-facing error from a module FfiError with automatic location tracking
    pub fn from_module(ident: &ModIdentity, inner: BridgeError) -> Self {
        Self {
            kind: ErrorKind::LogicPanic,
            inner: InnerError::Module(ident.clone(), inner),
        }
    }

    /// Create a new user-facing error from a module FfiError with automatic location tracking
    pub fn from_module_unknown(module: &ModIdentity, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::Unknown,
            inner: InnerError::ModuleUnknown(module.clone(), error.to_string()),
        }
    }

    /// Create a new user-facing error for module loading/linking failures
    pub fn from_module_linking(module: &ModIdentity, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::LinkingError,
            inner: InnerError::ModuleLinking(module.clone(), error.to_string()),
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

    pub fn from_module_serialization(module: &ModIdentity, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::IoError,
            inner: InnerError::ModuleSerialization(module.clone(), error.to_string()),
        }
    }

    pub fn from_module_memory(module: &ModIdentity, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::IoError,
            inner: InnerError::ModuleMemory(module.clone(), error.to_string()),
        }
    }

    pub fn from_module_validation(module: &ModIdentity, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::IoError,
            inner: InnerError::ModuleValidation(module.clone(), error.to_string()),
        }
    }

    pub fn from_module_execution(module: &ModIdentity, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::Unknown,
            inner: InnerError::ModuleExecution(module.clone(), error.to_string()),
        }
    }


    /// Get the path to the capability or module that errored
    pub fn path(&self) -> &Path {
        match &self.inner {
            InnerError::NotFound(_) => Path::new("./."),
            InnerError::Infrastructure(_) => Path::new("HOST_INFRASTRUCTURE"),
            InnerError::Capability(ident, _)
            | InnerError::CapabilityLinking(ident, _)
            | InnerError::CapabilityLoading(ident, _) => &ident.path,
            InnerError::Module(ident, _)
            | InnerError::ModuleLinking(ident, _)
            | InnerError::ModuleLoading(ident, _)
            | InnerError::ModuleUnknown(ident, _)
            | InnerError::ModuleSerialization(ident, _)
            | InnerError::ModuleMemory(ident, _)
            | InnerError::ModuleValidation(ident, _)
            | InnerError::ModuleExecution(ident, _) => &ident.path,
        }
    }

    /// Get the path to the capability or module that errored
    pub fn name(&self) -> &str {
        match &self.inner {
            InnerError::NotFound(name) => &name,
            InnerError::Infrastructure(_) => "Host Infrastructure",
            InnerError::Capability(ident, _)
            | InnerError::CapabilityLinking(ident, _)
            | InnerError::CapabilityLoading(ident, _) => &ident.name(),
            InnerError::Module(ident, _)
            | InnerError::ModuleLinking(ident, _)
            | InnerError::ModuleUnknown(ident, _)
            | InnerError::ModuleSerialization(ident, _)
            | InnerError::ModuleMemory(ident, _)
            | InnerError::ModuleValidation(ident, _)
            | InnerError::ModuleExecution(ident, _) => &ident.name(),
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
            InnerError::Capability(path, e) => {
                format!("Capability at {:?}: {:?}", path, e)
            }
            InnerError::Module(path, e) => {
                format!("Module at {:?}: {:?}", path, e)
            }
            InnerError::CapabilityLoading(path, e) => {
                format!("Capability loading at {:?}: {}", path, e)
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
            ErrorKind::NotFound => {
                write!(f, "Did not find: {}", self.name())?;
            }
            ErrorKind::Infrastructure => {
                write!(f, "Host Infrastructure failed (I fucked up)")?;
                write!(f, ": {}", self.inner)?;
            }
            ErrorKind::LogicPanic => {
                write!(f, "{} experienced a logic panic", self.name())?;
                write!(f, ": {}", self.inner)?;
            }
            ErrorKind::IoError => {
                write!(f, "{} experienced an I/O error", self.name())?;
                if let Some(loc) = &self.location {
                    write!(f, " (detected at {})", loc)?;
                }
                write!(f, ": {}", self.inner)?;
            }
            ErrorKind::LoadingError => {
                write!(f, "{} experienced a loading error", self.name())?;
                if let Some(loc) = &self.location {
                    write!(f, " (detected at {})", loc)?;
                }
                write!(f, ": {}", self.inner)?;
            }
            ErrorKind::LinkingError => {
                write!(f, "{} experienced a linking error", self.name())?;
                if let Some(loc) = &self.location {
                    write!(f, " (detected at {})", loc)?;
                }
                write!(f, ": {}", self.inner)?;
            }
            ErrorKind::Unknown => {
                write!(f, "{} failed with an unknown error", self.name())?;
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
            InnerError::NotFound(name) => {
                write!(f, "(not found: {})", name)
            }
            InnerError::Infrastructure(e) => {
                write!(f, "CRITICAL: {}", e)
            }
            InnerError::Capability(path, e) => {
                write!(f, "{} (capability: {})", e, path.display())
            }
            InnerError::Module(path, e) => {
                write!(f, "{} (module: {})", e, path.display())
            }
            InnerError::CapabilityLoading(path, e) => {
                write!(f, "{} (capability: {})", e, path.display())
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
    pub fn to_capability_error(self, ident: &CapIdentity) -> PyroductError {
        PyroductError::from_capability(ident, self)
    }

    pub fn to_module_error(self, ident: &ModIdentity) -> PyroductError {
        PyroductError::from_module(ident, self)
    }
}
