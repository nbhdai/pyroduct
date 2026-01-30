#[pyroduct::capability_client]
pub struct SimpleClient;

#[pyroduct::capability(StatefulServer)]
pub trait Simple {
    type Client = SimpleClient;

    fn client() -> SimpleClient {
        SimpleClient
    }
    fn call() -> f32;
}


#[pyroduct::capability_server]
pub struct StatefulServer;

#[pyroduct::capability_impl]
impl Simple for StatefulServer {
    fn new_client(&self, client: &SimpleClient) {  }
    fn call(&self, client: &SimpleClient)  -> f32 { 42.0 }
}

impl StatefulServerInit for StatefulServer {
    fn new(config: &()) -> Self { Self }
    fn default() -> Self { Self }
    fn reset(&mut self) {}
}

fn main() {
    // Just verifying compilation
}