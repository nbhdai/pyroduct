use pyroduct::session::SessionResponse;

#[pyroduct::magma]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[pyroduct::module(session, output = response)]
fn process<'a>(
    _prior_input: Vec<ChatMessage>,
    _prior_output: Vec<ChatMessage>,
    input: ChatMessage,
) -> Result<SessionResponse<ChatMessage>> {
    Ok(SessionResponse::Continue(ChatMessage {
        role: "assistant".to_string(),
        content: "hi".to_string(),
    }))
}

fn main() {
    let _ = process(vec![], vec![], ChatMessage { role: "user".into(), content: "hi".into() });
}