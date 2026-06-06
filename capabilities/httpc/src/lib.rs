// Async capability - HTTP client with URL allowlist
use pyroduct::{Capture, CapturedError};

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
#[pyroduct::magma]
pub struct HttpClient;

/// The internal state of the HTTP Server capability.
pub struct HttpServer {
    client: reqwest::Client,
    allowed_endpoints: Vec<AllowedEndpoint>,
}

#[pyroduct::capability]
impl HttpServer {
    type Client = HttpClient;
    type Config = HttpConfig;
    
    /// Initializes a new HttpServer instance. 
    /// If no config is provided, defaults to a 30-second timeout and an empty allowlist.
    async fn new(config: Option<HttpConfig>) -> Result<Self> {
        let config = config.unwrap_or(HttpConfig {
            timeout_ms: 30000,
            allowed_endpoints: Vec::new(),
        });
        let timeout = std::time::Duration::from_millis(config.timeout_ms);
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .capture("failed to build reqwest client")?;
        Ok(Self {
            client,
            allowed_endpoints: config.allowed_endpoints,
        })
    }
    
    /// Resets the capability state.
    async fn reset(&mut self) -> Result<(), CapturedError> {
        Ok(())
    }
    
    /// Validates and prepares a new client instance.
    fn register(&self, _client: &HttpClient) -> Result<(), CapturedError> {
        Ok(())
    }
    
    /// Performs an asynchronous GET request if the URL and method are allowed.
    async fn get(&self, _client: &HttpClient, url: String) -> Result<String, CapturedError> {
        self.check_allowed(&url, "GET")?;
        let resp = self.client
            .get(&url)
            .send()
            .await
            .capture("HTTP GET request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            pyroduct::bail!("HTTP GET returned status {}: {}", status, text);
        }

        let body = resp
            .text()
            .await
            .capture("Failed to read HTTP response body")?;

        Ok(body)
    }
    
    /// Performs an asynchronous POST request with a body if the URL and method are allowed.
    async fn post(&self, _client: &HttpClient, url: String, body: String) -> Result<String, CapturedError> {
        self.check_allowed(&url, "POST")?;
        let resp = self.client
            .post(&url)
            .body(body)
            .send()
            .await
            .capture("HTTP POST request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            pyroduct::bail!("HTTP POST returned status {}: {}", status, text);
        }

        let resp_body = resp
            .text()
            .await
            .capture("Failed to read HTTP response body")?;

        Ok(resp_body)
    }
    
    /// Performs an asynchronous PUT request with a body if the URL and method are allowed.
    async fn put(&self, _client: &HttpClient, url: String, body: String) -> Result<String, CapturedError> {
        self.check_allowed(&url, "PUT")?;
        let resp = self.client
            .put(&url)
            .body(body)
            .send()
            .await
            .capture("HTTP PUT request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            pyroduct::bail!("HTTP PUT returned status {}: {}", status, text);
        }

        let resp_body = resp
            .text()
            .await
            .capture("Failed to read HTTP response body")?;

        Ok(resp_body)
    }
    
    /// Performs an asynchronous DELETE request if the URL and method are allowed.
    async fn delete(&self, _client: &HttpClient, url: String) -> Result<String, CapturedError> {
        self.check_allowed(&url, "DELETE")?;
        let resp = self.client
            .delete(&url)
            .send()
            .await
            .capture("HTTP DELETE request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            pyroduct::bail!("HTTP DELETE returned status {}: {}", status, text);
        }

        let resp_body = resp
            .text()
            .await
            .capture("Failed to read HTTP response body")?;

        Ok(resp_body)
    }
}

impl HttpServer {
    fn check_allowed(&self, url: &str, method: &str) -> Result<(), CapturedError> {
        for endpoint in &self.allowed_endpoints {
            if url_matches(url, &endpoint.url) {
                if endpoint.methods.iter().any(|m| m.eq_ignore_ascii_case(method)) {
                    return Ok(());
                } else {
                    pyroduct::bail!(
                        "Method {} not allowed for {}. Allowed methods: {:?}",
                        method, endpoint.url, endpoint.methods
                    );
                }
            }
        }
        
        pyroduct::bail!(
            "URL {} is not in the allowlist. Allowed endpoints: {:?}",
            url,
            self.allowed_endpoints.iter().map(|e| &e.url).collect::<Vec<_>>()
        )
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