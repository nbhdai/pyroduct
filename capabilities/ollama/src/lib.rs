use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pyroduct::library!();

// =============================================================================
// Config
// =============================================================================

/// Configuration for the Ollama capability.
#[pyroduct::config]
pub struct OllamaConfig {
    /// Base URL of the Ollama server (e.g. "http://localhost:11434")
    pub base_url: String,
    /// Default request timeout in milliseconds.
    pub timeout_ms: u64,
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
pub struct OllamaClient {
    /// The model name to use (e.g. "llama3", "mistral", "codellama").
    pub model: String,
    /// Temperature for generation (0.0 - 2.0). 0 = deterministic.
    pub temperature: f32,
}

// =============================================================================
// Serde types for the Ollama HTTP API
// =============================================================================

#[pyroduct::magma]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
}

#[derive(Serialize)]
struct ChatOptions {
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

// =============================================================================
// Server (capability impl)
// =============================================================================

pub struct OllamaServer {
    base_url: String,
    http: Mutex<reqwest::Client>,
    permitted_models: Vec<String>,
    system_prompt: String,
}

/// Ollama LLM capability — chat with models hosted on a local Ollama instance.
#[pyroduct::capability]
impl OllamaServer {
    type Client = OllamaClient;
    type Config = OllamaConfig;
    type Error = String;

    async fn new(config: Option<OllamaConfig>) -> Self {
        let config = config.unwrap_or(OllamaConfig {
            base_url: "http://localhost:11434".to_string(),
            timeout_ms: 120_000,
            permitted_models: Vec::new(),
            system_prompt: String::new(),
        });
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .expect("failed to build reqwest client");
        let permitted_models = config.permitted_models.clone();
        let system_prompt = config.system_prompt.clone();
        Self {
            base_url: config.base_url,
            http: Mutex::new(http),
            permitted_models,
            system_prompt,
        }
    }

    async fn reset(&mut self) {}

    fn register(&self, client: &OllamaClient) -> Result<(), String> {
        if self.permitted_models.contains(&client.model) || self.permitted_models.is_empty() {
            Ok(())
        } else {
            Err(format!("Not Permitted, try: {:?}", self.permitted_models))
        }
    }

    /// Single-turn generate: send a prompt, get a completion.
    async fn generate(
        &self,
        client: &OllamaClient,
        prompt: String,
    ) -> Result<String, String> {
        let system = if self.system_prompt.is_empty() {
            None
        } else {
            Some(self.system_prompt.clone())
        };

        let body = GenerateRequest {
            model: client.model.clone(),
            prompt,
            stream: false,
            system,
            options: Some(ChatOptions {
                temperature: client.temperature,
            }),
        };
        let http = self.http.lock().await;

        let resp: reqwest::Response = http
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama returned {status}: {text}"));
        }

        let parsed: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama response: {e}"))?;

        Ok(parsed.response)
    }

    /// Multi-turn chat: send a JSON-encoded array of `{"role":"...","content":"..."}`
    /// messages and get the assistant's reply.
    async fn chat(
        &self,
        client: &OllamaClient,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatMessage, String> {
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
            options: Some(ChatOptions {
                temperature: client.temperature,
            }),
        };

        let http = self.http.lock().await;
        let resp: reqwest::Response = http
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama chat request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama returned {status}: {text}"));
        }

        let parsed: ChatResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama chat response: {e}"))?;

        Ok(parsed.message)
    }
}