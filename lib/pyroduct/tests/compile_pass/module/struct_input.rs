use pyroduct::Capture;

#[pyroduct::magma]
struct CallMessage {
    message: String,
    role: String,
}

#[pyroduct::module(output = (output, messages))]
fn process<'a>(input: Vec<CallMessageRef<'_>>) -> Result<(String, Vec<CallMessage>)> {
    let output = input.first().capture("Empty chat history")?;
    Ok((
        output.message.to_string(),
        vec![
            CallMessage {
                message: "hi".to_string(),
                role: "user".to_string(),
            },
            CallMessage {
                message: "How can I help?".to_string(),
                role: "agent".to_string(),
            },
        ],
    ))
}

fn main() {
    let _ = process(vec![
        CallMessageRef {
            message: "hi",
            role: "user",
        },
        CallMessageRef {
            message: "How can I help?",
            role: "agent",
        },
    ]);
}
