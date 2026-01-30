use pyroduct::{capability, capability_client, capability_server};
use serde::{Deserialize, Serialize};

// 1. Configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    pub ports: Vec<String>,
}

// 2. Client State
#[capability_client]
#[derive(Clone)]
pub struct SerialHandle {
    pub id: u64,
}

// 3. Capability Definition
#[capability(SerialServer)]
pub trait SerialPool {
    type Client = SerialHandle;
    async fn open(port: String, baud: u32) -> Result<SerialHandle, String>;
    fn close(handle: &SerialHandle) -> Result<(), String>;
}

// 4. Server State
#[capability_server(service = SerialPool, config = SerialConfig)]
pub struct SerialServer {
    next_id: u64,
}

// 5. Lifecycle Implementation
impl SerialServerInit for SerialServer {
    fn new() -> Self { Self { next_id: 0 } }
    fn with_config(_config: SerialConfig) -> Self { Self { next_id: 0 } }
    fn reset(&mut self) { self.next_id = 0; }
}

impl SerialPool for SerialServer {
    async fn open(port: String, baud: u32) -> Result<SerialHandle, String> {
        Ok(SerialHandle { id: 1 })
    }

    fn close(handle: &SerialHandle) -> Result<(), String> {
        Ok(())
    }
}

// 7. Export (generates plugin_manifest)
// Note: We use the manual export macro here because we want to wire up the 
// specific server implementation.
capability_export!(env = "serial", SerialServer);

fn main() {
    // Just verifying compilation
}