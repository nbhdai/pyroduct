mod interface;
pub use interface::*;

#[cfg(feature = "capability")]
pub mod guest;

use crate::format::UserHeaderValues;

impl UserHeaderValues for serde_json::Value {}
