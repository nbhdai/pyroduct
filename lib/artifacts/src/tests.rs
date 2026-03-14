//! Integration tests: package the real `capabilities/httpc` capability (with
//! its generated interface) and the real `modules/basic` module, then ship
//! them into a temporary cache and verify the on-disk layout.
//!
//! These tests invoke `cargo build` under the hood so they are slow — mark
//! them #[ignore] if you only want fast unit tests in CI.

use crate::cache::{CacheManager, PyroductConfig};
use crate::cargo::ResolvedCapability;
use crate::environment::Environment;
use sha2::Digest;
use std::path::PathBuf;
use std::{collections::BTreeMap};
use tempfile::TempDir;

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

pub fn test_config() -> PyroductConfig {
    let root = std::env::var("PYRODUCT").expect("PYRODUCT env var not set");
    let config_path = std::path::Path::new(&root).join("config.toml");
    
    // Read the base configuration for pyroduct dependencies
    let content = std::fs::read_to_string(&config_path).expect("Failed to read config.toml");
    let config = toml::from_str::<PyroductConfig>(&content).expect("Failed to parse config.toml");

    // Execute cargo metadata to find the actual target directory absolute path
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .expect("Failed to execute cargo metadata");

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("Failed to parse cargo metadata JSON");

    let target_dir = metadata["target_directory"]
        .as_str()
        .map(std::path::PathBuf::from)
        .expect("Missing target_directory in metadata");

    PyroductConfig {
        author: None,
        target: Some(target_dir), // Set the target to the absolute path found via cargo
        pyroduct: config.pyroduct,
    }
}

#[tokio::test]
async fn ship_httpc_capability_to_cache() {
    let dir = TempDir::new().unwrap();
    let cache = CacheManager::new(dir.path(), test_config()).await.unwrap();

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

    cache.write_artifacts(&interface.into()).await.unwrap();

    let iface_dir = cache.interface_dir("nbhdai", "httpc", "0.1.0");
    assert!(iface_dir.join("Capability.toml").exists());
    assert!(iface_dir.join("Cargo.toml").exists());
    assert!(iface_dir.join("src/lib.rs").exists());
    assert!(iface_dir.join("interface.json").exists());

    // 2. Build and ship the capability binary
    let cap_artifacts = env.package(true).await.unwrap();
    cache.write_artifacts(&cap_artifacts).await.unwrap();

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

#[tokio::test]
async fn ship_basic_module_to_cache() {
let dir = TempDir::new().unwrap();
    let cache = CacheManager::new(dir.path(), test_config()).await.unwrap();

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
    cache.write_artifacts(&module_artifacts).await.unwrap();

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

#[tokio::test]
async fn test_anon_compile_with_interface() {
let dir = TempDir::new().unwrap();
    let cache = CacheManager::new(dir.path(), test_config()).await.unwrap();

    // 1. Generate the interface for httpc to compile against
    let httpc_path = repo_root().join("capabilities/httpc");
    let env = Environment::new(httpc_path).await.unwrap();
    let interface = env
        .create_interface()
        .await
        .unwrap()
        .expect("httpc is a capability, so create_interface must return Some");

    // Write the interface manually to the capabilities directory to satisfy the `ResolvedCapability::interface_dir`
    cache.write_artifacts(&interface.into()).await.unwrap();

    let cap = ResolvedCapability {
        author: "nbhdai".to_string(),
        package: "httpc".to_string(),
        version: "0.1.0".to_string(),
    };

    let code = r#"
        use httpc::{HttpClient, HttpClientMethods};

        #[pyroduct::module(output = response)]
        fn call(url: &str) -> Result<String, String> {
            let client = HttpClient.register().map_err(|e| e.to_string())?;
            let response = client.get(url.to_string())?;
            Ok(response)
        }
    "#;
    let caps = vec![cap];
    let anon = cache
        .compile_anon(&BTreeMap::new(), &caps, code)
        .await
        .unwrap();

    assert!(!anon.wasm.is_empty());
    assert!(
        anon.wasm.starts_with(&[0x00, 0x61, 0x73, 0x6D]),
        "Compiled output should be a valid WASM binary"
    );

    let mut hasher = sha2::Sha256::new();
    sha2::Digest::update(&mut hasher, code);
    let hash = format!("{:x}", sha2::Digest::finalize(hasher));

    // 2. Test debug_module
    let debug_mod = cache
        .debug_module(&BTreeMap::new(), &caps, code)
        .await
        .unwrap();
    let mod_dir = cache.root.join("anon").join(&hash);
    assert!(mod_dir.join("mod.wat").exists());
    assert!(mod_dir.join("cap.rs").exists());
    assert!(debug_mod.wat.is_some());
    assert!(debug_mod.cap_rs.is_some());

    // 3. Test debug_capabilities
    let debug_cap = cache
        .debug_capabilities("nbhdai", "httpc", "0.1.0")
        .await
        .unwrap();
    let cap_dir = cache.capabilities_dir("nbhdai", "httpc", "0.1.0");
    assert!(cap_dir.join("cap.rs").exists());
    assert!(debug_cap.cap_rs.is_some());
}
