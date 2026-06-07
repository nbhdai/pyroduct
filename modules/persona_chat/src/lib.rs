use llm::{ChatMessage, LlmClient, LlmClientMethods};
use pyroduct;

#[pyroduct::module(output = conversation)]
pub fn run(persona: String, objective: String, turns: u32) -> Result<Vec<ChatMessage>> {
    let llm = LlmClient {
        model: "gemma-4-31B-it-Q8_0".to_string(),
        temperature: 0.7,
    }
    .register()?;

    let system_prompt = format!(
        "You are acting as the following persona:\n\
         {persona}\n\n\
         Your objective in this conversation is:\n\
         {objective}\n\n\
         Start the conversation and work to achieve your objective."
    );

    let mut persona_history = vec![ChatMessage {
        role: "system".to_string(),
        content: system_prompt,
    }];

    if turns == 0 {
        return Ok(Vec::new());
    }

    let mut conversation = Vec::new();

    // --- Turn 0 (Initial Turn) ---
    // 1. Get the Persona's initial response from LLM
    let persona_reply = llm.chat(persona_history.clone())?;
    persona_history.push(persona_reply.clone());

    let persona_msg = ChatMessage {
        role: "user".to_string(),
        content: persona_reply.content.clone(),
    };
    conversation.push(persona_msg);

    // 2. Call the chat module with call_playbook to start the session and retrieve the session ID
    let chat_input = pyroduct::PyroRow::from([
        ("role", "user".into()),
        ("content", persona_reply.content.clone().into()),
    ]);
    let (session_id, chat_output) = pyroduct::call_playbook("chat", &chat_input);

    // 3. Process the chat module's initial response
    let reply_content = match chat_output.get_str("content") {
        Some(content) => content,
        None => return Ok(conversation),
    };

    let chat_reply = ChatMessage {
        role: "user".to_string(),
        content: reply_content.to_string(),
    };
    persona_history.push(chat_reply);

    let chat_msg = ChatMessage {
        role: "assistant".to_string(),
        content: reply_content.to_string(),
    };
    conversation.push(chat_msg);

    // --- Subsequent Turns ---
    for _ in 1..turns {
        // 1. Get the Persona's response from LLM
        let persona_reply = llm.chat(persona_history.clone())?;
        persona_history.push(persona_reply.clone());

        let persona_msg = ChatMessage {
            role: "user".to_string(),
            content: persona_reply.content.clone(),
        };
        conversation.push(persona_msg);

        // 2. Call the chat module using call_session with the retrieved session ID
        let chat_input = pyroduct::PyroRow::from([
            ("role", "user".into()),
            ("content", persona_reply.content.clone().into()),
        ]);
        let chat_output = pyroduct::call_session("chat", session_id, &chat_input);

        // 3. Process the chat module's reply
        let reply_content = match chat_output.get_str("content") {
            Some(content) => content,
            None => {
                break;
            }
        };

        let chat_reply = ChatMessage {
            role: "user".to_string(),
            content: reply_content.to_string(),
        };
        persona_history.push(chat_reply);

        let chat_msg = ChatMessage {
            role: "assistant".to_string(),
            content: reply_content.to_string(),
        };
        conversation.push(chat_msg);
    }

    Ok(conversation)
}
