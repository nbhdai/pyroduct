// Async capability - HTTP client with URL allowlist
#[pyroduct::config]
pub struct HttpConfig {
    pub timeout_ms: u64,
}


#[pyroduct::interface_item]
pub struct HttpClient;

pub struct HttpServer {
    timeout: std::time::Duration,
}

#[pyroduct::capability]
impl HttpServer {
    type Client = HttpClient;
    type Config = HttpConfig;
    type Error = String;
    
    async fn new(config: Option<HttpConfig>) -> Self {
        let config = config.unwrap_or(HttpConfig {
            timeout_ms: 30000,
        });
        Self {
            timeout: std::time::Duration::from_millis(config.timeout_ms),
        }
    }
    
    async fn reset(&mut self) {}
    
    fn new_client(&self, _client: &HttpClient) -> Result<(), String> {
        Ok(())
    }
    
    async fn post(&self, _client: &HttpClient, url: String, body: String) -> Result<String, String> {
        Ok(format!("POST {} bytes to {}", body.len(), url))
    }
}

fn main() {}
