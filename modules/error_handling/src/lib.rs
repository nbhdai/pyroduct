use pyroduct;

#[pyroduct::module(output = message)]
pub fn call(input: String) -> Result<String> {
    if input == "panic" {
        panic!("intentional panic");
    }
    if input == "error" {
        return Err(pyroduct::capture!("intentional error"));
    }
    Ok(format!("Success: {}", input))
}
