#[pyroduct::module(output = message)]
fn call(input: &str) -> anyhow::Result<String> {
    Ok(format!("Hello, {}", input))
}

fn main() {
    let _ = call("world");
}