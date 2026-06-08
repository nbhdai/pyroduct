// Async capability - HTTP client with URL allowlist
#[pyroduct::config]
pub struct HttpConfig {
    pub timeout_ms: u64,
}

#[pyroduct::magma]
pub struct HttpClient;

pub struct HttpServer {
    timeout: std::time::Duration,
}

#[pyroduct::capability]
impl HttpServer {
    type Client = HttpClient;
    type Config = HttpConfig;

    async fn new(config: Option<HttpConfig>) -> Result<Self> {
        let config = config.unwrap_or(HttpConfig { timeout_ms: 30000 });
        Ok(Self {
            timeout: std::time::Duration::from_millis(config.timeout_ms),
        })
    }

    async fn reset(&mut self) -> Result<()> {
        Ok(())
    }

    fn register(&self, _client: &HttpClient) -> Result<(), pyroduct::CapturedError> {
        Ok(())
    }

    async fn post(
        &self,
        _client: &HttpClient,
        url: String,
        body: String,
        _len: u64,
    ) -> Result<String, pyroduct::CapturedError> {
        Ok(format!("POST {} bytes to {}", body.len(), url))
    }
}

fn main() {}
