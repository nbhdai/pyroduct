#[pyroduct::client]
pub struct SimpleClient;

#[pyroduct::capability(StatefulServer)]
pub trait Simple {
    type Client = SimpleClient;

    fn client() -> SimpleClient {
        SimpleClient
    }
    fn call() -> f32;
}


#[pyroduct::server(methods = Simple)]
pub struct StatefulServer;

#[pyroduct::capability_impl]
impl Simple for StatefulServer {
    fn new_client(&self, _client: &SimpleClient) {  }
    fn call(&self, _client: &SimpleClient)  -> f32 { 42.0 }
}

#[pyroduct::server_impl]
impl StatefulServerInit for StatefulServer {
    fn new() -> Self { Self }
    fn reset(&mut self) {}
}

fn main() {}