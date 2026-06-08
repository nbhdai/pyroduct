#[pyroduct::config]
pub struct SerialConfig {
    pub ports: Vec<String>,
}

#[pyroduct::magma]
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
    fn new(_config: Option<SerialConfig>) -> Result<Self> {
        Ok(Self { ids: Vec::new() })
    }
    fn reset(&mut self) -> Result<()> {
        self.ids.clear();
        Ok(())
    }

    fn register(&self, _client: &SerialHandle) -> Result<(), pyroduct::CapturedError> {
        Ok(())
    }

    fn close(&self, _client: &SerialHandle) -> Result<(), pyroduct::CapturedError> {
        Ok(())
    }
}

fn main() {}
