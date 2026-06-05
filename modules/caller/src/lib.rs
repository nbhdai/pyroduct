use pyroduct;
use pyroduct::call_session;
use pyroduct::format::PyroRow;
use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn caller(
    prior: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {
    let turn = (prior.len() as u32 + 1) / 2;
    let target_input = PyroRow::from([("input", input.into())]);
    let target_output = call_session("target", 42, &target_input);
    let msg = target_output.get_str("message").unwrap();
    match turn {
        0 => Ok(SessionResponse::Continue(format!("Caller turn 1: {}", msg))),
        1 => Ok(SessionResponse::End(format!("Caller turn 2: {}", msg))),
        _ => Ok(SessionResponse::Terminate),
    }
}
