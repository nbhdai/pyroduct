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

/// Unified capability export macro that handles all FFI generation.
///
/// # Usage
/// ```ignore
/// capability! {
///     env = "my_capability",
///     
///     #[capability_client]
///     pub struct MyClient { ... }
///     
///     #[capability]
///     pub trait MyTrait { ... }
///     
///     #[capability_server(service = MyTrait)]
///     pub struct MyServer { ... }
///     
///     impl MyTrait for MyServer { ... }
/// }
/// ```
///
/// ## Functions
/// Marks a standalone function as a capability function (Pattern 1: no state).
///
/// This attribute generates the necessary FFI boilerplate to expose a host-side function
/// to WASM modules. It handles argument serialization (deserializing inputs from WASM memory),
/// output serialization (writing results back to WASM memory), and execution.
///
/// # Supported Patterns
///
/// * **Synchronous**: Standard functions are executed immediately.
/// * **Asynchronous**: `async` functions are supported and executed via the host's async runtime.
/// * **Multiple Arguments**: Functions with multiple arguments are automatically handled by generating an internal struct for serialization.
///
/// # Examples
///
/// ## Simple Synchronous
/// ```ignore
/// #[capability_function]
/// pub fn add(a: u32, b: u32) -> u32 {
///     a + b
/// }
/// ```
///
/// ## Async I/O
/// ```ignore
/// #[capability_function]
/// pub async fn fetch_data(url: String) -> Result<Vec<u8>, String> {
///     // Implementation utilizing async runtime
///     Ok(vec![])
/// }
/// ```
///
/// ## Clients
/// Marks a struct as client-side state that gets serialized and sent to the host.
///
/// Generates rkyv derives for serialization.
///
/// # Example
/// ```ignore
/// #[capability_client]
/// #[derive(Debug, Clone)]
/// pub struct HttpClient {
///     pub base_url: String,
///     pub timeout_secs: Option<u64>,
/// }
/// ```
///
/// ## Services
/// Defines a capability trait with automatic FFI generation.
///
/// # Attributes
/// - `stateless` - The server has no persistent state (creates fresh state per request)
///
/// # Method Attributes
/// - `#[client_state]` - Marks a parameter as client state to be serialized
///
/// # Example
/// ```ignore
/// #[capability]
/// pub trait Reporter {
///     fn report(&mut self, message: String) -> String;
/// }
///
/// #[capability(stateless)]
/// pub trait Http {
///     async fn get(#[client_state] client: &HttpClient, path: &str) -> Result<HttpResponse, String>;
/// }
/// ```
///
/// ## Service Implementations
/// Marks a struct as the server-side implementation of a capability.
///
/// # Attributes
/// - `service = TraitName` - The capability trait this implements
/// - `config = ConfigType` - Optional configuration type (must impl serde::Deserialize)
/// - `stateless` - No persistent server state
///
/// Generates:
/// - FFI host functions for each trait method
/// - `plugin_init`, `plugin_drop`, `plugin_reset` lifecycle functions
/// - A `{StructName}Init` trait for initialization
///
/// # Example
/// ```ignore
/// #[capability_server(service = Reporter, config = ReporterConfig)]
/// pub struct ReporterServer {
///     logs: VecDeque<String>,
///     max_history: usize,
/// }
///
/// impl ReporterServerInit for ReporterServer {
///     fn new() -> Self { ... }
///     fn with_config(config: ReporterConfig) -> Self { ... }
///     fn reset(&mut self) { ... }
/// }
///
/// impl Reporter for ReporterServer {
///     fn report(&mut self, message: String) -> String { ... }
/// }
/// ```
///
/// ## Service Definitions
/// Generates FFI host functions from a trait implementation.
///
/// This should be placed on the `impl TraitName for ServerStruct` block.
/// It generates the host FFI functions that bridge WASM calls to your implementation.
///
/// # Attributes
/// - `env = "module_name"` - The WASM import module name
///
/// # Example
/// ```ignore
/// #[capability_impl(env = "reporter")]
/// impl Reporter for ReporterServer {
///     fn report(&mut self, message: String) -> String {
///         // implementation
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