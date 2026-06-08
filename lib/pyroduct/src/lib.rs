// Re-export rkyv for users

pub mod captured;
pub mod error;

pub mod format;
pub mod panic;

pub mod session;

#[cfg(feature = "host")]
pub mod module;

#[cfg(feature = "module")]
pub mod wasm;

#[cfg(feature = "module")]
pub use wasm::interconnect::{
    call_playbook, call_session, query_all_specs, query_spec, with_spec, with_specs,
};

#[cfg(feature = "host")]
pub mod pipeline;

#[cfg(feature = "host")]
pub mod transport;

#[cfg(any(feature = "host", feature = "capability"))]
pub mod ffi;

pub use captured::{Capture, CapturedError};
pub use error::PyroError;
pub use format::{PyroRow, PyroValue};
pub use rkyv::rancor::Error as RancorError;
pub use tracing;

// Documented by the lib.
pub use pyro_derive::magma;

/// Registers the library identity and metadata for the pyro system.
///
/// This macro generates a `Library` struct and a registration function used to identify
/// the current binary during RPC operations and error reporting.
/// The identity is stored in a global static and automatically captured by
/// `CapturedError` instances.
///
/// ### Generated Items
/// The macro generates a `pub struct Library` with the following constants:
/// * **`META`**: A custom metadata string passed to the macro (defaults to `""`).
/// * **`NAME`**: The name of the crate, pulled from `CARGO_PKG_NAME` at compile time.
/// * **`VERSION`**: The crate version from `CARGO_PKG_VERSION`.
/// * **`AUTHORS`**: The crate authors from `CARGO_PKG_AUTHORS`.
///
/// It also defines a private method `register_info()` that populates the global
/// `APP_IDENTITY` static in the pyro's captured error module.
///
/// ### Usage
///
/// **Basic usage:**
/// ```rust
/// pyroduct::library!();
/// ```
///
/// **With custom metadata:**
/// ```rust
/// pyroduct::library!("pyroduct-engine");
/// ```
///
/// ### Integration with `CapturedError`
/// When this macro is used and the library identity is registered, any `CapturedError`
/// created thereafter will automatically include this identity. This is
/// particularly useful for debugging remote errors in an RPC system where you need
/// to know exactly which version and crate produced a panic or failure.
pub use pyro_derive::library;

/// A specialized Result type for PyroVec operations.
pub type PyroResult<T> = Result<T, PyroError>;

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
#[cfg(feature = "capability")]
pub use pyro_derive::config;

/// Defines a capability implementation with lifecycle methods and callable functions.
///
/// This macro transforms an impl block into a complete capability with FFI bindings,
/// WASM imports, and client-side method generation.
///
/// # Associated Types
///
/// The impl block must define the following associated types:
///
/// - **`type Client = ...`** (required): The client state struct marked with `#[pyroduct::magma]`.
///   All capability methods must accept `&Self::Client` as their second parameter.
///
/// - **`type Config = ...`** (optional): The configuration struct marked with `#[pyroduct::config]`.
///   If specified, `fn new` must accept `Option<Self::Config>` as its parameter.
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
/// ## `fn register(&self, client: &Client)` or `fn register(&self, client: &Client) -> Result<()>`
///
/// Called when a WASM module registers a new client instance. Use this to validate
/// client configuration, allocate per-client resources, or perform authentication.
/// Fallible implementations must return `Result<()>`.
///
/// # Capability Methods
///
/// Additional methods define the capability's API. All methods must:
/// - Take `&self` as the first parameter (not `&mut self`)
/// - Take `client: &Client` (or `_client: &Client`) as the second parameter
/// - Return `Result<T>` if they are fallible
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
/// #[pyroduct::magma]
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
///
///     fn new(config: Option<SerialConfig>) -> Result<Self> {
///         Ok(Self {
///             ports: config.map(|c| c.ports).unwrap_or_default(),
///             next_id: 0,
///         })
///     }
///
///     fn reset(&mut self) -> Result<()> {
///         self.next_id = 0;
///         Ok(())
///     }
///
///     fn register(&self, client: &SerialHandle) -> Result<()> {
///         if client.id > 100 {
///             pyroduct::bail!("Invalid client ID");
///         }
///         Ok(())
///     }
///
///     fn write(&self, _client: &SerialHandle, data: Vec<u8>) -> Result<usize> {
///         Ok(data.len())
///     }
///
///     async fn read(&self, _client: &SerialHandle, count: usize) -> Result<Vec<u8>> {
///         Ok(vec![0u8; count])
///     }
/// }
/// ```
#[cfg(feature = "capability")]
pub use pyro_derive::capability;

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
/// pub fn call(input: &str) -> Result<String> {
///     Ok(format!("Hello, {}", input))
/// }
/// ```
/// *Pattern 2*: Tuple with named fields. This makes a pair of arrow columns called "count" and "data"
/// ```rust
/// use pyroduct::*;
///
/// #[module(output = (count, data))]
/// fn process(input: &str) -> Result<(u32, Vec<u8>)> {
///     Ok((input.len() as u32, input.as_bytes().to_vec()))
/// }
/// ```
/// *Pattern 3*: Return a struct that implements ToRow. This makes the same output as pattern 2.
/// ```rust
/// use pyroduct::*;
///
/// #[magma]
/// struct ProcessResult {
///     count: u32,
///     data: Vec<u8>,
/// }
///
/// #[module(output = ProcessResult)]
/// fn process(input: &str) -> Result<ProcessResult> {
///     Ok(ProcessResult { count: 42, data: vec![] })
/// }
/// ```
/// You can even nest fields.
/// ```rust
/// use pyroduct::*;
///
/// #[magma]
/// struct CallMessage {
///     message: String,
///     role: String,
/// }
///
/// #[module(output = (output, messages))]
/// fn process(input: Vec<CallMessage>) -> Result<(String, Vec<CallMessage>)> {
///     let output = input.first().ok_or(pyroduct::capture!("Empty chat history"))?;
///     Ok((
///         output.message.to_string(),
///         vec![
///             CallMessage { message: "hi".to_string(), role: "user".to_string() },
///             CallMessage { message: "How can I help?".to_string(), role: "agent".to_string() },
///         ],
///     ))
/// }
/// ```
///
///
/// If you don't want to copy a large vector and just need to reference it, you can do that too.
/// ```rust
/// use pyroduct::*;
///
/// #[magma]
/// struct Embedding {
///     message: String,
///     embedding: Vec<f32>,
/// }
///
/// #[module(output = messages)]
/// fn process(input: Vec<EmbeddingRef<'_>>) -> Result<Vec<String>> {
///     Ok(input.iter().map(|e| {
///        let embedding: &[f32] = e.embedding;
///         format!("Embedding of {} has {}", e.message, embedding.len())
///     }).collect())
/// }
/// ```
#[cfg(feature = "module")]
pub use pyro_derive::module;
