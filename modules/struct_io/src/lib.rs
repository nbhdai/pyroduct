use httpc::{HttpClient, HttpClientMethods};
use pyroduct::{magma, module};

#[magma]
struct HttpRequest {
    method: String,
    url: String,
    body: String,
}

#[magma]
struct HttpResponse {
    status: String,
    body: String,
}

#[module(output = response)]
fn http_call(request: HttpRequestRef<'_>) -> Result<HttpResponse, String> {
    let http = HttpClient.register()?;

    let body = match request.method {
        "GET" => http.get(request.url.to_string())?,
        "POST" => http.post(request.url.to_string(), request.body.to_string())?,
        "PUT" => http.put(request.url.to_string(), request.body.to_string())?,
        "DELETE" => http.delete(request.url.to_string())?,
        _ => pyroduct::bail!("Unsupported method: {}", request.method),
    };

    Ok(HttpResponse {
        status: "ok".to_string(),
        body,
    })
}
