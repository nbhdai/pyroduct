use llm::{LlmClient, ChatMessage, ChatMessageRef, LlmClientMethods};
use pyroduct::session::SessionResponse;

/// Takes an array of chat messages, sends them to the HF LLM capability,
/// and returns the assistant's reply along with the full updated history.
#[pyroduct::module(session, output = ChatMessage)]
fn process<'a>(
    mut prior: Vec<ChatMessage>,
    input: ChatMessage,
) -> Result<SessionResponse<ChatMessage>> {

    // 1. Register a client with the HF LLM capability
    let llm = LlmClient {
        model: "gemma3:270m".to_string(),
        temperature: 0.7,
    }.register()?;

    prior.push(input);

    // 3. Call the capability
    let reply = llm.chat(prior)?;

    Ok(SessionResponse::Continue(reply))
}