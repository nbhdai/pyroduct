#[pyroduct::module(output = result)]
fn call(port: &str, baud: u32, command: &str, data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(port.as_bytes());
    out.extend_from_slice(&baud.to_le_bytes());
    out.extend_from_slice(command.as_bytes());
    out.extend_from_slice(data);
    Ok(out)
}

fn main() {}