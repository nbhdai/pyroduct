use pyroduct::module;

#[module(output = (count, message, data))]
fn call(input: &str) -> Result<(u32, String, Vec<u8>), String> {
    Ok((input.len() as u32, input.to_string(), input.as_bytes().to_vec()))
}

fn main() {
    let _ = call("test");
}