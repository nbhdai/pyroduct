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
//! use pyroduct::{magma, format::{Bridgeable, HasReceiver}};
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
//! use pyroduct::{magma, format::{Bridgeable, BridgeableResult}};
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

pub mod bridgeable;
pub mod format;
pub mod header;
pub mod json;
pub mod value;
pub mod vec_buf;
mod view;

pub use bridgeable::{Bridgeable, Decoder, Encoder};
pub use format::{HasReceiver, Receiver};
pub use header::{MAGIC_VAL, ParseError};
pub use value::{DeepRef, PyroRow, PyroValue, ToRow};
pub use vec_buf::{PyroBuf, PyroBufPtr, PyroVec, PyroVecPtr};
pub use view::{PyroMutView, PyroView, PyroViewPtr, get_view, get_view_mut};

// Async is not supported for wasm
#[cfg(any(feature = "host", feature = "capability"))]
pub mod tokio;

pub use serde;
pub use serde_json;

/// Derives a "Ref" struct (a view struct) and implements the `FromRow` trait.
///
/// Example:
/// ```rust
/// use pyroduct::format::{FromRow, DeepRef};
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
/// use pyroduct::format::DeepRef;
///
/// #[derive(DeepRef)]
/// struct Foo { val: String }
/// // Generates:
/// // impl AsDeepRef for Foo { type Ref<'a> = FooRef<'a>; ... }
/// ```
pub use pyro_derive::DeepRef;

/// Derives the `ToRow` trait for converting structs into PyroRow/PyroValue.
/// This is the opposite of `ArrowRef` which extracts references from PyroRow.
///
/// Example:
/// ```rust
/// use pyroduct::format::ToRow;
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

/// Derives the `TypeableRow` trait for a struct, generating a `PyroSchema` that
/// describes its fields, their types, nullability, and any doc-comments.
///
/// Each field must implement `Typeable`. Struct-level and field-level `///` doc-comments
/// are captured and embedded in the schema at compile time.
///
/// # Example
/// ```rust
/// use pyroduct::format::Document;
///
/// #[derive(Document)]
/// /// A sensor reading.
/// struct Reading {
///     /// Unique sensor ID.
///     id: u32,
///     value: f64,
/// }
/// // Generates:
/// // impl TypeableRow for Reading {
/// //     fn schema() -> PyroSchema<'static> { ... }
/// // }
/// ```
pub use pyro_derive::Document;
