use pyroduct::{
    ModIdentity, PyroductResult,
    arrow_scalars::ArrowRow,
    host::{Capabilities, pipeline::CapabilityDef, wasm_execute::Harness},
};
use serde_json::json;
use std::path::Path;
use wasmtime::{Config, Engine};

/// Test that capability state is preserved across multiple calls to the same module instance.
#[tokio::test]
async fn test_capability_state_preservation() -> PyroductResult<()> {
    let mut caps = Capabilities::new();
    // Use the counter capability from tests/cap_config
    #[cfg(target_os = "linux")]
    let cap_path = Path::new("../../capabilities/state/artifacts/lib.so");
    #[cfg(target_os = "macos")]
    let cap_path = Path::new("../../capabilities/state/artifacts/lib.dylib");
    caps.load("counter", cap_path)?;

    let wasm_path = Path::new("../../modules/cap_state/artifacts/mod.wasm");
    let wasm_bytes = std::fs::read(wasm_path).unwrap();

    let mod_ident = ModIdentity::from(wasm_path);
    let cap_defs = vec![CapabilityDef {
        name: "counter".to_string(),
        config: None,
    }];

    let harness_state = caps.init(&mod_ident, &cap_defs).await?;
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config).unwrap();
    let mut harness = Harness::new(&engine, &wasm_bytes, &caps, harness_state).await?;

    // First call: Increment counter (start_value: 0)
    let input1 = ArrowRow::from([("input", "0".into())]);
    let result1 = harness.process(&input1).await?.unwrap();
    // Result should be (count: 0, incremented: 0) since fetch_add returns previous
    assert_eq!(result1.get_u64("incremented").unwrap(), 0);

    // Second call: The call_count in CounterServer should now be 1
    let input2 = ArrowRow::from([("input", "0".into())]);
    let result2 = harness.process(&input2).await?.unwrap();
    assert_eq!(result2.get_u64("incremented").unwrap(), 1);

    Ok(())
}

/// Test that capability configurations (passed via PipelineDef) are correctly respected by the server.
#[tokio::test]
async fn test_capability_configuration_respect() -> PyroductResult<()> {
    let mut caps = Capabilities::new();
    // Use the transform capability from capabilities/state
    #[cfg(target_os = "linux")]
    let cap_path = Path::new("../../capabilities/config/artifacts/lib.so");
    #[cfg(target_os = "macos")]
    let cap_path = Path::new("../../capabilities/config/artifacts/lib.dylib");
    caps.load("transform", cap_path)?;

    let wasm_path = Path::new("../../modules/cap_config/artifacts/mod.wasm");
    let wasm_bytes = std::fs::read(wasm_path).unwrap();

    let mod_ident = ModIdentity::from(wasm_path);

    // Configure the capability to uppercase and add a specific suffix
    let cap_defs = vec![CapabilityDef {
        name: "transform".to_string(),
        config: Some(json!({
            "uppercase": true,
            "suffix": "!!!"
        })),
    }];

    let harness_state = caps.init(&mod_ident, &cap_defs).await?;
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config).unwrap();
    let mut harness = Harness::new(&engine, &wasm_bytes, &caps, harness_state).await?;

    // The module adds "[TEST] " prefix. With config, result should be:
    // "[TEST] HELLO!!!"
    let input = ArrowRow::from([("input", "hello".into())]);
    let result = harness.process(&input).await?.unwrap();

    let transformed = result.get_str("transformed").unwrap();
    assert_eq!(transformed, "[TEST] HELLO!!!");

    Ok(())
}
