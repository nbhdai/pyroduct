//! RPC message envelope types for the cache server.

use serde::{Deserialize, Serialize};

/// Requests that can be sent to a [`super::CacheServer`].
///
/// Covers every operation exposed by [`pyro_artifacts::cache::CacheManager`]
/// that is meaningful over a remote connection.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CacheRequest {
    // ── Capabilities ────────────────────────────────────────────────────────

    /// List all cached capabilities, optionally filtered by author and/or package.
    ListCapabilities {
        author: Option<String>,
        package: Option<String>,
    },

    /// Fetch the raw `interface.json` string for a capability.
    GetCapabilityInterfaceSpec {
        author: String,
        name: String,
        version: String,
    },

    /// Fetch the raw `config.json` string for a capability (if present).
    GetCapabilityConfigSpec {
        author: String,
        name: String,
        version: String,
    },

    // ── Modules / Playbooks ─────────────────────────────────────────────────

    /// List all cached playbook modules, optionally filtered.
    ListModules {
        author: Option<String>,
        package: Option<String>,
    },

    /// Find the latest semver version of a module in the cache.
    FindLatestVersion { author: String, package: String },

    /// Fetch the `spec.json` for a playbook module as a JSON value.
    GetPlaybookSpec {
        author: String,
        name: String,
        version: String,
    },

    /// Fetch the Rust source file (`source.rs`) for a playbook module.
    GetPlaybookSource {
        author: String,
        name: String,
        version: String,
    },

    /// Load the playbook binary and return its `configurations` field as JSON.
    GetPlaybookConfigurations {
        author: String,
        name: String,
        version: String,
    },

    // ── Config ──────────────────────────────────────────────────────────────

    /// Read the `config.toml` of the cache root as a JSON value.
    GetPyroductConfig,

    // ── Purge ────────────────────────────────────────────────────────────────

    /// Remove all cached capabilities, interfaces, and modules, then re-init.
    PurgeCache,

    /// Remove only capabilities and interfaces from the cache.
    PurgeCapabilities,

    /// Remove only playbook modules from the cache.
    PurgeModules,

    // ── Status ───────────────────────────────────────────────────────────────

    /// Ping — returns the cache root path and the server version.
    Status,
}

/// Responses sent back by the [`super::CacheServer`].
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CacheResponse {
    /// A list of `(author, name, version)` tuples.
    ArtifactList { items: Vec<(String, String, String)> },

    /// Raw JSON or TOML content decoded as a `serde_json::Value`.
    Json { value: serde_json::Value },

    /// Raw string content (Rust source, JSON interface spec, etc.).
    Text { content: String },

    /// The latest version string found for a module.
    Version { version: Option<String> },

    /// Server status / ping reply.
    Status {
        cache_root: String,
        version: String,
    },

    /// A successful operation with no meaningful return value.
    Ok,

    /// An error occurred while processing the request.
    Error { message: String },
}

// ── Bridgeable impls ─────────────────────────────────────────────────────────

impl pyroduct::format::Bridgeable for CacheRequest {
    type Format = pyroduct::format::json::Json<CacheRequest>;
}

impl pyroduct::format::Bridgeable for CacheResponse {
    type Format = pyroduct::format::json::Json<CacheResponse>;
}
