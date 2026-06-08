use pyroduct::session::SessionResponse;

/// Takes an array of chat messages, sends them to the HF LLM capability,
/// and returns the assistant's reply along with the full updated history.
#[pyroduct::module(session, output = response)]
fn process<'a>(
    mut prior: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {

    prior.push(input);

    Ok(SessionResponse::Continue(prior.pop().unwrap()))
}

fn main() {
    let _ = process(vec![], "user".into() );
}