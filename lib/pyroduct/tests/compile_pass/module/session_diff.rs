use pyroduct::session::SessionResponse;


#[pyroduct::magma]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[pyroduct::module(session, output = response)]
fn process<'a>(
    prior_input: Vec<ChatMessage>,
    prior_output: Vec<String>,
    input: ChatMessage,
) -> Result<SessionResponse<String>> {
    Ok(SessionResponse::End)
}

fn main() {
    let _ = process(vec![], vec![], ChatMessage { role: "user".into(), content: "hi".into() });
}