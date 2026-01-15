//! HTTP Client Capability
//! 
//! Pattern: Client state only (client holds config, server is stateless per-request)

use capability_derive::*;

// ============================================================================
// SHARED TYPES
// ============================================================================

/// HTTP response structure - shared between client and server
#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Client-side state that gets serialized and sent to host.
/// 
/// `#[capability_client]` generates:
/// - rkyv derives for serialization
/// - Helper methods for the client-side API
#[capability_client]
#[derive(Debug, Clone)]
pub struct HttpClient {
    pub base_url: String,
    pub timeout_secs: Option<u64>,
}

// ============================================================================
// CLIENT-SIDE API
// ============================================================================

impl HttpClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            timeout_secs: None,
        }
    }
    
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }
}

// ============================================================================
// CAPABILITY DEFINITION
// ============================================================================

/// HTTP capability trait.
/// 
/// Functions that take `client: &HttpClient` as first parameter indicate
/// that client state should be serialized and passed to the host.
/// Stateless means the "init", "reset", and "drop" methods aren't generated.
#[capability(stateless)]
pub trait Http {
    type Config = ();
    
    /// GET request
    /// The `#[client_state]` attribute (or convention of first &ClientType param)
    /// tells the macro this needs client state serialization
    async fn get(#[client_state] client: &HttpClient, path: &str) -> Result<HttpResponse, String>;
    
    /// POST request with optional body
    async fn post(
        #[client_state] client: &HttpClient, 
        path: &str, 
        body: Option<String>
    ) -> Result<HttpResponse, String>;
    
    /// HEAD request
    async fn head(#[client_state] client: &HttpClient, path: &str) -> Result<HttpResponse, String>;
}

// ============================================================================
// CLIENT-SIDE METHODS (convenience wrappers)
// ============================================================================

/// These methods provide a nicer API on the client struct.
/// They delegate to the generated client functions.
impl HttpClient {
    pub fn get(&self, path: &str) -> Result<HttpResponse, String> {
        // Calls the generated client::get(self, path)
        http_client::get(self, path)
    }
    
    pub fn post(&self, path: &str, body: Option<&str>) -> Result<HttpResponse, String> {
        http_client::post(self, path, body.map(|s| s.to_string()))
    }
    
    pub fn head(&self, path: &str) -> Result<HttpResponse, String> {
        http_client::head(self, path)
    }
}

// ============================================================================
// SERVER IMPLEMENTATION
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod server {
    use super::*;

    /// Server doesn't need persistent state - each request creates a fresh client.
    /// But we still need a struct for the capability_server macro.
    #[capability_server(service = Http, stateless)]
    pub struct HttpServer;

    impl Http for HttpServer {
        async fn get(client: &HttpClient, path: &str) -> Result<HttpResponse, String> {
            let http = Self::build_client(client)?;
            let url = Self::build_url(client, path);
            
            let resp = http.get(&url)
                .send()
                .await
                .map_err(|e| format!("GET failed: {}", e))?;
            
            Self::convert_response(resp).await
        }
        
        async fn post(client: &HttpClient, path: &str, body: Option<String>) -> Result<HttpResponse, String> {
            let http = Self::build_client(client)?;
            let url = Self::build_url(client, path);
            
            let mut req = http.post(&url);
            if let Some(b) = body {
                req = req.body(b);
            }
            
            let resp = req.send()
                .await
                .map_err(|e| format!("POST failed: {}", e))?;
            
            Self::convert_response(resp).await
        }
        
        async fn head(client: &HttpClient, path: &str) -> Result<HttpResponse, String> {
            let http = Self::build_client(client)?;
            let url = Self::build_url(client, path);
            
            let resp = http.head(&url)
                .send()
                .await
                .map_err(|e| format!("HEAD failed: {}", e))?;
            
            Self::convert_response(resp).await
        }
    }
    
    /// Private helper methods for the server
    impl HttpServer {
        fn build_client(config: &HttpClient) -> Result<reqwest::Client, String> {
            let mut builder = reqwest::Client::builder();
            if let Some(timeout) = config.timeout_secs {
                builder = builder.timeout(std::time::Duration::from_secs(timeout));
            }
            builder.build().map_err(|e| format!("Failed to build client: {}", e))
        }
        
        fn build_url(config: &HttpClient, path: &str) -> String {
            format!("{}{}", config.base_url.trim_end_matches('/'), path)
        }
        
        async fn convert_response(resp: reqwest::Response) -> Result<HttpResponse, String> {
            let status = resp.status().as_u16();
            let headers: Vec<(String, String)> = resp.headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let body = resp.bytes()
                .await
                .map_err(|e| format!("Failed to read body: {}", e))?
                .to_vec();
            
            Ok(HttpResponse { status, headers, body })
        }
    }

    capability_export!(env = "basic_http", HttpServer);
}