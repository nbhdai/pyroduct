// Async capability - HTTP client with URL allowlist
#[pyroduct::config]
pub struct HttpConfig {
    pub timeout_ms: u64,
    pub allowed_endpoints: Vec<AllowedEndpoint>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AllowedEndpoint {
    pub url: String,
    pub methods: Vec<String>, // "GET", "POST", etc.
}

#[pyroduct::client]
pub struct HttpClient;

pub struct HttpServer {
    timeout: std::time::Duration,
    allowed_endpoints: Vec<AllowedEndpoint>,
}

#[pyroduct::capability]
impl HttpServer {
    type Client = HttpClient;
    type Config = HttpConfig;
    type Error = String;
    
    async fn new(config: Option<HttpConfig>) -> Self {
        let config = config.unwrap_or(HttpConfig {
            timeout_ms: 30000,
            allowed_endpoints: Vec::new(),
        });
        Self {
            timeout: std::time::Duration::from_millis(config.timeout_ms),
            allowed_endpoints: config.allowed_endpoints,
        }
    }
    
    async fn reset(&mut self) {}
    
    fn new_client(&self, _client: &HttpClient) -> Result<(), String> {
        Ok(())
    }
    
    async fn get(&self, _client: &HttpClient, url: String) -> Result<String, String> {
        self.check_allowed(&url, "GET")?;
        // Simulated async HTTP request
        Ok(format!("GET response from {}", url))
    }
    
    async fn post(&self, _client: &HttpClient, url: String, body: String) -> Result<String, String> {
        self.check_allowed(&url, "POST")?;
        Ok(format!("POST {} bytes to {}", body.len(), url))
    }
    
    async fn put(&self, _client: &HttpClient, url: String, body: String) -> Result<String, String> {
        self.check_allowed(&url, "PUT")?;
        Ok(format!("PUT {} bytes to {}", body.len(), url))
    }
    
    async fn delete(&self, _client: &HttpClient, url: String) -> Result<String, String> {
        self.check_allowed(&url, "DELETE")?;
        Ok(format!("DELETE {}", url))
    }
}

impl HttpServer {
    fn check_allowed(&self, url: &str, method: &str) -> Result<(), String> {
        for endpoint in &self.allowed_endpoints {
            if url_matches(url, &endpoint.url) {
                if endpoint.methods.iter().any(|m| m.eq_ignore_ascii_case(method)) {
                    return Ok(());
                } else {
                    return Err(format!(
                        "Method {} not allowed for {}. Allowed methods: {:?}",
                        method, endpoint.url, endpoint.methods
                    ));
                }
            }
        }
        
        Err(format!(
            "URL {} is not in the allowlist. Allowed endpoints: {:?}",
            url,
            self.allowed_endpoints.iter().map(|e| &e.url).collect::<Vec<_>>()
        ))
    }
}

fn url_matches(url: &str, pattern: &str) -> bool {
    // Exact match
    if url == pattern {
        return true;
    }
    
    // Prefix match with wildcard (e.g., "https://api.example.com/*")
    if let Some(prefix) = pattern.strip_suffix("/*") {
        if url.starts_with(prefix) {
            return true;
        }
    }
    
    // Prefix match (e.g., "https://api.example.com/v1/users" matches "https://api.example.com/v1/users/123")
    if url.starts_with(pattern) && url[pattern.len()..].starts_with('/') {
        return true;
    }
    
    false
}