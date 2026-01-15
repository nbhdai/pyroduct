use pyroduct::{module, FromRow, DeepRef, ToRow};

#[derive(FromRow, DeepRef, ToRow)]
struct CallMessage {
    message: String,
    role: String,
}

#[module(output = (output, messages))]
fn process<'a>(input: &[CallMessageRef<'_>]) -> Result<(String, Vec<CallMessage>), String> {
    let output = input.first().ok_or("Empty chat history".to_string())?;
    Ok((
        output.message.to_string(),
        vec![
            CallMessage { message: "hi".to_string(), role: "user".to_string() },
            CallMessage { message: "How can I help?".to_string(), role: "agent".to_string() },
        ],
    ))
}

fn main() {
    let _ = process(
        &[
            CallMessageRef { message: "hi", role: "user" },
            CallMessageRef { message: "How can I help?", role: "agent" },
        ],
    );
}