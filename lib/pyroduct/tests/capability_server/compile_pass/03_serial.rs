#[pyroduct::config]
pub struct SerialConfig {
    pub ports: Vec<String>,
}

#[pyroduct::client]
#[derive(Clone)]
pub struct SerialHandle {
    pub id: u64,
}

#[pyroduct::capability(SerialServer)]
pub trait SerialPool {
    type Client = SerialHandle;
    
    fn open(_port: String, _baud: u32) -> SerialHandle {
        SerialHandle { id: 0 }
    }
    fn close() -> Result<(), String>;
}

// 4. Server State
#[pyroduct::server(methods = SerialPool, config = SerialConfig)]
pub struct SerialServer {
    next_id: u64,
}

#[pyroduct::server_impl]
impl SerialServerInit for SerialServer {
    fn new(_config: Option<SerialConfig>) -> Self { 
        Self { next_id: 0 } 
    }
    fn reset(&mut self) { 
        self.next_id = 0; 
    }
}

#[pyroduct::capability_impl]
impl SerialPool for SerialServer {
    fn new_client(&self, _client: &SerialHandle) -> () {}
    
    fn close(&self, _client: &SerialHandle) -> Result<(), String> {
        Ok(())
    }
}

fn main() {}