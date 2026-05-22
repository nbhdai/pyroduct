//! Artifact management and CLI commands for Pyroduct.
//!
//! This crate handles building, caching, and resolving Pyroduct capabilities:
//! - [`artifacts`] — capability binary parsing (PE, Mach-O, ELF), interface extraction
//! - [`cache`] — artifact caching with content-addressed storage
//! - [`cargo`] — Cargo manifest and lockfile parsing for dependency resolution
//! - [`build`] — build orchestration (requires `compiler` feature)
//! - [`command`] — CLI command definitions (requires `compiler` feature)
//! - [`debug`] — debugging utilities (requires `compiler` feature)
//! - [`environment`] — environment setup (requires `compiler` feature)
//!
//! Capabilities are built as shared libraries, packed into tar.gz archives, and
//! resolved against Cargo.toml manifests for reproducible builds.

pub mod artifacts;
#[cfg(feature = "compiler")]
pub mod build;
#[cfg(feature = "compiler")]
pub mod command;

pub mod cache;
pub mod cargo;
#[cfg(feature = "compiler")]
pub mod debug;
#[cfg(feature = "compiler")]
pub mod environment;

#[cfg(test)]
mod tests;
