//! # PyroVec: Zero-Copy Transport
//!
//! `pyroduct` provides a specialized buffer type optimized for moving complex Rust types
//! across boundaries (like FFI to dynamically loaded Rust) or process boundaries with zero-copy
//! overhead.
//!
//! It is meant to provide a unified data structure to safely pass across FFI, TCP, WASM, and
//! Unix sockets to other Rust libraries.
//!
//! ## How it works
//!
//! This library pyros the gap between high-level Rust types and raw memory pointers by
//! combining **`rkyv`** (for zero-copy serialization) with a **custom memory layout** that
//! carries protocol metadata (length, capacity, status codes) in a 16-byte aligned header.
//!
//! 1.  **Define**: Annotate your Rust types with `#[bridgeable]`. This derives the necessary
//!     `rkyv` traits, implements `UserHeaderValues` (version = 0) and `Bridgeable` (format = Rkyv).
//! 2.  **Ship**: Call `.ship()` on your type to produce a `PyroVec`. This serializes the data
//!     directly into an FFI-safe, aligned memory buffer.
//! 3.  **Transport**: Pass the raw pointer (`vec.into_raw()`) to the foreign system.
//! 4.  **Expose**: On the receiving end, the pointer is reconstructed into a `PyroVec`. The
//!     data can then be accessed immediately (zero-copy) via `expose()`, or fully deserialized
//!     back into a Rust type via a `Receiver`.
//!
//! ## The `#[bridgeable]` macro
//!
//! The attribute macro is the primary entry point for users. It accepts the following syntax:
//!
//! ```rust,ignore
//! #[magma]                                      // bare minimum
//! #[magma(derive(Debug, PartialEq))]            // + derives on the archived type
//! #[magma(derive(Debug, PartialEq), Document)]  // all of the above
//! ```
//!
//! **What it generates:**
//!
//! For every annotated struct or enum the macro produces:
//!
//! - `#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]` on the type itself.
//! - Whitelisted derives forwarded to the archived type via `#[rkyv(attr(derive(...)))]`.
//!   The whitelist is: `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`.
//! - `#[rkyv(compare(...))]` attributes for `PartialEq` / `PartialOrd`.
//! - `impl UserHeaderValues` with `VERSION = 0`.
//! - `impl Bridgeable` with `type Format = Rkyv<T>`.
//! - *(opt-in)* `impl Documented` producing a [`documented::TypeSpec`] when `Document` is passed.
//!
//! ### Basic example
//!
//! ```rust,ignore
//! use pyroduct::{magma, Bridgeable};
//! use pyroduct::format::HasReceiver;
//!
//! #[magma]
//! #[derive(Debug, PartialEq)]
//! struct UserProfile {
//!     id: u32,
//!     username: String,
//!     tags: Vec<String>,
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let original = UserProfile {
//!         id: 101,
//!         username: "ferris".to_string(),
//!         tags: vec!["rust".into(), "ffi".into()],
//!     };
//!
//!     // Ship — serialize into a PyroVec
//!     let pyroduct = original.ship()?;
//!
//!     // --- FFI / transport boundary ---
//!     let ptr = pyroduct.into_raw();
//!     let passed_vec = unsafe { PyroVec::from_raw(ptr) }?;
//!     // ---------------------------------
//!
//!     // Expose — zero-copy validated access to the archived data
//!     let typed = UserProfile::expose(passed_vec)?;
//!     assert_eq!(typed.id, 101);
//!     assert_eq!(typed.username.as_str(), "ferris");
//!
//!     // Receive — full deserialization back into the owned Rust type
//!     let mut receiver = typed.receiver();
//!     let recovered: UserProfile = receiver.receive(&typed)?;
//!     assert_eq!(original, recovered);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Results
//!
//! `Result<T, E>` is handled natively through the wire format's **Status** header field.
//! Instead of serializing the `Result` enum wrapper, the variant discriminant is lifted into
//! the header so the receiver can distinguish success from failure before parsing the payload.
//!
//! Use the [`BridgeableResult`] trait (automatically implemented for `Result<T, E>` where
//! both `T` and `E` are `Bridgeable`):
//!
//! - `result.ship()` — serializes `Ok(T)` with Status `ValidData` (0) or `Err(E)` with
//!   Status `UserError` (1).
//! - `Result::<T, E>::expose(vec)` — returns `Result<Result<TypedBuf<T>, TypedBuf<E>>, PyroError>`.
//!
//! ### Example: Result transport
//!
//! ```rust,ignore
//! use pyroduct::{magma, Bridgeable, BridgeableResult};
//!
//! #[magma]
//! #[derive(Debug, PartialEq)]
//! struct Response {
//!     id: u32,
//!     payload: String,
//! }
//!
//! #[magma]
//! #[derive(Debug, PartialEq)]
//! struct ApiError {
//!     code: u16,
//!     reason: String,
//! }
//!
//! let success: Result<Response, ApiError> = Ok(Response {
//!     id: 101,
//!     payload: "Data retrieved".to_string(),
//! });
//!
//! // Ship the Result — Status = 0 (ValidData)
//! let vec = success.ship()?;
//!
//! // Expose — discriminate via the header, then zero-copy access
//! match <Result<Response, ApiError>>::expose(vec)? {
//!     Ok(typed_ok)  => assert_eq!(typed_ok.id, 101),
//!     Err(typed_err) => panic!("expected success"),
//! }
//! ```
//!
//! # FFI Safety & Panic Handling
//!
//! This module provides the safety layer required when crossing FFI boundaries (e.g., calling
//! into a dynamic library or from a C host). It ensures that panics in Rust code do not
//! unwind across the FFI boundary, which is undefined behavior.
//!
//! ### Features
//!
//! 1. **Panic Catching**: Wraps execution in `catch_unwind` to contain panics.
//! 2. **Rich Error Reporting**: Installs a custom panic hook to capture file, line, and message
//!    details into Thread Local Storage (TLS) before the stack unwinds.
//! 3. **Transport Error Serialization**: Converts panics or serialization failures into a
//!    `PyroVec` with a pyro-error status code, allowing the caller to receive a structured
//!    error report safely.
//!
//! ### Usage
//!
//! Use `execute_safe` to wrap any logic intended for FFI export:
//!
//! ```rust,ignore
//! #[unsafe(no_mangle)]
//! pub extern "C" fn my_ffi_func() -> *const u8 {
//!     pyroduct::ffi::execute_safe(|| {
//!         // Your logic here
//!         process_data()
//!     })
//! }
//! ```
//!
//! # Memory Layout & Header Protocol
//!
//! `PyroVec` utilizes a custom 16-byte aligned memory layout compatible with FFI
//! boundary crossing. The allocation consists of a **16-byte Header** followed immediately
//! by the **Data Payload**.
//!
//! ## Layout Diagram
//!
//! ```text
//!  Pointer (16-byte aligned)
//!  │
//!  ▼
//! ┌───────────────────────────────────────────────────────────────────┐
//! │                             Magic (u32)                           │
//! ├───────────────────────────────────────────────────────────────────┤
//! │                              Len (u32)                            │
//! ├───────────────────────────────────────────────────────────────────┤
//! │                            Reserved (u32)                         │
//! ├─────────────────┬────────────────┬──────────────────┬─────────────┤
//! │ Wire Format(u8) │ User Vers (u8) │ User Err Ver(u8) │ Status (u8) │
//! ├─────────────────┴────────────────┴──────────────────┴─────────────┤
//! │                           Data Payload ...                        │
//! └───────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Header Fields
//!
//! | Offset | Type  | Field          | Description                                                        |
//! |--------|-------|----------------|--------------------------------------------------------------------|
//! | `0x00` | `u32` | Magic          | Constant `0x7079726F` (ASCII "pyro"). Verifies pointer validity.   |
//! | `0x04` | `u32` | Len            | Current length of the data payload in bytes.                       |
//! | `0x08` | `u32` | Cap            | Total allocated capacity in bytes, not including header.           |
//! | `0x0C` | `u8`  | Wire Format    | Protocol version number (currently `1`).                           |
//! | `0x0D` | `u8`  | User Version   | `UserHeaderValues::VERSION` of the `Ok(T)` / value type.           |
//! | `0x0E` | `u8`  | User Error Ver | `UserHeaderValues::VERSION` of the `Err(E)` type.                  |
//! | `0x0F` | `u8`  | Status         | **Message status**. Determines how the payload is interpreted.     |
//!
//! ## Status Codes (Offset 0x0F)
//!
//! When passing `Result<T, E>` across FFI or transport boundaries, the status field determines
//! how the payload should be interpreted:
//!
//! | Code   | Name                | Meaning                                                           |
//! |--------|---------------------|-------------------------------------------------------------------|
//! | `0`    | `ValidData`         | Payload is a valid `rkyv`-archived `T` — corresponds to `Ok(T)`.  |
//! | `1`    | `UserError`         | Payload is a valid `rkyv`-archived `E` — corresponds to `Err(E)`. |
//! | `3`    | `CodeError`         | A panic was caught; payload is a JSON `CapturedError`.            |
//! | `99`   | `PyroFfiFail`     | FFI pyro failed to parse a header (critical).                   |
//! | `100‑108` | `Local*`         | Errors detected on the local (sending) side.                      |
//! | `150‑158` | `Remote*`        | Errors detected on the remote (receiving) side.                   |
//!
//! `PyroVec` optimizes `Result<T, E>` transport by lifting the variant discriminant into the
//! **Status** header field. This avoids the overhead of serializing the `Result` enum wrapper
//! and allows the receiving end to immediately distinguish between success and failure before
//! parsing the payload.
//!
//! - **Success (`Ok(T)`)**: Status `0`, payload = serialized `T`, version in `User Version`.
//! - **Failure (`Err(E)`)**: Status `1`, payload = serialized `E`, version in `User Error Ver`.
//!
//! ## Do Not Use In Production (yet)
//!
//! We're going to dog-food this until we get versioning correct.

// Re-export rkyv for users

pub mod bridgeable;
pub mod captured;
pub mod error;

pub mod format;
pub mod header;
pub mod json;
pub mod panic;
pub mod rkyv_8;
pub mod value;
pub mod vec_buf;
mod view;
pub mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub mod pipeline;

#[cfg(not(target_arch = "wasm32"))]
pub mod ffi;

pub use bridgeable::{Bridgeable, BridgeableResult};
pub use captured::CapturedError;
pub use error::PyroError;
pub use header::{MAGIC_VAL, ParseError};
pub use rkyv::rancor::Error as RancorError;
pub use rkyv_8::{Rkyv, RkyvParser, RkyvWriter, TypedBuf, TypedPyroView};
pub use value::{DeepRef, PyroRow, PyroValue, ToRow};
pub use vec_buf::{PyroBuf, PyroBufPtr, PyroVec, PyroVecPtr};
pub use view::{PyroMutView, PyroView, PyroViewPtr, get_view, get_view_mut};

// Async is not supported for wasm
#[cfg(not(target_arch = "wasm32"))]
pub mod tokio;
pub use tracing;
pub use serde;
pub use serde_json;

// Documented by the lib.
pub use pyro_derive::bridgeable;

// Documented by the lib.
pub use pyro_derive::magma;

pub use pyro_derive::Document;

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

/// Derives a "Ref" struct (a view struct) and implements the `FromRow` trait.
///
/// Example:
/// ```rust
/// use pyroduct::{FromRow, DeepRef};
///
/// #[derive(FromRow, DeepRef)]
/// struct Foo { val: String }
/// // Generates:
/// // struct FooRef<'a> { val: &'a str }
/// // impl<'a> FromRow<'a> for FooRef<'a> { ... }
/// ```
pub use pyro_derive::{FromRow, RefFromRow};

/// Derives the `AsDeepRef` trait for both the original struct AND its rkyv `Archived` counterpart.
/// This requires that the "Ref" struct (e.g., FooRef) already exists (usually via `ArrowRef`).
///
/// Example:
/// ```rust
/// use pyroduct::DeepRef;
///
/// #[derive(DeepRef)]
/// struct Foo { val: String }
/// // Generates:
/// // impl AsDeepRef for Foo { type Ref<'a> = FooRef<'a>; ... }
/// ```
pub use pyro_derive::DeepRef;

/// Derives `DeepRef` for the rkyv `Archived` counterpart of a struct.
///
/// Given a struct `Foo` that already has `FooRef<'a>` (from `#[derive(DeepRef)]`),
/// this generates:
///
/// ```ignore
/// impl DeepRef for ArchivedFoo {
///     type Ref<'a> = FooRef<'a>;
///     fn as_deep_ref(&self) -> Self::Ref<'_> { ... }
/// }
/// ```
///
/// This allows zero-copy access from archived data directly to the lightweight
/// ref struct, without deserializing.
pub use pyro_derive::DeepRefArchived;

/// Derives the `ToRow` trait for converting structs into PyroRow/PyroValue.
/// This is the opposite of `ArrowRef` which extracts references from PyroRow.
///
/// Example:
/// ```rust
/// use pyroduct::ToRow;
///
/// #[derive(ToRow)]
/// struct Foo { val: String }
/// // Generates:
/// // impl ToRow for Foo {
/// //     fn to_arrow_row(&self) -> PyroRow<'_> { ... }
/// //     fn to_arrow_row_owned(self) -> PyroRow<'static> { ... }
/// //     fn to_arrow_value(&self) -> PyroValue<'_> { ... }
/// //     fn to_arrow_value_owned(self) -> PyroValue<'static> { ... }
/// // }
/// ```
pub use pyro_derive::ToRow;


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
#[cfg(not(target_arch = "wasm32"))]
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
///     fn register(&self, client: &SerialHandle) -> Result<(), String> {
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
#[cfg(not(target_arch = "wasm32"))]
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
///     let output = input.first().ok_or(anyhow::anyhow!("Empty chat history"))?;
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
pub use pyro_derive::module;