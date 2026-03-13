//! Integration tests: package the real `capabilities/httpc` capability (with
//! its generated interface) and the real `modules/basic` module, then ship
//! them into a temporary cache and verify the on-disk layout.
//!
//! These tests invoke `cargo build` under the hood so they are slow — mark
//! them #[ignore] if you only want fast unit tests in CI.

use crate::cache::CacheManager;
use crate::environment::Environment;
use std::path::PathBuf;

/// Resolve the repo root from the artifacts crate (lib/artifacts -> ../..).
fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // lib/artifacts
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

// ---------------------------------------------------------------------------
// Ship capabilities/httpc (capability + interface) into the cache
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ship_httpc_capability_to_cache() {
    let cache = CacheManager::new().await.unwrap();

    let httpc_path = repo_root().join("capabilities/httpc");
    assert!(
        httpc_path.join("Capability.toml").exists(),
        "Cannot find capabilities/httpc — run tests from the repo root"
    );

    let env = Environment::new(httpc_path).await.unwrap();

    // 1. Generate and ship the interface
    let interface = env
        .create_interface()
        .await
        .unwrap()
        .expect("httpc is a capability, so create_interface must return Some");

    cache.write_artifacts(interface.into()).await.unwrap();

    let iface_dir = cache.interface_dir("nbhdai", "httpc", "0.1.0");
    assert!(iface_dir.join("Capability.toml").exists());
    assert!(iface_dir.join("Cargo.toml").exists());
    assert!(iface_dir.join("src/lib.rs").exists());
    assert!(iface_dir.join("interface.json").exists());

    // 2. Build and ship the capability binary
    let cap_artifacts = env.package(true).await.unwrap();
    cache.write_artifacts(cap_artifacts).await.unwrap();

    let cap_dir = cache.capabilities_dir("nbhdai", "httpc", "0.1.0");
    assert!(cap_dir.join("Capability.toml").exists());
    assert!(cap_dir.join("Cargo.toml").exists());
    assert!(cap_dir.join("Cargo.lock").exists());
    assert!(cap_dir.join("src/lib.rs").exists());
    assert!(cap_dir.join("interface.json").exists());

    // The native library must exist (platform-dependent extension)
    let has_lib = cap_dir.join("lib.dylib").exists()
        || cap_dir.join("lib.so").exists()
        || cap_dir.join("lib.dll").exists();
    assert!(has_lib, "expected a native library in the cache");
}

// ---------------------------------------------------------------------------
// Ship modules/basic into the cache
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ship_basic_module_to_cache() {
    let cache = CacheManager::new().await.unwrap();

    let basic_path = repo_root().join("modules/basic");
    assert!(
        basic_path.join("Module.toml").exists(),
        "Cannot find modules/basic — run tests from the repo root"
    );

    let env = Environment::new(basic_path).await.unwrap();

    // Modules have no interface step
    assert!(
        env.create_interface().await.unwrap().is_none(),
        "a module should not produce an interface"
    );

    let module_artifacts = env.package(true).await.unwrap();
    cache.write_artifacts(module_artifacts).await.unwrap();

    let mod_dir = cache
        .root
        .join("modules")
        .join("nbhdai")
        .join("basic")
        .join("0.1.0");

    assert!(mod_dir.join("mod.wasm").exists());
    assert!(mod_dir.join("Module.toml").exists());
    assert!(mod_dir.join("Cargo.toml").exists());
    assert!(mod_dir.join("Cargo.lock").exists());
    assert!(mod_dir.join("src/lib.rs").exists());

    // Sanity-check the wasm has the right magic bytes
    let wasm = std::fs::read(mod_dir.join("mod.wasm")).unwrap();
    assert!(
        wasm.starts_with(&[0x00, 0x61, 0x73, 0x6D]),
        "mod.wasm should start with the wasm magic number"
    );
}