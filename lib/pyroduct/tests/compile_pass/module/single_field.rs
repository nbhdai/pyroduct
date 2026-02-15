#[pyroduct::module(output = message)]
fn call(input: &str) -> Result<String, String> {
    Ok(format!("Hello, {}", input))
}

fn main() {
    let _ = call("world");
}