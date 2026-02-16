#[pyroduct::magma]
struct ProcessResult {
    status: String,
    count: u32,
}

#[pyroduct::module(output = ProcessResult)]
fn call(input: &str) -> anyhow::Result<ProcessResult> {
    Ok(ProcessResult {
        status: "ok".to_string(),
        count: input.len() as u32,
    })
}

fn main() {
    let _ = call("test");
}
