#[pyroduct::magma]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// Should throw a 
#[pyroduct::module(session, output = response)]
fn process<'a>(
    other: Vec<ChatMessage>,
    input: ChatMessage,
) -> Result<ChatMessage> {
    Ok(SessionResponse::Continue(ChatMessage {
        role: "assistant".to_string(),
        content: "hi".to_string(),
    }))
}

fn main() {
    let _ = process(vec![], ChatMessage { role: "user".into(), content: "hi".into() });
}