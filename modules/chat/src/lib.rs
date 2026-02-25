use ollama::{OllamaClient, ChatMessage, ChatMessageRef, OllamaClientMethods};


/// Takes an array of chat messages, sends them to the HF LLM capability,
/// and returns the assistant's reply along with the full updated history.
#[pyroduct::module(output = response)]
fn process<'a>(
    input: Vec<ChatMessage>,
) -> Result<ChatMessage> {

    // 1. Register a client with the HF LLM capability
    let llm = OllamaClient {
        model: "gemma3:270m".to_string(),
        temperature: 0.7,
    }
    .register()?;

    // 3. Call the capability
    let reply = llm.chat(input.to_owned())?;

    Ok(reply)
}