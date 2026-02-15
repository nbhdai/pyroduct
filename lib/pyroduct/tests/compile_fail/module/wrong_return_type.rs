use pyroduct::module;

#[module(output = value)]
fn call(input: &str) -> String {  // ERROR: must return Result<T, String>
    input.to_string()
}

fn main() {}