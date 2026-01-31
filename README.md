# Pyroduct

A Rust framework for building sandboxed WASM modules with host capabilities. Modules run in WebAssembly isolation while accessing native functionality (HTTP, serial ports, CPU info, etc.) through a capability system.

## Quick Start

```bash
# Enter dev shell
nix develop

# Run example modules
nix run .#run-tests

# Build everything
nix build
```

## Architecture

```
┌────────────────────────────────────────────────────────┐
│                        Host                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Reporter   │  │ HTTP Client │  │  CPU Info   │     │
│  │  Capability │  │  Capability │  │  Capability │     │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘     │
│         │                │                │            │
│  ═══════╪════════════════╪════════════════╪══════════  │
│         │          FFI Boundary           │            │
│  ═══════╪════════════════╪════════════════╪══════════  │
│         │                │                │            │
│  ┌──────┴────────────────┴────────────────┴──────┐     │
│  │              WASM Module (Sandboxed)          │     │
│  │                                               │     │
│  │   fn call(input) -> Result<Output, String>    │     │
│  └───────────────────────────────────────────────┘     │
└────────────────────────────────────────────────────────┘

```

## Writing a Module

Modules are WASM binaries that process inputs and return outputs. Use the `#[pyroduct::module]` macro to generate the FFI boilerplate.

### Simple Module

```rust
use pyroduct::*;

#[pyroduct::module(output = message)]
fn call(input: &str) -> Result<String, String> {
    Ok(format!("Hello, {}", input))
}

```

### Multiple Outputs

```rust
use pyroduct::*;

#[pyroduct::module(output = (count, data))]
fn process(input: &str) -> Result<(u32, Vec<u8>), String> {
    Ok((input.len() as u32, input.as_bytes().to_vec()))
}

```

### Struct Output

```rust
use pyroduct::*;

#[derive(ToRow)]
struct ProcessResult {
    count: u32,
    data: Vec<u8>,
}

#[pyroduct::module(output = ProcessResult)]
fn process(input: &str) -> Result<ProcessResult, String> {
    Ok(ProcessResult { count: 42, data: vec![] })
}

```

### Using Capabilities

```rust
use http_client::{HttpClient, HttpClientMethods};

#[pyroduct::module(output = response)]
fn call(url: &str) -> Result<String, String> {
    // Initialize the capability client
    let client = HttpClient.register()?;
    
    // Call capability methods
    let response = client.get(url)?;
    
    Ok(response)
}

```

## Writing a Capability

Capabilities expose native functionality to WASM modules using a unified macro system. You define a server struct (Host), a client struct (WASM), and an optional config struct.

### Components

1. **Config**: `#[pyroduct::config]` (Optional) - Configuration passed from Host to Capability on startup.
2. **Client**: `#[pyroduct::client]` - State passed from WASM to Host during calls.
3. **Server**: `#[pyroduct::capability]` - The implementation block defining lifecycle and logic.

### Example: HTTP Client

```rust
use pyroduct;

// 1. Configuration (Optional)
#[pyroduct::config]
pub struct HttpConfig {
    pub timeout_ms: u64,
}

// 2. Client State
#[pyroduct::client]
pub struct HttpClient;

// 3. Server Implementation
pub struct HttpServer {
    timeout: std::time::Duration,
}

#[pyroduct::capability]
impl HttpServer {
    // Required associated types
    type Client = HttpClient;
    type Config = HttpConfig; 
    type Error = String; // Optional, makes methods return Result<T, String>
    
    // Lifecycle: Initialize
    fn new(config: Option<HttpConfig>) -> Self {
        let config = config.unwrap_or(HttpConfig { timeout_ms: 30000 });
        Self {
            timeout: std::time::Duration::from_millis(config.timeout_ms),
        }
    }
    
    // Lifecycle: Reset state between module calls
    fn reset(&mut self) {}
    
    // Lifecycle: Register a new client instance
    fn new_client(&self, _client: &HttpClient) -> Result<(), String> {
        Ok(())
    }
    
    // Capability Method: Exposed to WASM
    // Must take &self and client: &Client
    async fn get(&self, _client: &HttpClient, url: String) -> Result<String, String> {
        // Implementation...
        Ok(format!("Fetched {}", url))
    }
}

```

## Adding a New Capability

To add a new capability to the project:

1. **Create Directory**: Create a folder in `capabilities/`, e.g., `capabilities/my_cap`.
2. **Create Definition**: Add `capability.nix` in that folder:
```nix
{ myLib }:
myLib.buildCapability {
  name = "my_cap";
  src = ./.;
  # Native dependencies (e.g. tokio, reqwest)
  hostDependencies = [
    { name = "tokio"; version = "1.49.0"; features = ["full"]; }
  ];
}

```


3. **Implement**: Create `src/lib.rs` with the Rust implementation using the macros described above.
4. **Register**: Add the capability to `flake.nix` in the `capabilities` set:
```nix
capabilities = {
  # ... existing capabilities ...
  my_cap = (import ./capabilities/my_cap/capability.nix { inherit myLib; });
};

```


5. **Generate Cargo Files**: Run the generator to create `Cargo.toml` for your new crate.
```bash
nix run .#generate-cargo-toml

```



## Project Structure

```
.
├── lib/
│   ├── pyroduct/          # Core library
│   ├── arrow-scalars/     # Arrow serialization
│   ├── module-derive/     # #[module] macro
│   └── capability-derive/ # Capability macros
├── capabilities/          # Capability implementations
│   ├── cpu_client/
│   ├── http_client/
│   ├── rag/
│   └── serial_client/
├── modules/               # Example WASM modules
│   ├── basic/
│   ├── basic_capability/
│   └── struct_io/
└── flake.nix              # Nix build definition

```

## Derive Macros Reference

| Macro | Attribute | Description |
| --- | --- | --- |
| `#[pyroduct::module]` | `output = ...` | Generates WASM entry point. Output can be a field name `val`, a tuple `(a, b)`, or a struct `MyStruct`. |
| `#[pyroduct::capability]` |  | Applied to the `impl` block of the server struct. Generates FFI glue and Client traits. |
| `#[pyroduct::client]` |  | Marks a struct as the Client state. Adds serialization. |
| `#[pyroduct::config]` |  | Marks a struct as the Capability configuration. Adds serialization. |
| `#[derive(ToRow)]` |  | Implements serialization for a struct to be returned to the host or passed to a capability. |
| `#[derive(FromRow)]` |  | Implements deserialization for a struct coming from the host (module input). |
| `#[derive(DeepRef)]` |  | Generates a zero-copy view struct (e.g., `MyStructRef`) for reading inputs efficiently. |

## Building with Nix

Capabilities and modules are defined in `.nix` files.

**capability.nix**

```nix
{ myLib }:
myLib.buildCapability {
  name = "proto_reporter";
  src = ./.;
  hostDependencies = [];
}

```

**module.nix**

```nix
{ myLib }:
myLib.buildModule {
  name = "proto_module";
  src = ./.;
  capabilities = [
    { path = "../../capabilities/proto_reporter"; }
  ];
}

```

## Running Modules

The harness runs modules with a JSON config:

```json
{
  "module_name": "proto_module",
  "module": "/path/to/module.wasm",
  "capabilities": ["/path/to/libreporter.so"],
  "inputs": [
    { "input": "Hello World" }
  ]
}

```

```bash
harness config.json

```

## Available Commands

```bash
nix develop                    # Enter dev shell
nix run .#run-tests            # Run all example modules
nix run .#generate-cargo-toml  # Generate Cargo.toml for IDE support
nix build .#harness            # Build the harness
nix build .#basic              # Build a specific module
nix build .#http_client        # Build a specific capability

```