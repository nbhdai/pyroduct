#[pyroduct::client]
pub struct GreeterClient {}

#[pyroduct::config]
pub struct GreeterConfig {
    data: u32
}

#[pyroduct::capability(GreeterServer)]
trait Greeter {
    type Client = GreeterClient;
    
    fn new() -> GreeterClient {
        GreeterClient {}
    }
    
    fn greet(name: String) -> String;
}

#[pyroduct::server(methods = Greeter, config = GreeterConfig)]
pub struct GreeterServer;


#[pyroduct::capability_impl]
impl Greeter for GreeterServer {
    fn new_client(&self, _client: &GreeterClient) -> () {}
    
    fn greet(&self, _client: &GreeterClient, name: String) -> String {
        format!("Hello, {}", name)
    }
}

#[pyroduct::server_impl]
impl GreeterServerInit for GreeterServer {
    fn new(_config: Option<GreeterConfig>) -> Self {
        GreeterServer
    }
    
    fn reset(&mut self) {}
}

fn main() {}