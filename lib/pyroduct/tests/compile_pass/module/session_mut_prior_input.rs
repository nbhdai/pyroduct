use pyroduct::session::SessionResponse;

/// Takes an array of chat messages, sends them to the HF LLM capability,
/// and returns the assistant's reply along with the full updated history.
#[pyroduct::module(session, output = response)]
fn process<'a>(
    mut prior_input: Vec<String>,
    mut prior_output: Vec<String>,
    input: String,
) -> Result<SessionResponse<String>> {

    prior_input.push(input.clone());
    prior_output.push(input);

    Ok(SessionResponse::Continue(prior_input.pop().unwrap()))
}

fn main() {
    let _ = process(vec![], vec![], "hi".into());
}