#[pyroduct::magma]
pub struct CpuClient;

pub struct CpuServer;

#[pyroduct::capability]
/// Access some CPU info for the host computer
impl CpuServer {
    type Client = CpuClient;

    fn new() -> Result<Self> {
        Ok(Self)
    }
    fn reset(&mut self) -> Result<(), CapturedError> {
        Ok(())
    }
    fn register(&self, _client: &CpuClient) -> Result<(), CapturedError> {
        Ok(())
    }

    /// Returns the number of vcpus the host machine has
    fn get_cpu_count(&self, _client: &CpuClient) -> Result<u32> {
        Ok(std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1))
    }
}
