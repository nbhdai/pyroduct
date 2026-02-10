//! JSON-based pyro format.
//!
//! This module mirrors the `rkyv_8` module but uses `serde_json` for
//! serialization. There is **no zero-copy access** — `expose()` eagerly
//! deserializes into the owned Rust type.
//!
//! # When to use JSON over Rkyv
//!
//! - Debugging / human-readable wire dumps
//! - Interop with non-Rust consumers (Python, JS, etc.)
//! - Types that already derive `serde::{Serialize, Deserialize}` but not rkyv
//! - Prototyping before committing to a zero-copy schema
//!
//! # Usage
//!
//! ```rust,ignore
//! use pyro_vec::json::Json;
//! use pyro_vec::format::UserHeaderValues;
//! use pyro_vec::Bridgeable;
//!
//! // Implement manually (or via a future `#[bridgeable(json)]` macro):
//! impl UserHeaderValues for MyType {
//!     const VERSION: u8 = 0;
//! }
//!
//! impl Bridgeable for MyType {
//!     type Format = Json<MyType>;
//! }
//! ```

use crate::format::PyroHeaderValues;
use crate::header::DataStatus;

mod pyro;
mod buffers;
mod deserialize;
mod serialize;

pub use pyro::Json;
pub use deserialize::JsonParser;
pub use serialize::JsonWriter;

// ─── Header values ───────────────────────────────────────────────────────────

/// Header constants for the JSON wire format.
///
/// Uses the same OK/ERR status codes as rkyv so that the transport layer
/// (TCP framing, FFI, etc.) does not need to know which format is in use —
/// the `wire_format` byte distinguishes them.
pub struct JsonHeader;

impl PyroHeaderValues for JsonHeader {
    const OK_CODE: DataStatus = DataStatus::JsonValid;
    const ERR_CODE: DataStatus = DataStatus::JsonError;
}
