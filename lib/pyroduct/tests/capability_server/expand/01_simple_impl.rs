#[pyroduct::client]
pub struct GreeterClient {}


#[pyroduct::capability]
impl Greeter for Greeter {
    type Config = GreeterConfig;
    type Client = GreeterClient;
    fn new(config: Option<GreeterConfig>) -> Self {
        Greeter
    }
    
    fn reset(&mut self) {}

    fn new_client(&self, _client: &GreeterClient) -> () {}
    
    fn greet(&self, _client: &GreeterClient, name: String) -> String {
        format!("Hello, {}", name)
    }
}


fn main() {}