use pyroduct::module;

#[module(output = (a, b))] // ERROR: tuple fields but return is not a tuple
fn call(input: &str) -> Result<String> {
    Ok(input.to_string())
}

fn main() {}
