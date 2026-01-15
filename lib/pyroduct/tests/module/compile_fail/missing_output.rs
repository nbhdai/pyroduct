use pyroduct::module;

#[module]  // ERROR: missing `output = ...`
fn call(input: &str) -> Result<String, String> {
    Ok(input.to_string())
}

fn main() {}