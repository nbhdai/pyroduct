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
┌─────────────────────────────────────────────────────────┐
│                        Host                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Reporter   │  │ HTTP Client │  │  CPU Info   │     │
│  │  Capability │  │  Capability │  │  Capability │     │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘     │
│         │                │                │             │
│  ═══════╪════════════════╪════════════════╪═══════════  │
│         │          FFI Boundary           │             │
│  ═══════╪════════════════╪════════════════╪═══════════  │
│         │                │                │             │
│  ┌──────┴────────────────┴────────────────┴──────┐     │
│  │              WASM Module (Sandboxed)          │     │
│  │                                                │     │
│  │   fn call(input) -> Result<Output, String>    │     │
│  └────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────┘
```

## Writing a Module

Modules are WASM binaries that process inputs and return outputs. Use the `#[module]` macro to generate the FFI boilerplate.

### Simple Module

```rust
use pyroduct::*;

#[module(output = message)]
fn call(input: &str) -> Result<String, String> {
    Ok(format!("Hello, {}", input))
}
```

### Multiple Outputs

```rust
use pyroduct::*;

#[module(output = (count, data))]
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

#[module(output = ProcessResult)]
fn process(input: &str) -> Result<ProcessResult, String> {
    Ok(ProcessResult { count: 42, data: vec![] })
}
```

### Multiple Inputs

```rust
use pyroduct::*;

#[module(output = result)]
fn call(port: &str, baud: u32, command: &str) -> Result<Vec<u8>, String> {
    // Process multiple typed inputs
    Ok(vec![])
}
```

### Using Capabilities

```rust
use proto_reporter::report;
use proto_http_client::HttpClient;

#[module(output = cpu_count)]
fn call(url: &str) -> Result<u32, String> {
    // Call a stateless capability
    let cpus = proto_cpu_info::get_cpu_count();
    
    // Use a stateful capability  
    let client = HttpClient::new(url);
    let response = client.get("/")?;
    
    // Report to the host
    report(format!("Got {} bytes", response.body.len()));
    
    Ok(cpus)
}
```

## Writing a Capability

Capabilities expose native functionality to WASM modules. There are three patterns.

### Pattern 1: Stateless Functions

For simple functions with no state:

```rust
use capability_derive::*;

#[capability_function]
pub fn get_cpu_count() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

capability_export!(env = "cpu_info", functions = [get_cpu_count]);
```

### Pattern 2: Host State Only

The server maintains state across calls, client is stateless:

```rust
use capability_derive::*;

#[capability]
pub trait Reporter {
    fn report(&mut self, message: String) -> String;
}

#[cfg(not(target_arch = "wasm32"))]
mod server {
    use super::*;
    
    #[capability_server(service = Reporter, config = ReporterConfig)]
    pub struct ReporterServer {
        logs: VecDeque<String>,
    }

    impl ReporterServerInit for ReporterServer {
        fn new() -> Self {
            Self { logs: VecDeque::new() }
        }
        
        fn with_config(config: ReporterConfig) -> Self {
            Self { logs: VecDeque::with_capacity(config.max_history) }
        }
        
        fn reset(&mut self) {
            self.logs.clear();
        }
    }

    impl Reporter for ReporterServer {
        fn report(&mut self, message: String) -> String {
            self.logs.push_back(message.clone());
            format!("Logged: {}", message)
        }
    }
    
    capability_export!(env = "reporter", ReporterServer);
}
```

### Pattern 3: Client State (Stateless Server)

Client holds configuration, server is stateless per-request:

```rust
use capability_derive::*;

#[capability_client]
#[derive(Debug, Clone)]
pub struct HttpClient {
    pub base_url: String,
    pub timeout_secs: Option<u64>,
}

#[capability(stateless)]
pub trait Http {
    async fn get(#[client_state] client: &HttpClient, path: &str) -> Result<HttpResponse, String>;
}

#[cfg(not(target_arch = "wasm32"))]
mod server {
    use super::*;
    
    #[capability_server(service = Http, stateless)]
    pub struct HttpServer;

    impl Http for HttpServer {
        async fn get(client: &HttpClient, path: &str) -> Result<HttpResponse, String> {
            let url = format!("{}{}", client.base_url, path);
            // Make actual HTTP request...
            Ok(HttpResponse { status: 200, headers: vec![], body: vec![] })
        }
    }
    
    capability_export!(env = "http_client", HttpServer);
}
```

## Project Structure

```
.
├── lib/
│   ├── pyroduct/          # Core library
│   ├── arrow-scalars/     # Arrow serialization
│   ├── module-derive/     # #[module] macro
│   └── capability-derive/ # Capability macros
├── proto/
│   ├── capabilities/      # Example capabilities
│   │   ├── reporter/
│   │   ├── cpu_info/
│   │   ├── http_client/
│   │   └── serial_client/
│   └── modules/           # Example modules
│       ├── module/        # Uses reporter
│       ├── module_2/      # Uses cpu_info + http_client
│       └── module_3/      # Uses serial_client
└── flake.nix
```

## Building with Nix

Capabilities and modules are defined in `.nix` files:

**capability.nix**
```nix
{ myLib }:

myLib.buildCapability {
  name = "proto_reporter";
  src = ./.;
  hostDependencies = [
    { name = "tracing"; version = "0.1"; }
  ];
}
```

**module.nix**
```nix
{ myLib, capabilities }:

myLib.buildModule {
  name = "proto_module";
  src = ./.;
  capabilities = [ capabilities.proto_reporter ];
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
    { "input": "Hello World" },
    { "input": "Second input" }
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
nix run .#show-cargo-toml      # Preview generated Cargo.toml
nix build .#harness            # Build the harness
nix build .#proto_module       # Build a specific module
nix build .#proto_reporter     # Build a specific capability
```

## Derive Macros Reference

| Macro | Purpose |
|-------|---------|
| `#[module(output = ...)]` | Generate WASM entry point for a module |
| `#[capability]` | Define a capability trait |
| `#[capability_function]` | Mark a standalone function as a capability |
| `#[capability_client]` | Mark a struct as client-side state |
| `#[capability_server(...)]` | Mark a struct as the server implementation |
| `#[capability_impl(...)]` | Generate FFI for a trait impl |
| `capability_export!(...)` | Generate plugin manifest |
| `#[derive(ToRow)]` | Serialize struct to Arrow row |
| `#[derive(FromRow)]` | Deserialize struct from Arrow row |
| `#[derive(DeepRef)]` | Generate `*Ref` type for zero-copy access |