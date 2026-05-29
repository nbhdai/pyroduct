//! Integration tests: package the real `capabilities/httpc` capability (with
//! its generated interface) and the real `modules/basic` module, then ship
//! them into a temporary cache and verify the on-disk layout.
//!
//! These tests invoke `cargo build` under the hood so they are slow — mark
//! them #[ignore] if you only want fast unit tests in CI.

use crate::artifacts::{
    Artifact, Artifacts, CapabilityBinary, CapabilityConfig, ModuleDependencies, ModuleSource,
    Playbook,
};
use crate::build::Builder;
use crate::cache::{CacheManager, PyroductConfig};
use crate::cargo::ResolvedCapability;
use crate::environment::Environment;
use cargo_toml::Dependency;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

const BASIC_MODULE: &str = r#"
/// You can do a basic transform on the data without relying on a capability
#[pyroduct::module(output = output)]
fn prefix(input: &str) -> Result<String> {
    Ok(format!("Prefixed: {input}"))
}
"#;

const HTTPC_MODULE: &str = r#"
use httpc::{HttpClient, HttpClientMethods};

#[pyroduct::module(output = response)]
fn call(url: &str) -> Result<String, String> {
    let client = HttpClient.register().map_err(|e| e.to_string())?;
    let response = client.get(url.to_string())?;
    Ok(response)
}
"#;

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
    let mut config =
        toml::from_str::<PyroductConfig>(&content).expect("Failed to parse config.toml");

    // Execute cargo metadata to find the actual target directory absolute path
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .expect("Failed to execute cargo metadata");

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Failed to parse cargo metadata JSON");

    let target_dir = metadata["target_directory"]
        .as_str()
        .map(std::path::PathBuf::from)
        .expect("Missing target_directory in metadata");

    // Ensure the pyroduct dependency path is absolute before we pass it to
    // a CacheManager running inside a temporary directory.
    if let Some(Dependency::Detailed(detail)) = &mut config.pyroduct
        && let Some(path) = &mut detail.path
    {
        let absolute_path = std::path::Path::new(&root).join(&path);
        *path = absolute_path
            .canonicalize()
            .unwrap_or(absolute_path)
            .to_string_lossy()
            .into_owned();
    }

    PyroductConfig {
        author: None,
        target: Some(target_dir), // Set the target to the absolute path found via cargo
        pyroduct: config.pyroduct,
        build_slots: Some(4),
    }
}

#[tokio::test]
async fn ship_httpc_capability_to_cache() {
    let dir = TempDir::new().unwrap();
    let cache = Arc::new(CacheManager::new(dir.path(), None).await.unwrap());

    let httpc_path = repo_root().join("capabilities/httpc");
    assert!(
        httpc_path.join("Capability.toml").exists(),
        "Cannot find capabilities/httpc — run tests from the repo root"
    );

    let env = Environment::new(httpc_path, cache.clone()).await.unwrap();

    // 2. Build and ship the capability binary
    let cap_artifacts = env.package(true).await.unwrap();
    for artifact in &cap_artifacts {
        cache.write_artifacts(artifact).await.unwrap();
    }

    let cap_dir = cache.capabilities_dir("nbhdai", "httpc", "0.1.0");
    assert!(cap_dir.join("Capability.toml").exists());
    assert!(cap_dir.join("Cargo.toml").exists());
    assert!(cap_dir.join("Cargo.lock").exists());
    assert!(cap_dir.join("src/lib.rs").exists());

    let iface_dir = cache.interface_dir("nbhdai", "httpc", "0.1.0");
    assert!(iface_dir.join("Capability.toml").exists());
    assert!(iface_dir.join("Cargo.toml").exists());
    assert!(iface_dir.join("src/lib.rs").exists());
    assert!(iface_dir.join("interface.json").exists());

    // The native library must exist (platform-dependent extension)
    let has_lib = cap_dir.join("lib.dylib").exists()
        || cap_dir.join("lib.so").exists()
        || cap_dir.join("lib.dll").exists();
    assert!(has_lib, "expected a native library in the cache");
}

#[tokio::test]
async fn test_anon_compile_with_interface() {
    let dir = TempDir::new().unwrap();
    let config = test_config();

    let cache = Arc::new(
        CacheManager::new(dir.path(), config.pyroduct.clone())
            .await
            .unwrap(),
    );
    let builder = Builder::new(&dir.path().join("build"), config, Arc::clone(&cache))
        .await
        .unwrap();

    // 1. Generate the interface for httpc to compile against
    let httpc_path = repo_root().join("capabilities/httpc");
    let env = Environment::new(httpc_path, cache.clone()).await.unwrap();
    let capability = env.package(true).await.unwrap();
    for artifact in &capability {
        cache.write_artifacts(artifact).await.unwrap();
    }

    let cap = ResolvedCapability {
        author: "nbhdai".to_string(),
        package: "httpc".to_string(),
        version: "0.1.0".to_string(),
    };

    let mod_source = ModuleSource {
        ident: None,
        dependencies: ModuleDependencies {
            dependencies: BTreeMap::new(),
            capabilities: vec![cap],
        },
        source: HTTPC_MODULE.to_string(),
    };
    let anon = builder.compile(&mod_source).await.unwrap();

    assert!(!anon.wasm.is_empty());
    assert!(
        anon.wasm.starts_with(&[0x00, 0x61, 0x73, 0x6D]),
        "Compiled output should be a valid WASM binary"
    );
}

// -----------------------------------------------------------------------------
// Data Integrity Roundtrip Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_module_wasm_exact_match() {
    let dir = TempDir::new().unwrap();
    let cache = Arc::new(CacheManager::new(dir.path(), None).await.unwrap());
    let builder = Builder::new(&dir.path().join("build"), test_config(), Arc::clone(&cache))
        .await
        .unwrap();

    let source = ModuleSource {
        ident: None,
        dependencies: ModuleDependencies {
            dependencies: BTreeMap::new(),
            capabilities: vec![],
        },
        source: BASIC_MODULE.to_string(),
    };
    cache.write_artifacts(&source.clone().into()).await.unwrap();
    let binary = builder.compile(&source).await.unwrap();
    let original_wasm = binary.wasm.clone();
    cache.write_artifacts(&binary.clone().into()).await.unwrap();

    let hash = source.hash();

    let loaded_artifact = cache.get_binary(&hash).await.unwrap();

    assert_eq!(
        original_wasm, loaded_artifact.wasm,
        "WASM reloaded from dir does not match original"
    );
}

#[tokio::test]
async fn test_capability_lib_exact_match() {
    let dir = TempDir::new().unwrap();
    let cache = Arc::new(CacheManager::new(dir.path(), None).await.unwrap());

    let httpc_path = repo_root().join("capabilities/httpc");
    let env = Environment::new(httpc_path, cache.clone()).await.unwrap();

    let cap_artifacts = env.package(true).await.unwrap();

    // Extract original shared library bytes
    let original_lib_bytes = cap_artifacts
        .iter()
        .find_map(|a| match a {
            Artifacts::CapabilityBinary(c) => Some(c.libs[0].to_vec()),
            _ => None,
        })
        .expect("Expected CapabilityBinary artifact");

    // Write to disk
    for artifact in &cap_artifacts {
        cache.write_artifacts(artifact).await.unwrap();
    }
    let cap_dir = cache.capabilities_dir("nbhdai", "httpc", "0.1.0");

    // Verify exact match after Artifacts::from_dir read
    let loaded_artifact = CapabilityBinary::from_dir(&cap_dir).await.unwrap();

    assert_eq!(
        original_lib_bytes,
        loaded_artifact.libs[0].to_vec(),
        "Capability library bytes do not match after roundtrip to disk"
    );
}

#[tokio::test]
async fn test_load_playbook() {
    let dir = TempDir::new().unwrap();
    let config = test_config();

    let cache = Arc::new(
        CacheManager::new(dir.path(), config.pyroduct.clone())
            .await
            .unwrap(),
    );
    let builder = Builder::new(&dir.path().join("build"), config, Arc::clone(&cache))
        .await
        .unwrap();

    // 1. Setup a module with a capability
    let httpc_path = repo_root().join("capabilities/httpc");
    let env = Environment::new(httpc_path, cache.clone()).await.unwrap();
    let capability = env.package(true).await.unwrap();
    for artifact in &capability {
        cache.write_artifacts(artifact).await.unwrap();
    }

    let cap = ResolvedCapability {
        author: "nbhdai".to_string(),
        package: "httpc".to_string(),
        version: "0.1.0".to_string(),
    };

    let mod_source = ModuleSource {
        ident: None,
        dependencies: ModuleDependencies {
            dependencies: BTreeMap::new(),
            capabilities: vec![cap],
        },
        source: HTTPC_MODULE.to_string(),
    };
    let binary = builder.compile(&mod_source).await.unwrap();

    // 2. Create a Playbook
    let playbook = Playbook {
        hash: binary.spec.hash.clone(),
        configurations: HashMap::from([(
            "httpc".to_string(),
            CapabilityConfig {
                classes: HashMap::from([(
                    "CounterClient".to_string(),
                    Some(serde_json::json!({"timeout": 30})),
                )]),
            },
        )]),
    };

    // 3. Load the Playbook
    let loaded = cache
        .load_playbook(playbook.clone(), HashMap::new(), "", "", "")
        .await
        .unwrap();

    // 4. Verify
    assert_eq!(loaded.binary.spec.hash, binary.spec.hash);
    assert_eq!(loaded.configurations, playbook.configurations);
    assert!(loaded.paths.contains_key("httpc"));
    let cap_path = loaded.paths.get("httpc").unwrap();
    assert!(cap_path.exists());
    assert!(cap_path.to_string_lossy().contains("httpc"));
}
