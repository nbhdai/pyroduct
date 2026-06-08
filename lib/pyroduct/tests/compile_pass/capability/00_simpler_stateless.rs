#[pyroduct::magma]
pub struct SimpleClient;

pub struct StatefulServer;

#[pyroduct::capability]
impl StatefulServer {
    type Client = SimpleClient;
    fn new() -> Result<Self> {
        Ok(Self)
    }
    fn register(&self, _client: &SimpleClient) -> Result<()> {
        Ok(())
    }
    fn reset(&mut self) -> Result<()> {
        Ok(())
    }
    fn call(&self, _client: &SimpleClient) -> Result<f32> {
        Ok(42.0)
    }
}

fn main() {}
