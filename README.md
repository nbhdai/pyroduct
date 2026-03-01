# Pyroduct

A Rust framework for building data pipelines from sandboxed WASM modules with host capabilities. Modules run in WebAssembly isolation while accessing native functionality (HTTP, RAG, state, etc.) through a capability system. Pipelines are defined in TOML/YAML and orchestrate multiple modules in sequence.

## Quick Start

```bash
# Create a new module
pyroduct init my_module

# Create a new capability
pyroduct init --cap my_capability

# Expand (generate Cargo.toml + FFI glue)
pyroduct expand my_module

# Package (compile to WASM / dylib)
pyroduct package my_module

# Run a pipeline
pyroduct run pipeline.yaml data.jsonl -o output/

# Interactive TUI
pyroduct tui pipeline.yaml data.jsonl
```

## Architecture

```
┌────────────────────────────────────────────────────────┐
│                        Host                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │    HTTP     │  │     RAG     │  │    State    │     │
│  │  Capability │  │  Capability │  │  Capability │     │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘     │
│         │                │                │            │
│  ═══════╪════════════════╪════════════════╪══════════  │
│         │            Boundary             │            │
│  ═══════╪════════════════╪════════════════╪══════════  │
│         │                │                │            │
│  ┌──────┴────────────────┴────────────────┴───────┐    │
│  │              WASM Module (Sandboxed)           │    │
│  │                                                │    │
│  │   fn call(input) -> Result<Output, String>     │    │
│  └────────────────────────────────────────────────┘    │
└────────────────────────────────────────────────────────┘
```

The `libraries` section points to capability directories (which contain `artifacts/lib.dylib` or `artifacts/lib.so`). The `modules` section points to module directories (which contain `artifacts/mod.wasm`). Each module can receive per-capability configuration. The `pipeline` array defines execution order.

### Simple Module

```rust
use pyroduct::module;

#[module(output = message)]
fn call(input: &str) -> Result<String> {
    Ok(format!("Hello, {}", input))
}
```

### Using Capabilities

```rust
use httpc::{HttpClient, HttpClientMethods};

#[pyroduct::module(output = response)]
fn call(url: &str) -> Result<String> {
    let client = HttpClient.register()?;
    let response = client.get(url)?;
    Ok(response)
}
```

## Writing a Capability

Capabilities expose native functionality to WASM modules. Define your project in `Capability.toml`:

Dependencies are split into three sections: `host` (only available on the native side, compiled as optional), `module` (available in WASM), and `shared` (available on both sides).

A capability has three components:

```rust
// 1. Configuration (Optional) — passed from pipeline config at startup
#[pyroduct::config]
pub struct TransformConfig { ... }

// 2. Client State — serialized across the FFI boundary
#[pyroduct::magma]
pub struct TransformClient { ... }

// 3. Server — the impl block with lifecycle + methods
pub struct TransformServer { ... }

#[pyroduct::capability]
impl TransformServer {
    type Client = TransformClient;
    type Config = TransformConfig;
    type Error = String;

    // Lifecycle: initialize (sync or async)
    async fn new(config: Option<TransformConfig>) -> Self { ... }

    // Lifecycle: reset state between pipeline invocations
    async fn reset(&mut self) {}

    // Lifecycle: validate a new client instance
    fn register(&self, client: &TransformClient) -> Result<(), String> { ... }

    // Methods: exposed to WASM, must take &self and &Client
    async fn transform(&self, client: &TransformClient, input: String) -> Result<String, String> { ... }
}
```

## CLI Reference

```
pyroduct init [PATH]            Create a new module project
pyroduct init --cap [PATH]      Create a new capability project
pyroduct expand <DIR>           Generate Cargo.toml and FFI glue from manifests
pyroduct expand -b <DIR>        Convert compiled artifacts to WAT / dump symbols
pyroduct package <DIR>          Compile module (.wasm) or capability (.dylib/.so)
pyroduct clean <DIR>            Remove generated files (Cargo.toml, artifacts/, target/)
pyroduct run <CONFIG> <INPUT>   Run a pipeline (file input = batch mode, JSON string = single row)
pyroduct tui <CONFIG> <INPUT>   Interactive TUI: edit code, configure capabilities, run pipeline
```

All commands support recursive mode: point them at a parent directory and they'll discover all `Module.toml` / `Capability.toml` projects in subdirectories.

`run` supports multiple output formats: `--format json` (default, JSONL), `csv`, `ipc` (Arrow), `parquet`.

## Macro Reference

| Macro                               | Description |
| ----------------------------------- | --------------- |
| `#[pyroduct::module(output = ...)]` | Generates WASM entry point. Output can be a field name, a tuple `(a, b)`, or a struct type. |
| `#[pyroduct::capability]`           | Applied to the server `impl` block. Generates host-side FFI glue. |
| `#[pyroduct::magma]`                | Marks a struct as the capability client state. Serialized across FFI. |
| `#[pyroduct::config]`               | Marks a struct as capability configuration. Deserialized from pipeline config. |