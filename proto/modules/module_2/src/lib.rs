use proto_cpu_info::get_cpu_count;
use proto_http_client::HttpClient;
use tracing::info;

#[unsafe(no_mangle)]
pub extern "C" fn exter_call(input_ptr: *mut u8, input_len: usize) -> u64 {
    #[derive(::pyroduct::FromRow, ::pyroduct::DeepRef)]
    struct __Input {
        input: String,
    }

    #[derive(::pyroduct::ToRow)]
    struct __Output {
        cpu_count: u32,
    }

    let call = |input: &__InputRef| call(&input.input).map(|result| __Output { cpu_count: result });

    ::pyroduct::module::call::<__Input, __Output, _>(input_ptr, input_len, call)
}

/// This should be represented in the macro by
/// pub fn call(input: &str) -> Result<{ cpu_count: u32 }, String> { ... }
fn call(input: &str) -> Result<u32, String> {
    info!("Processing request for input: '{}'", input);

    // 1. Call CPU Capability
    info!("Requesting CPU count...");
    let cpu_info = get_cpu_count();
    info!("Got CPU info: {}", cpu_info);

    // 2. Call HTTP Capability
    // Use the input as the URL, or default to a safe test URL if input is not a URL
    let url = if input.starts_with("http") {
        input
    } else {
        return Err(format!("Tried to call Http client with {input}"));
    };
    let client = HttpClient::new(url);

    info!("Requesting HTTP GET for: {}", url);
    let http_response = client.get("/").expect("Can get the root");
    let body = String::from_utf8_lossy(&http_response.body);
    // Truncate response for readability in logs
    info!(
        "Got HTTP response (len: {}): {}...",
        http_response.body.len(),
        &body[..60],
    );

    Ok(cpu_info)
}
