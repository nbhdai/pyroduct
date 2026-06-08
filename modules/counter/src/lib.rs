use pyroduct::session::SessionResponse;

#[pyroduct::module(session, output = message)]
fn counter(prior: Vec<String>, _input: String) -> Result<SessionResponse<String>> {
    let turn = (prior.len() as u32 + 1) / 2;
    match turn {
        0 => Ok(SessionResponse::Continue(format!(
            "Hello! Turn {}",
            turn + 1
        ))),
        1 => Ok(SessionResponse::End(format!("Goodbye! Turn {}", turn + 1))),
        _ => Ok(SessionResponse::Terminate),
    }
}
