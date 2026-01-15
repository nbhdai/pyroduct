use pyroduct::{capability, capability_server};
use serde::Deserialize;

#[capability]
pub trait Configured {
    fn call(&mut self);
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