
/// You can do a basic transform on the data without relying on a capability
#[pyroduct::module(output = CpuInfo)]
fn prefix(input: &str) -> Result<String, String> {
    Ok(format!("Prefixed: {input}"))
}