# Pyroduct

A Rust framework for building data pipelines from sandboxed WASM modules with host capabilities. Modules run in WebAssembly isolation while accessing native functionality (HTTP, RAG, state, etc.) through a capability system. Pipelines are defined in TOML/YAML and orchestrate multiple modules in sequence.

## Installation

### Prerequisites

Install the following before running the install script:

| Platform | Command |
|----------|---------|
| **Ubuntu/Debian** | `sudo apt install build-essential pkg-config libssl-dev libsqlite3-dev curl` |
| **Arch/Manjaro** | `sudo pacman -S base-devel openssl sqlite pkg-config curl` |
| **macOS (brew)** | `brew install openssl sqlite pkg-config` |
| **macOS (nix)** | Available via the Nix dev shell: `nix develop` |
| **NixOS** | Use the NixOS module below — no manual prerequisites needed |

A [Rust toolchain](https://rustup.rs) is also required. The install script will install it automatically if not present.

### Install Script

Clone the repo and run the install script. It compiles the CLI and daemon from source and optionally sets up a background service:

```bash
git clone https://github.com/nbhdai/pyroduct.git
cd pyroduct
./install.sh            # interactive
./install.sh -d         # use all defaults (installs daemon)
```

On **Linux**, the daemon runs as a systemd service under a dedicated `pyroduct` system user. On **macOS**, it runs as a launchd user agent.

### NixOS Module

Add pyroduct as a flake input and import the module:

```nix
# flake.nix
inputs.pyroduct.url = "github:nbhdai/pyroduct";

# configuration.nix
{ inputs, pkgs, ... }:
{
  imports = [ inputs.pyroduct.nixosModules.pyro-daemon ];

  services.pyro-daemon = {
    enable = true;
    package = inputs.pyroduct.packages.${pkgs.system}.pyroduct;
    members = [ "your-username" ];   # users who can access the daemon
    authorName = "your-name";
    buildSlots = 4;
  };
}
```

### Daemon Management

<details>
<summary>Linux (systemd)</summary>

```bash
systemctl status pyro-daemon          # check status
journalctl -u pyro-daemon -f          # follow logs
sudo systemctl restart pyro-daemon    # restart
```

**Paths:**
- Working dir: `/var/lib/pyro-daemon`
- Shared cache: `/var/lib/pyro-daemon/cache`
- Control socket: `/var/lib/pyro-daemon/control`

</details>

<details>
<summary>macOS (launchd)</summary>

```bash
launchctl print gui/$(id -u)/com.pyroduct.daemon    # check status
cat ~/Library/Logs/pyro-daemon/stderr.log            # view logs
launchctl kickstart gui/$(id -u)/com.pyroduct.daemon # restart
```

**Paths:**
- Working dir: `~/Library/Application Support/pyro-daemon`
- Shared cache: `~/Library/Application Support/pyro-daemon/cache`
- Control socket: `~/Library/Application Support/pyro-daemon/control`

</details>

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

    // Lifecycle: initialize (sync or async)
    async fn new(config: Option<TransformConfig>) -> Self { ... }

    // Lifecycle: reset state between pipeline invocations
    async fn reset(&mut self) {}

    // Lifecycle: validate a new client instance
    fn register(&self, client: &TransformClient) -> Result<(), pyroduct::CapturedError> { ... }

    // Methods: exposed to WASM, must take &self and &Client
    async fn transform(&self, client: &TransformClient, input: String) -> Result<String, pyroduct::CapturedError> { ... }
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