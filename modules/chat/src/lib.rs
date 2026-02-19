use ollama::{OllamaClient, ChatMessage, ChatMessageRef, OllamaClientMethods};


/// Takes an array of chat messages, sends them to the HF LLM capability,
/// and returns the assistant's reply along with the full updated history.
#[pyroduct::module(output = history)]
fn process<'a>(
    input: Vec<ChatMessage>,
) -> Result<Vec<ChatMessage>> {

    // 1. Register a client with the HF LLM capability
    let llm = OllamaClient {
        model: "gemini".to_string(),
        temperature: 0.7,
    }
    .register()?;

    // 3. Call the capability
    let reply = llm.chat(input.to_owned())?;

    // 4. Build the full history (input messages + assistant reply)
    let mut history: Vec<ChatMessage> = input
        .iter()
        .map(|m| ChatMessage {
            role: m.role.to_string(),
            content: m.content.to_string(),
        })
        .collect();
    history.push(reply.clone());

    Ok(history)
}