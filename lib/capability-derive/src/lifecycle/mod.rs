//! Lifecycle function parsing and FFI generation
//!
//! This module handles the three required lifecycle functions:
//! - `new` - Initialize the server state
//! - `reset` - Reset the server state
//! - `new_client` - Register a new client

pub mod init;
pub mod new_client;
pub mod reset;

pub use init::InitFn;
pub use new_client::NewClientFn;
pub use reset::ResetFn;