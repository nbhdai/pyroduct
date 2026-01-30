#[pyroduct::capability(GreeterServer)]
trait Greeter {
    type Client = GreeterClient;
    
    fn new() -> GreeterClient {
        GreeterClient {}
    }
    
    fn greet(name: String) -> String;
}

#[pyroduct::capability_client]
pub struct GreeterClient {}

#[derive(serde::Deserialize)]
pub struct GreeterConfig {
    data: u32
}

#[pyroduct::capability_server(methods = Greeter, config = GreeterConfig)]
pub struct GreeterServer;


#[pyroduct::capability_impl]
impl Greeter for GreeterServer {
    fn new_client(&self, _client: &GreeterClient) -> () {
        // Initialize client state on server if needed
    }
    
    fn greet(&self, _client: &GreeterClient, name: String) -> String {
        format!("Hello, {}", name)
    }
}

impl GreeterServerInit for GreeterServer {
    fn new(_config: &GreeterConfig) -> Self {
        GreeterServer
    }

    fn default() -> Self {
        GreeterServer
    }
    
    fn reset(&mut self) {
        // Reset server state if needed
    }
}

fn main() {
    // Just verifying compilation
}