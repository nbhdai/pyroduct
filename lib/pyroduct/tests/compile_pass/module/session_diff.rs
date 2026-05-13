use pyroduct::session::SessionResponse;


#[pyroduct::magma]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[pyroduct::module(session, output = response)]
fn process<'a>(
    prior: Vec<(ChatMessage, String)>,
    input: ChatMessage,
) -> Result<SessionResponse<String>> {
    Ok(SessionResponse::End)
}