//! Cache RPC — serve and consume [`pyro_artifacts::cache::CacheManager`] over a socket.
//!
//! This module mirrors the structure of the daemon's own RPC layer:
//!
//! - [`CacheRequest`] / [`CacheResponse`] — top-level message envelopes used in
//!   [`crate::DaemonRequest::Cache`] / [`crate::DaemonResponse::Cache`]
//! - [`handle_request`] — stateless dispatcher called by the daemon server
//! - [`CacheClient`] — connect to a remote daemon and send `CacheRequest`s
//! - [`CacheServer`] — standalone server that wraps a [`CacheManager`] directly

mod types;
mod client;
mod server;

pub use types::{CacheRequest, CacheResponse};
pub use client::{CacheClient, connect_to_active_cache_server, invalidate_cached_cache_connection};
pub use server::CacheServer;

/// Handle a single [`CacheRequest`] against the local cache.
///
/// This is what the daemon's control-socket dispatcher calls for
/// `DaemonRequest::Cache(req)` — it loads [`pyro_artifacts::cache::CacheManager`]
/// from the environment and delegates to the appropriate operation.
pub async fn handle_request(req: CacheRequest) -> CacheResponse {
    match pyro_artifacts::cache::CacheManager::from_env().await {
        Ok(cache) => server::dispatch(&cache, req).await,
        Err(e) => CacheResponse::Error {
            message: format!("Failed to load CacheManager: {:?}", e),
        },
    }
}
