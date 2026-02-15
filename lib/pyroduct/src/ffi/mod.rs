mod interface;
pub use interface::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod guest;
pub mod host;

use crate::format::UserHeaderValues;

impl UserHeaderValues for serde_json::Value {}
