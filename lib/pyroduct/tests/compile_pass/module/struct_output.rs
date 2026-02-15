#[pyroduct::bridgeable]
struct ProcessResult {
    status: String,
    count: u32,
}

#[pyroduct::module(output = ProcessResult)]
fn call(input: &str) -> Result<ProcessResult, String> {
    Ok(ProcessResult {
        status: "ok".to_string(),
        count: input.len() as u32,
    })
}

fn main() {
    let _ = call("test");
}
