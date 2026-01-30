#[pyroduct::client]
pub struct SimpleClient;


pub struct StatefulServer;

#[pyroduct::capability]
impl StatefulServer {
    type Client = SimpleClient;
    fn new() -> Self { Self }
    fn new_client(&self, _client: &SimpleClient) {  }
    fn reset(&mut self) {}
    fn call(&self, _client: &SimpleClient)  -> f32 { 42.0 }
}


fn main() {}