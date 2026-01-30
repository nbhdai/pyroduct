#[pyroduct::client]
pub struct CpuClient;

pub struct CpuServer;

#[pyroduct::capability]
impl CpuServer {
    type Client = CpuClient;
    
    fn new() -> Self { Self }
    fn reset(&mut self) {}
    fn new_client(&self, _client: &CpuClient) {}
    
    fn get_cpu_count(&self, _client: &CpuClient) -> u32 {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1)
    }
}