use serde::{Deserialize, Serialize};

// 1. Configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    pub ports: Vec<String>,
}

// 2. Client State
#[pyroduct::capability_client]
#[derive(Clone)]
pub struct SerialHandle {
    pub id: u64,
}

// 3. Capability Definition
#[pyroduct::capability(SerialServer)]
pub trait SerialPool {
    type Client = SerialHandle;
    
    // Constructor - returns the Client type directly
    fn open(port: String, baud: u32) -> SerialHandle {
        SerialHandle { id: 0 }
    }
    
    // Regular method - returns Result, can include Client in Ok variant
    fn close() -> Result<(), String>;
}

// 4. Server State
#[pyroduct::capability_server(config = SerialConfig)]
pub struct SerialServer {
    next_id: u64,
}

// 5. Lifecycle Implementation
impl state::SerialServerInit for state::SerialServer {
    fn new(config: &SerialConfig) -> Self { 
        Self { next_id: 0 } 
    }
    fn reset(&mut self) { 
        self.next_id = 0; 
    }
}

// 6. Trait Implementation  
impl methods::SerialPool for state::SerialServer {
    fn new_client(&self, _client: &SerialHandle) -> () {}
    
    fn close(&self, _client: &SerialHandle) -> Result<(), String> {
        Ok(())
    }
}

fn main() {
    // Just verifying compilation
}