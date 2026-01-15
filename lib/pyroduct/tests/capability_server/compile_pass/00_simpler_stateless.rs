use pyroduct::{capability, capability_server};

#[capability]
pub trait Simple {
    fn call(&mut self);
}

#[capability_server(service = Simple, stateless)]
pub struct StatelessServer;

#[capability_server(service = Simple)]
pub struct StatefulServer;

impl StatefulServerInit for StatefulServer {
    fn new() -> Self { Self }
    fn reset(&mut self) {}
}

fn main() {
    // Just verifying compilation
}