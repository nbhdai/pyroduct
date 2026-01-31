// Module 4: HTTP API caller
use pyroduct::{module, FromRow, DeepRef, ToRow};

#[derive(FromRow, DeepRef)]
struct HttpRequest {
    method: String,
    url: String,
    body: String,
}

#[derive(ToRow)]
struct HttpResponse {
    status: String,
    body: String,
}

#[module(output = response)]
fn http_call(request: &HttpRequestRef<'_>) -> Result<HttpResponse, String> {
    let http = HttpClient.register()?;
    
    let body = match input.method {
        "GET" => http.get(input.url.to_string())?,
        "POST" => http.post(input.url.to_string(), input.body.to_string())?,
        "PUT" => http.put(input.url.to_string(), input.body.to_string())?,
        "DELETE" => http.delete(input.url.to_string())?,
        _ => return Err(format!("Unsupported method: {}", input.method)),
    };
    
    Ok(HttpResponse {
        status: "ok".to_string(),
        body,
    })
}