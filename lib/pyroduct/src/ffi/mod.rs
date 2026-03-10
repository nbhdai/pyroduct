mod interface;
pub use interface::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod guest;

use crate::format::UserHeaderValues;

impl UserHeaderValues for serde_json::Value {}
