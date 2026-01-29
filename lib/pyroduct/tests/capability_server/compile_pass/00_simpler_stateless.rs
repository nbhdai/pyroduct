use pyroduct::{capability};

capability! {
    #[capability_client]
    pub struct SimpleClient;

    #[capability]
    pub trait Simple {
        type Client = SimpleClient;
        fn call(&mut self);
    }

    #[capability_server(service = Simple)]
    pub struct StatefulServer;

    impl Simple for StatefulServer {
        fn new() -> Self { Self }
        fn reset(&mut self) {}
    }
}

fn main() {
    // Just verifying compilation
}