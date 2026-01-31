#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

pub mod capability;
pub mod host;
pub mod module;

pub mod capability_host;
pub mod module_capability;
pub mod module_host;

pub mod errors;

use std::path::Display;

pub use arrow_scalars::{deep_ref::DeepRef, from_row::FromRow, to_row::ToRow};

pub use arrow_derive::{DeepRef, FromRow, ToRow};

pub use serde;
pub use rkyv;
pub use arrow_scalars;
pub use tracing;

/// Module Derive Macro
///
/// This crate provides the `#[module]` attribute macro to generate the WASM
/// entry point boilerplate for modules.
///
/// # Return Value Patterns
///
/// 1. **Single value with named field**: `#[module(output = field_name)]`
/// 2. **Tuple with named fields**: `#[module(output = (field1, field2, ...))]`
/// 3. **Struct implementing ToRow**: `#[module(output = MyStruct)]`
///
/// # Examples
/// *Pattern 1*: Single named output. This makes an arrow column called "message"
/// ```rust
/// use pyroduct::*;
///  
/// #[module(output = output)]
/// pub fn call(input: &str) -> Result<String, String> {
///     Ok(format!("Hello, {}", input))
/// }
/// ```
/// *Pattern 2*: Tuple with named fields. This makes a pair of arrow columns called "count" and "data"
/// ```rust
/// use pyroduct::*;
///
/// #[module(output = (count, data))]
/// fn process(input: &str) -> Result<(u32, Vec<u8>), String> {
///     Ok((input.len() as u32, input.as_bytes().to_vec()))
/// }
/// ```
/// *Pattern 3*: Return a struct that implements ToRow. This makes the same output as pattern 2.
/// ```rust
/// use pyroduct::*;
///
/// #[derive(ToRow)]
/// struct ProcessResult {
///     count: u32,
///     data: Vec<u8>,
/// }
///
/// #[module(output = ProcessResult)]
/// fn process(input: &str) -> Result<ProcessResult, String> {
///     Ok(ProcessResult { count: 42, data: vec![] })
/// }
/// ```
/// You can even nest fields.
/// ```rust
/// use pyroduct::*;
///
/// #[derive(FromRow, DeepRef, ToRow)]
/// struct CallMessage {
///     message: String,
///     role: String,
/// }
///
/// #[module(output = (output, messages))]
/// fn process(input: &[CallMessageRef<'_>]) -> Result<(String, Vec<CallMessage>), String> {
///     let output = input.first().ok_or("Empty chat history".to_string())?;
///     Ok((
///         output.message.to_string(),
///         vec![
///             CallMessage { message: "hi".to_string(), role: "user".to_string() },
///             CallMessage { message: "How can I help?".to_string(), role: "agent".to_string() },
///         ],
///     ))
/// }
/// ```
pub use module_derive::module;


/// Marks a struct as configuration for a capability.
///
/// Adds serde serialization/deserialization derives so the config can be passed
/// across the FFI boundary as JSON.
///
/// # Example
/// ```rust
/// #[pyroduct::config]
/// pub struct SerialConfig {
///     pub ports: Vec<String>,
/// }
/// ```
pub use capability_derive::config;

/// Marks a struct as client-side state for a capability.
///
/// Adds rkyv serialization/deserialization derives so the client state can be
/// passed efficiently across the WASM/FFI boundary using zero-copy deserialization.
///
/// # Example
/// ```rust
/// #[pyroduct::client]
/// pub struct SerialHandle {
///     pub id: u64,
/// }
/// ```
pub use capability_derive::client;


/// Defines a capability implementation with lifecycle methods and callable functions.
///
/// This macro transforms an impl block into a complete capability with FFI bindings,
/// WASM imports, and client-side method generation.
///
/// # Associated Types
///
/// The impl block must define the following associated types:
///
/// - **`type Client = ...`** (required): The client state struct marked with `#[pyroduct::client]`.
///   All capability methods must accept `&Self::Client` as their second parameter.
///
/// - **`type Config = ...`** (optional): The configuration struct marked with `#[pyroduct::config]`.
///   If specified, `fn new` must accept `Option<Self::Config>` as its parameter.
///
/// - **`type Error = ...`** (optional): The error type for fallible operations.
///   If specified, `new_client` and all other methods must return `Result<T, Self::Error>`.
///
/// # Lifecycle Methods
///
/// Three lifecycle methods are required:
///
/// ## `fn new(config: Option<Config>) -> Self` or `fn new() -> Self`
///
/// Called once when the capability is loaded by the host. Use this to initialize
/// server-side state such as connection pools, caches, or hardware handles.
/// May be `async`. The `config` parameter is `Option<T>` because the host may
/// not provide configuration.
///
/// ## `fn reset(&mut self)`
///
/// Called before each module invocation to reset server state to a clean baseline.
/// Use this to clear per-request caches, reset counters, or release temporary resources
/// while preserving expensive-to-create resources like connections. May be `async`.
///
/// ## `fn new_client(&self, client: &Client)` or `fn new_client(&self, client: &Client) -> Result<(), Error>`
///
/// Called when a WASM module registers a new client instance. Use this to validate
/// client configuration, allocate per-client resources, or perform authentication.
/// If `type Error` is defined, this must return `Result<(), Error>`.
///
/// # Capability Methods
///
/// Additional methods define the capability's API. All methods must:
/// - Take `&self` as the first parameter (not `&mut self`)
/// - Take `client: &Client` (or `_client: &Client`) as the second parameter
/// - Return `Result<T, Error>` if `type Error` is defined
///
/// Methods may be `async` and may take additional parameters which will be
/// automatically serialized across the FFI boundary.
///
/// # Example
///
/// ```rust
/// #[pyroduct::config]
/// pub struct SerialConfig {
///     pub ports: Vec<String>,
/// }
///
/// #[pyroduct::client]
/// pub struct SerialHandle {
///     pub id: u64,
/// }
///
/// pub struct SerialServer {
///     ports: Vec<String>,
///     next_id: u64,
/// }
///
/// #[pyroduct::capability]
/// impl SerialServer {
///     type Config = SerialConfig;
///     type Client = SerialHandle;
///     type Error = String;
///
///     fn new(config: Option<SerialConfig>) -> Self {
///         Self {
///             ports: config.map(|c| c.ports).unwrap_or_default(),
///             next_id: 0,
///         }
///     }
///
///     fn reset(&mut self) {
///         self.next_id = 0;
///     }
///
///     fn new_client(&self, client: &SerialHandle) -> Result<(), String> {
///         if client.id > 100 {
///             return Err("Invalid client ID".to_string());
///         }
///         Ok(())
///     }
///
///     fn write(&self, _client: &SerialHandle, data: Vec<u8>) -> Result<usize, String> {
///         Ok(data.len())
///     }
///
///     async fn read(&self, _client: &SerialHandle, count: usize) -> Result<Vec<u8>, String> {
///         Ok(vec![0u8; count])
///     }
/// }
/// ```
pub use capability_derive::capability;

pub type PyroductResult<T> = Result<T, errors::PyroductError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapIdentity {
    pub path: std::sync::Arc<std::path::Path>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModIdentity {
    pub path: std::sync::Arc<std::path::Path>,
}

impl CapIdentity {
    pub fn from<'a, P: Into<&'a std::path::Path>>(p: P) -> Self {
        Self {
            path: std::sync::Arc::from(p.into()),
        }
    }

    pub fn display(&self) -> Display<'_> {
        self.path.display()
    }

    pub fn name(&self) -> &str {
        self.path
            .as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.strip_prefix("lib").unwrap_or(s))
            .unwrap_or_else(|| "unknown")
    }
}

impl ModIdentity {
    pub fn from<'a, P: Into<&'a std::path::Path>>(p: P) -> Self {
        Self {
            path: std::sync::Arc::from(p.into()),
        }
    }

    pub fn display(&self) -> Display<'_> {
        self.path.display()
    }

    pub fn name(&self) -> &str {
        self.path
            .as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.strip_prefix("lib").unwrap_or(s))
            .unwrap_or_else(|| "unknown")
    }
}
