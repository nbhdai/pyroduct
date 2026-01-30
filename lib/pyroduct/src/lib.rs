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

pub use rkyv;

pub use capability_derive::{capability, client, config};

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
