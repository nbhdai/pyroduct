//! Integration tests: package the real `capabilities/httpc` capability (with
//! its generated interface) and the real `modules/basic` module, then ship
//! them into a temporary cache and verify the on-disk layout.
//!
//! These tests invoke `cargo build` under the hood so they are slow — mark
//! them #[ignore] if you only want fast unit tests in CI.

use crate::artifacts::{Artifact, Artifacts, CapabilityBinary, Module, ModuleBinary, ModuleDependencies, ModuleSource};
use crate::cache::{CacheManager, PyroductConfig};
use crate::cargo::ResolvedCapability;
use crate::environment::Environment;
use cargo_toml::Dependency;
use std::collections::BTreeMap;
use std::path::PathBuf;
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
    if let Some(Dependency::Detailed(detail)) = &mut config.pyroduct {
        if let Some(path) = &mut detail.path {
            let absolute_path = std::path::Path::new(&root).join(&path);
            *path = absolute_path
                .canonicalize()
                .unwrap_or(absolute_path)
                .to_string_lossy()
                .into_owned();
        }
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
    for artifact in &cap_artifacts {
        cache.write_artifacts(artifact).await.unwrap();
    }

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
    for artifact in &module_artifacts {
        cache.write_artifacts(artifact).await.unwrap();
    }

    let source = module_artifacts
        .iter()
        .find_map(|a| match a {
            Artifacts::Module(Module::Source(s)) => Some(s),
            _ => None,
        })
        .expect("Expected ModuleSource artifact");

    // Modules are ephemeral and are placed in anon/{hash}
    let hash = source.hash();

    let mod_dir = cache.root.join("anon").join(&hash);

    assert!(mod_dir.join("mod.wasm").exists());
    assert!(mod_dir.join("source.rs").exists());
    assert!(mod_dir.join("spec.json").exists());
    assert!(mod_dir.join("dependencies.json").exists());

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
    let capability = env.package(true).await.unwrap();

    // Write the interface manually to the capabilities directory to satisfy the `ResolvedCapability::interface_dir`
    cache.write_artifacts(&interface.into()).await.unwrap();
    for artifact in &capability {
        cache.write_artifacts(artifact).await.unwrap();
    }

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
    let mod_source = ModuleSource {
        dependencies: ModuleDependencies {
            dependencies: BTreeMap::new(),
            capabilities: vec![cap],
        },
        source: code.to_string(),
    };
    let anon = cache.compile(&mod_source).await.unwrap();

    assert!(!anon.wasm.is_empty());
    assert!(
        anon.wasm.starts_with(&[0x00, 0x61, 0x73, 0x6D]),
        "Compiled output should be a valid WASM binary"
    );

    let hash = mod_source.hash();

    // 2. Test debug_module
    let debug_mod = cache.debug_module(&mod_source.hash()).await.unwrap();
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

// -----------------------------------------------------------------------------
// Data Integrity Roundtrip Tests
// -----------------------------------------------------------------------------

#[tokio::test]
async fn test_module_wasm_exact_match() {
    let dir = TempDir::new().unwrap();
    let cache = CacheManager::new(dir.path(), test_config()).await.unwrap();

    let basic_path = repo_root().join("modules/basic");
    let env = Environment::new(basic_path).await.unwrap();

    let module_artifacts = env.package(true).await.unwrap();

    // Extract original WASM bytes
    let source = module_artifacts
        .iter()
        .find_map(|a| match a {
            Artifacts::Module(Module::Source(s)) => Some(s),
            _ => None,
        })
        .expect("Expected ModuleSource artifact");
    let binary = module_artifacts
        .iter()
        .find_map(|a| match a {
            Artifacts::Module(Module::Binary(b)) => Some(b),
            _ => None,
        })
        .expect("Expected ModuleBinary artifact");

    let original_wasm = binary.wasm.clone();

    // Write to disk
    for artifact in &module_artifacts {
        cache.write_artifacts(artifact).await.unwrap();
    }

    let hash = source.hash();
    let mod_dir = cache.root.join("anon").join(&hash);

    // 1. Verify exact match against file on disk
    let disk_wasm = std::fs::read(mod_dir.join("mod.wasm")).unwrap();
    assert_eq!(
        original_wasm, disk_wasm,
        "WASM on disk does not match original memory representation"
    );

    // 2. Verify exact match after Artifacts::from_dir read
    let loaded_artifact = ModuleBinary::from_dir(&mod_dir).await.unwrap();

    assert_eq!(
        original_wasm, loaded_artifact.wasm,
        "WASM reloaded from dir does not match original"
    );
}

#[tokio::test]
async fn test_capability_lib_exact_match() {
    let dir = TempDir::new().unwrap();
    let cache = CacheManager::new(dir.path(), test_config()).await.unwrap();

    let httpc_path = repo_root().join("capabilities/httpc");
    let env = Environment::new(httpc_path).await.unwrap();

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
        original_lib_bytes, loaded_artifact.libs[0].to_vec(),
        "Capability library bytes do not match after roundtrip to disk"
    );
}

#[tokio::test]
async fn test_artifact_tarball_roundtrips() {
    let basic_path = repo_root().join("modules/basic");
    let env = Environment::new(basic_path).await.unwrap();

    let module_artifacts = env.package(true).await.unwrap();

    // Serialize to .tar.gz buffer
    // Pick one to test tarball
    let source_artifact = module_artifacts
        .iter()
        .find(|a| matches!(a, Artifacts::Module(Module::Source(_))))
        .unwrap();
    let tarball_bytes = source_artifact
        .to_tarball()
        .expect("Failed to create tarball");

    // Deserialize back into memory
    let unpacked = Artifacts::from_tarball(&tarball_bytes).expect("Failed to unpack tarball");

    match unpacked {
        Artifacts::Module(Module::Source(m)) => {
            let original_source = match source_artifact {
                Artifacts::Module(Module::Source(orig)) => &orig.source,
                _ => unreachable!(),
            };
            assert_eq!(
                &m.source, original_source,
                "Source code corrupted after tarball extraction"
            );
        }
        _ => panic!("Expected unpacked tarball to be a ModuleSource"),
    }
}
