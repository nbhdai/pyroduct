use pyro_artifacts::{
    artifacts::{CapabilityConfig, ModuleDependencies, ModuleSource, Playbook},
    build::Builder,
    cache::{CacheManager, RemoteAddress},
    cargo::ResolvedCapability,
};
use pyroduct::transport::socket::PyroListener;
use pyroduct::transport::socket::capability::{PyroRouter, PyroServer};
use pyroduct::{PyroRow, pipeline::PipelineConfig};
use std::collections::{BTreeMap, HashMap};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

const CODE: &str = r#"
use config::{TransformClient, TransformClientMethods};

#[pyroduct::module(output = (original, transformed, transform_count))]
pub fn call(input: &str) -> Result<(String, String, u64)> {
    let client = TransformClient {
        prefix: "[TEST] ".to_string(),
    }.register()?;

    let original = input.to_string();
    let transformed = client.transform(input.to_string())?;
    let transform_count = client.get_transform_count()? as u64;

    Ok((original, transformed, transform_count))
}
"#;

#[tokio::test]
async fn test_socket_capability_remote() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "trace,cranelift_frontend=off,cranelift_codegen=off,wasmtime=off".into()
    });

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).pretty())
        .with(filter)
        .try_init();

    let cache = std::sync::Arc::new(CacheManager::from_env().await.unwrap());
    let builder = Builder::from_env(cache.clone()).await.unwrap();

    let source = ModuleSource {
        dependencies: ModuleDependencies {
            dependencies: BTreeMap::new(),
            capabilities: vec![ResolvedCapability {
                package: "config".to_string(),
                author: "nbhdai".to_string(),
                version: "0.1.0".to_string(),
            }],
        },
        source: CODE.to_string(),
        ident: None,
    };
    cache.remove_anon(&source.hash()).await.unwrap();

    let binary = builder
        .compile(&source)
        .await
        .expect("Valid module should compile");

    // 1. Setup Capability Server
    let lib_path = cache
        .capability_binary_path("nbhdai", "config", "0.1.0")
        .await
        .unwrap();

    let mut router =
        PyroRouter::load("config".into(), &lib_path).expect("Failed to load capability library");

    let cap_config = CapabilityConfig {
        classes: HashMap::from([(
            "transform".to_string(),
            Some(serde_json::json!({
                "uppercase": true,
                "suffix": "!!!"
            })),
        )]),
    };
    router
        .configure(&cap_config)
        .await
        .expect("Failed to configure capability");

    let server = PyroServer::new(router);
    let listener = PyroListener::bind_tcp("127.0.0.1:0")
        .await
        .expect("Failed to bind listener");
    let addr = listener.local_addr_tcp().expect("Failed to get local addr");

    // Run capability server in background
    tokio::spawn(async move {
        if let Err(e) = server.run(listener).await {
            eprintln!("Capability server error: {:?}", e);
        }
    });

    // 2. Setup Playbook and Pipeline Config
    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().to_path_buf();

    let config = PipelineConfig {
        playbook: Playbook {
            hash: binary.hash(),
            configurations: HashMap::new(),
        },
        remote: HashMap::from([("config".to_string(), RemoteAddress::Tcp(addr.to_string()))]),
        wal_capacity: 10,
        success_log_retention_secs: 3600,
        error_log_retention_secs: 86400 * 7,
        input_dir: tmp_path.clone(),
        output_dir: tmp_path.clone(),
        log_dir: tmp_path.clone(),
    };

    let loaded_config = config.load(&cache).await.unwrap();

    let factory = loaded_config.factory().unwrap();
    let mut pipeline = factory.build().await.unwrap();

    let input = PyroRow::from([("input", "hello".into())]);
    let result = pipeline.process(0, &input).await.unwrap();

    let row = result.row().unwrap();
    let transformed = row.get_str("transformed").unwrap();
    assert_eq!(transformed, "[TEST] HELLO!!!");
}
