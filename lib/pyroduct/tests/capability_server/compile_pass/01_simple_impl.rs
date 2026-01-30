#[pyroduct::client]
pub struct GreeterClient {}

pub struct Greeter;

#[pyroduct::config]
pub struct GreeterConfig {}


#[pyroduct::capability]
impl Greeter {
    type Config = GreeterConfig;
    type Client = GreeterClient;
    fn new(_config: Option<GreeterConfig>) -> Self {
        Greeter
    }
    
    fn reset(&mut self) {}

    fn new_client(&self, _client: &GreeterClient) -> () {}
    
    fn greet(&self, _client: &GreeterClient, name: String) -> String {
        format!("Hello, {}", name)
    }
}


fn main() {}