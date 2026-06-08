use pyroduct::module;

#[module(output = (a, b, c))] // ERROR: 3 fields but tuple has 2 elements
fn call(input: &str) -> Result<(i32, String)> {
    Ok((1, input.to_string()))
}

fn main() {}
