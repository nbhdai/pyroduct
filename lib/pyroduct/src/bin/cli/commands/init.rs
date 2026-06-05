use anyhow::{Context, Result};
use fs_err as fs;
use heck::AsPascalCase;
use std::path::{Path, PathBuf};

pub fn init(path: Option<PathBuf>, is_cap: bool) -> Result<()> {
    let (root, name) = match path {
        Some(p) => {
            let name = p
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid path: cannot determine folder name"))?
                .to_string();
            if !p.exists() {
                fs::create_dir_all(&p).context("Failed to create project directory")?;
            }
            (p, name)
        }
        None => {
            let p = std::env::current_dir().context("Failed to get current directory")?;
            let name = p
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project")
                .to_string();
            (p, name)
        }
    };
    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).context("Failed to create src directory")?;

    if is_cap {
        create_capability(&root, &src_dir, &name)?;
        tracing::info!("Created capability '{}' at {:?}", name, root);
    } else {
        create_module(&root, &src_dir, &name)?;
        tracing::info!("Created module '{}' at {:?}", name, root);
    }

    Ok(())
}

pub fn create_module(root: &Path, src: &Path, name: &str) -> Result<()> {
    let toml_content = format!(
        r#"[module]
name = "{name}"
version = "0.1.0"
edition = "2024"
authors = ["Your Name"]

[pyroduct]
path = "../../lib/pyroduct"

[capabilities]

[dependencies]
"#
    );

    fs::write(root.join("Module.toml"), toml_content)?;

    let lib_rs = r#"use pyroduct::module;

#[module(output = message)]
fn call(input: &str) -> Result<String> {
    Ok(format!("Hello from module: {}", input))
}
"#;
    fs::write(src.join("lib.rs"), lib_rs)?;
    Ok(())
}

pub fn create_capability(root: &Path, src: &Path, name: &str) -> Result<()> {
    let toml_content = format!(
        r#"[capability]
name = "{name}"
version = "0.1.0"
edition = "2024"
authors = ["Your Name"]

[pyroduct]
path = "../../lib/pyroduct"

[dependencies.host]

[dependencies.module]

[dependencies.shared]
"#
    );

    fs::write(root.join("Capability.toml"), toml_content)?;

    let pascal_name = AsPascalCase(name).to_string();

    let lib_rs = format!(
        r#"#[pyroduct::config]
pub struct {0}Config {{
    pub timeout_ms: u64,
}}

#[pyroduct::magma]
pub struct {0}Client;

pub struct {0}Server;

#[pyroduct::capability]
impl {0}Server {{
    type Config = {0}Config;
    type Client = {0}Client;

    fn new(_config: Option<{0}Config>) -> Self {{
        Self
    }}

    fn reset(&mut self) {{}}

    fn register(&self, _client: &{0}Client) -> Result<(), ::pyroduct::CapturedError> {{
        Ok(())
    }}

    fn example_method(&self, _client: &{0}Client) -> Result<String, ::pyroduct::CapturedError> {{
        Ok("Hello from capability".to_string())
    }}
}}
"#,
        pascal_name
    );

    fs::write(src.join("lib.rs"), lib_rs)?;
    Ok(())
}
