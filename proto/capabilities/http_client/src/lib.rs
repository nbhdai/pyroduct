// proto/http_client/src/lib.rs
//
// PATTERN 3: Client state only - stateless host functions, client holds config
//
// Host: No state, creates new reqwest::Client each request (wasteful but simple)
// Client: HttpClient struct holds base URL, headers, timeout config
// ============================================================================

// ============================================================================
// SHARED: Client state and types
// ============================================================================

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub struct HttpClient {
    pub base_url: String,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

// ============================================================================
// DEVELOPER WRITES: Host functions (receive client state, no host state)
// ============================================================================

#[cfg(feature = "capability")]
mod host {
    use super::*;

    // Helper: build a fresh reqwest client from client config (no headers, so it's empty)
    fn build_client(_: &HttpClient) -> Result<reqwest::Client, String> {
        let builder = reqwest::Client::builder();

        builder
            .build()
            .map_err(|e| format!("Failed to build client: {}", e))
    }

    // Helper: build full URL
    fn build_url(client: &HttpClient, url: &str) -> String {
        format!("{}{}", client.base_url.trim_end_matches('/'), url)
    }

    // Helper: convert reqwest response to our HttpResponse
    async fn convert_response(resp: reqwest::Response) -> Result<HttpResponse, String> {
        let status = resp.status().as_u16();
        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp
            .bytes()
            .await
            .map_err(|e| format!("Failed to read body: {}", e))?;

        Ok(HttpResponse {
            status,
            headers,
            body: body.to_vec(),
        })
    }

    pub async fn get(client: &HttpClient, url: &str) -> Result<HttpResponse, String> {
        let http_client = build_client(client)?;
        let url = build_url(client, &url);

        let req_builder = http_client.get(&url);

        let resp = req_builder
            .send()
            .await
            .map_err(|e| format!("GET failed: {}", e))?;
        convert_response(resp).await
    }

    pub async fn post(
        client: &HttpClient,
        url: &str,
        body: Option<&str>,
    ) -> Result<HttpResponse, String> {
        let http_client = build_client(client)?;
        let url = build_url(client, url);

        let mut req_builder = http_client.post(&url);

        if let Some(body) = body {
            req_builder = req_builder.body::<String>(body.into());
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| format!("POST failed: {}", e))?;
        convert_response(resp).await
    }
}

// ============================================================================
// DEVELOPER WRITES: Client interface (methods on HttpClient)
// ============================================================================

#[cfg(feature = "module")]
impl HttpClient {
    pub fn new(base_url: &str) -> Self {
        HttpClient {
            base_url: base_url.to_string(),
        }
    }

    pub fn get(&self, url: &str) -> Result<HttpResponse, String> {
        http_client::call_get(self, url)
    }

    pub fn post(&self, url: &str, body: Option<&str>) -> Result<HttpResponse, String> {
        http_client::call_post(self, url, body)
    }
}

// ============================================================================
// GENERATED/BOILERPLATE: FFI layer (capability side)
// ============================================================================

#[cfg(feature = "capability")]
pub mod __http_ffi {
    #[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
    pub struct __HttpClientPost {
        pub url: String,
        pub body: Option<String>,
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn host_http_get<'a>(
        client_state_ptr: *const u8,
        client_state_len: usize,
        input_ptr: *const u8,
        input_len: usize,
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
        ::pyroduct::capability::safe_async::async_ci_call::<
            crate::HttpClient,
            String,
            Result<crate::HttpResponse, String>,
            _,
            _,
        >(
            client_state_ptr,
            client_state_len,
            input_ptr,
            input_len,
            host_state_ptr,
            |client, input| async move { crate::host::get(&client, &input).await },
        )
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn host_http_post<'a>(
        client_state_ptr: *const u8,
        client_state_len: usize,
        input_ptr: *const u8,
        input_len: usize,
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
        #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
        struct __InputHttpPost {
            url: String,
            body: Option<String>,
        }

        ::pyroduct::capability::safe_async::async_ci_call::<
            crate::HttpClient,
            __InputHttpPost,
            Result<crate::HttpResponse, String>,
            _,
            _,
        >(
            client_state_ptr,
            client_state_len,
            input_ptr,
            input_len,
            host_state_ptr,
            |client, input| async move {
                crate::host::post(&client, &input.url, input.body.as_ref().map(|s| s.as_str()))
                    .await
            },
        )
    }

    // --- Lifecycle (no host state) ---

    // --- Manifest ---

    #[unsafe(no_mangle)]
    pub extern "C" fn plugin_manifest<'a>(
        id: u64,
        log_callback: ::pyroduct::capability_host::ffi::LogCallback,
    ) -> ::pyroduct::capability_host::ffi::PluginExports<'a> {
        ::pyroduct::capability::init_logging(id, log_callback);
        const CLASS: &'static str = "env";
        const FUNC_1: &'static str = "host_http_get";
        const FUNC_2: &'static str = "host_http_post";
        const EXPORTS: [::pyroduct::capability_host::ffi::PluginExport;2] = [
            ::pyroduct::capability_host::ffi::PluginExport {
                module: CLASS.as_ptr(),
                module_len: CLASS.len(),
                name: FUNC_1.as_ptr(),
                name_len: FUNC_1.len(),
                func: ::pyroduct::capability_host::ffi::PluginFunction::Async(host_http_get),
            },
            ::pyroduct::capability_host::ffi::PluginExport {
                module: "env".as_ptr(),
                module_len: 3,
                name: FUNC_2.as_ptr(),
                name_len: FUNC_2.len(),
                func: ::pyroduct::capability_host::ffi::PluginFunction::Async(host_http_post),
            },
        ];

        let result = ::pyroduct::capability_host::ffi::PluginExports {
            len: EXPORTS.len(),
            ptr: EXPORTS.as_ptr(),
            reset: ::pyroduct::capability_host::ffi::PluginResetFn::Null,
            init: ::pyroduct::capability_host::ffi::PluginInitFn::Null,
            drop: ::pyroduct::capability_host::ffi::PluginDropFn::Null,
        };
        std::mem::forget(exports);
        result
    }
}

// ============================================================================
// GENERATED/BOILERPLATE: FFI layer (client/WASM side)
// ============================================================================

#[cfg(feature = "module")]
mod http_client {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        fn host_http_get(
            cs_ptr: *const u8,
            cs_len: usize,
            in_ptr: *const u8,
            in_len: usize,
        ) -> *const u8;
        fn host_http_post(
            cs_ptr: *const u8,
            cs_len: usize,
            in_ptr: *const u8,
            in_len: usize,
        ) -> *const u8;
    }

    pub fn call_get(client: &crate::HttpClient, url: &str) -> Result<crate::HttpResponse, String> {
        ::pyroduct::module_capability::access::call_from_wasm::<
            crate::HttpClient,
            String,
            Result<crate::HttpResponse, String>,
            _,
        >(
            "http_client",
            Some(client),
            Some(&url.to_string()),
            |client_state_ptr: *const u8,
             client_state_len: usize,
             input_ptr: *const u8,
             input_len: usize| {
                unsafe { host_http_get(client_state_ptr, client_state_len, input_ptr, input_len) }
            },
        )
    }

    pub fn call_post(
        client: &crate::HttpClient,
        url: &str,
        body: Option<&str>,
    ) -> Result<crate::HttpResponse, String> {
        let request = crate::__http_ffi::__HttpClientPost {
            url: url.to_owned(),
            body: body.map(|b| b.to_owned()),
        };
        ::pyroduct::module_capability::access::call_from_wasm::<
            crate::HttpClient,
            crate::__http_ffi::__HttpClientPost,
            Result<crate::HttpResponse, String>,
            _,
        >(
            "http_client",
            Some(client),
            Some(&request),
            |client_state_ptr: *const u8,
             client_state_len: usize,
             input_ptr: *const u8,
             input_len: usize| {
                unsafe { host_http_post(client_state_ptr, client_state_len, input_ptr, input_len) }
            },
        )
    }
}
