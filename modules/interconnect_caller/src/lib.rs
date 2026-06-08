use pyroduct;
use pyroduct::call_playbook;
use pyroduct::format::PyroRow;

#[pyroduct::module(output = message)]
pub fn call(input: String) -> Result<String> {
    let target_input = PyroRow::from([("input", input.into())]);
    let (_session_id, target_output) = call_playbook("target", &target_input);
    let msg = target_output.get_str("message").unwrap();
    Ok(format!("Caller received: {}", msg))
}
