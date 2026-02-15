// Async capability - HTTP client with URL allowlist

/// Defines a specific URL pattern and the HTTP methods permitted for that pattern.
#[pyroduct::config]
pub struct AllowedEndpoint {
    /// The URL or prefix pattern (e.g., "https://api.example.com/*").
    pub url: String,
    /// The HTTP verbs allowed for this endpoint (e.g., "GET", "POST").
    pub methods: Vec<String>,
}

/// Configuration for the HTTP Server capability, including timeouts and security constraints.
#[pyroduct::config]
pub struct HttpConfig {
    /// Maximum time in milliseconds to wait for a request to complete.
    pub timeout_ms: u64,
    /// A list of endpoints that the client is permitted to access.
    pub allowed_endpoints: Vec<AllowedEndpoint>,
}

/// The Pyroduct interface item representing the HTTP Client.
#[pyroduct::interface_item]
pub struct HttpClient;

/// The internal state of the HTTP Server capability.
pub struct HttpServer {
    _timeout: std::time::Duration,
    allowed_endpoints: Vec<AllowedEndpoint>,
}

#[pyroduct::capability]
impl HttpServer {
    type Client = HttpClient;
    type Config = HttpConfig;
    type Error = String;
    
    /// Initializes a new HttpServer instance. 
    /// If no config is provided, defaults to a 30-second timeout and an empty allowlist.
    async fn new(config: Option<HttpConfig>) -> Self {
        let config = config.unwrap_or(HttpConfig {
            timeout_ms: 30000,
            allowed_endpoints: Vec::new(),
        });
        Self {
            _timeout: std::time::Duration::from_millis(config.timeout_ms),
            allowed_endpoints: config.allowed_endpoints,
        }
    }
    
    /// Resets the capability state.
    async fn reset(&mut self) {}
    
    /// Validates and prepares a new client instance.
    fn register(&self, _client: &HttpClient) -> Result<(), String> {
        Ok(())
    }
    
    /// Performs an asynchronous GET request if the URL and method are allowed.
    async fn get(&self, _client: &HttpClient, url: String) -> Result<String, String> {
        self.check_allowed(&url, "GET")?;
        // Simulated async HTTP request
        Ok(format!("GET response from {}", url))
    }
    
    /// Performs an asynchronous POST request with a body if the URL and method are allowed.
    async fn post(&self, _client: &HttpClient, url: String, body: String) -> Result<String, String> {
        self.check_allowed(&url, "POST")?;
        Ok(format!("POST {} bytes to {}", body.len(), url))
    }
    
    /// Performs an asynchronous PUT request with a body if the URL and method are allowed.
    async fn put(&self, _client: &HttpClient, url: String, body: String) -> Result<String, String> {
        self.check_allowed(&url, "PUT")?;
        Ok(format!("PUT {} bytes to {}", body.len(), url))
    }
    
    /// Performs an asynchronous DELETE request if the URL and method are allowed.
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
    if url == pattern {
        return true;
    }

    if let Some(prefix) = pattern.strip_suffix("/*") {
        if url.starts_with(prefix) {
            return true;
        }
    }

    if url.starts_with(pattern) && url[pattern.len()..].starts_with('/') {
        return true;
    }
    
    false
}