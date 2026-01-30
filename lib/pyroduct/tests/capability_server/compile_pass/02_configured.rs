use pyroduct::capability;
use serde::Deserialize;

#[capability]
pub trait Configured {
    type Client = MyClient;
    fn new_client(&self);
    fn call(&self, client: &MyClient) -> u32;
}

#[derive(Deserialize)]
pub struct MyConfig {
    limit: u32,
}

#[capability_server(service = Configured, config = MyConfig)]
pub struct ConfiguredServer {
    limit: u32,
}

impl ConfiguredServerInit for ConfiguredServer {
    fn new() -> Self { 
        Self { limit: 10 } 
    }
    
    fn with_config(config: MyConfig) -> Self {
        Self { limit: config.limit }
    }

    fn reset(&mut self) {}
}

fn main() {
    // Just verifying compilation
}