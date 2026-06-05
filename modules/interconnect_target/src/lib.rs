use pyroduct;

#[pyroduct::module(output = message)]
pub fn call(input: String) -> Result<String> {
    Ok(format!("Hello: {}", input))
}
