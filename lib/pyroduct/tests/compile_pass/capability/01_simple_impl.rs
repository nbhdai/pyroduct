#[pyroduct::magma]
pub struct GreeterClient {}

pub struct Greeter;

#[pyroduct::config]
pub struct GreeterConfig {}

#[pyroduct::capability]
impl Greeter {
    type Config = GreeterConfig;
    type Client = GreeterClient;
    fn new(_config: Option<GreeterConfig>) -> Result<Self> {
        Ok(Greeter)
    }

    fn reset(&mut self) -> Result<()> {
        Ok(())
    }

    fn register(&self, _client: &GreeterClient) -> Result<()> {
        Ok(())
    }

    fn greet(&self, _client: &GreeterClient, name: String) -> Result<String> {
        Ok(format!("Hello, {}", name))
    }
}

fn main() {}
