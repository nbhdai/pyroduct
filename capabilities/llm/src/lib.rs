pyroduct::library!();

// =============================================================================
// Config
// =============================================================================

/// Configuration for the Llm capability.
#[pyroduct::config]
pub struct LlmConfig {
    /// Base URL of the Llm server (e.g. "http://localhost:11434")
    pub base_url: String,
    /// Optional system prompt prepended to every chat request.
    pub system_prompt: String,
    /// Permitted Models
    pub permitted_models: Vec<String>,
}

// =============================================================================
// Client
// =============================================================================

/// Per-client state: which model to talk to and optional system prompt.
#[pyroduct::magma]
pub struct LlmClient {
    /// The model name to use (e.g. "llama3", "mistral", "codellama").
    pub model: String,
    /// Temperature for generation (0.0 - 2.0). 0 = deterministic.
    pub temperature: f32,
}

// =============================================================================
// Serde types for the Llm HTTP API
// =============================================================================

#[pyroduct::magma]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(serde::Serialize)]
pub struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    temperature: f32,
}

#[derive(serde::Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(serde::Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(serde::Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    temperature: f32,
}

#[derive(serde::Deserialize)]
struct GenerateResponse {
    choices: Vec<CompletionChoice>,
}

#[derive(serde::Deserialize)]
struct CompletionChoice {
    text: String,
}

// =============================================================================
// Server (capability impl)
// =============================================================================

pub struct LlmServer {
    base_url: String,
    http: reqwest::Client,
    permitted_models: Vec<String>,
    system_prompt: String,
}

/// Llm LLM capability — chat with models hosted on standard OpenAI-compatible endpoints.
#[pyroduct::capability]
impl LlmServer {
    type Client = LlmClient;
    type Config = LlmConfig;

    async fn new(config: Option<LlmConfig>) -> Result<Self> {
        let config = config.unwrap_or(LlmConfig {
            base_url: "http://localhost:11434".to_string(),
            permitted_models: Vec::new(),
            system_prompt: String::new(),
        });
        let http = reqwest::Client::builder()
            .build()
            .expect("failed to build reqwest client");
        let base_url = config.base_url.clone();
        let permitted_models = config.permitted_models.clone();
        let system_prompt = config.system_prompt.clone();
        Ok(Self {
            base_url,
            http,
            permitted_models,
            system_prompt,
        })
    }

    async fn reset(&mut self) -> Result<(), CapturedError> {
        Ok(())
    }

    fn register(&self, client: &LlmClient) -> Result<(), CapturedError> {
        if self.permitted_models.contains(&client.model) || self.permitted_models.is_empty() {
            Ok(())
        } else {
            pyroduct::bail!("Not Permitted, try: {:?}", self.permitted_models)
        }
    }

    /// Single-turn generate: send a prompt, get a completion.
    async fn generate(
        &self,
        client: &LlmClient,
        prompt: String,
    ) -> Result<String, CapturedError> {
        let prompt = if self.system_prompt.is_empty() {
            prompt
        } else {
            format!("{}\n{}", self.system_prompt, prompt)
        };

        let body = GenerateRequest {
            model: client.model.clone(),
            prompt,
            stream: false,
            temperature: client.temperature,
        };

        let     req = self.http.post(format!("{}/completions", self.base_url))
            .json(&body);

        let resp: reqwest::Response = req.send()
            .await
            .map_err(|e| pyroduct::capture!("Llm request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            pyroduct::bail!("Llm returned {}: {}", status, text);
        }

        let parsed: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| pyroduct::capture!("Failed to parse Llm response: {}", e))?;

        let response = parsed.choices.into_iter().next()
            .ok_or_else(|| pyroduct::capture!("Llm returned empty choices"))?
            .text;

        Ok(response)
    }

    /// Multi-turn chat: send a JSON-encoded array of `{"role":"...","content":"..."}`
    /// messages and get the assistant's reply.
    async fn chat(
        &self,
        client: &LlmClient,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatMessage, CapturedError> {
        let mut messages = messages;

        // Prepend system prompt if configured and not already present
        if !self.system_prompt.is_empty() {
            let has_system = messages.first().is_some_and(|m| m.role == "system");
            if !has_system {
                messages.insert(
                    0,
                    ChatMessage {
                        role: "system".to_string(),
                        content: self.system_prompt.clone(),
                    },
                );
            }
        }

        let body: ChatRequest = ChatRequest {
            model: client.model.clone(),
            messages,
            stream: false,
            temperature: client.temperature,
        };

        let req = self.http.post(format!("{}/chat/completions", self.base_url))
            .json(&body);

        let resp: reqwest::Response = req.send()
            .await
            .map_err(|e| pyroduct::capture!("Llm chat request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            pyroduct::bail!("Llm returned {}: {}", status, text);
        }

        let parsed: ChatResponse = resp
            .json()
            .await
            .map_err(|e| pyroduct::capture!("Failed to parse Llm chat response: {}", e))?;

        let message = parsed.choices.into_iter().next()
            .ok_or_else(|| pyroduct::capture!("Llm returned empty choices"))?
            .message;

        Ok(message)
    }
}