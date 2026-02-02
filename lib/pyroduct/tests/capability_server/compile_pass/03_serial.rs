#[pyroduct::config]
pub struct SerialConfig {
    pub ports: Vec<String>,
}

#[pyroduct::interface_item]
#[derive(Clone)]
pub struct SerialHandle {
    pub id: u64,
}

struct SerialServer {
    ids: Vec<u64>,
}

#[pyroduct::capability]
impl SerialServer {
    type Client = SerialHandle;
    type Config = SerialConfig;
    type Error = String;
    fn new(_config: Option<SerialConfig>) -> Self { 
        Self { ids: Vec::new() } 
    }
    fn reset(&mut self) { 
        self.ids.clear(); 
    }

    fn new_client(&self, _client: &SerialHandle) -> Result<(), String>  {
        Ok(())
    }
    
    fn close(&self, _client: &SerialHandle) -> Result<(), String> {
        Ok(())
    }
}

fn main() {}