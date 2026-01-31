use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::SerialPortBuilderExt; // Crucial trait for open_native_async

// --- Configuration Structs ---

#[pyroduct::config]
#[derive(Clone, Debug, PartialEq)]
pub struct AllowedPort {
    pub path: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
}

#[pyroduct::config]
#[derive(Clone, Debug)]
pub struct SerialConfig {
    pub allowed_ports: Vec<AllowedPort>,
}



// --- Client Struct ---

#[pyroduct::client]
pub struct SerialClient {
    pub port_path: String,
}

// --- Server Implementation ---

pub struct SerialServer {
    allowed_ports: HashMap<String, AllowedPort>,
    // Mapping path -> active stream
    connections: HashMap<String, tokio_serial::SerialStream>,
}

#[pyroduct::capability]
impl SerialServer {
    type Client = SerialClient;
    type Config = SerialConfig;
    type Error = String;

    fn new(config: Option<SerialConfig>) -> Self {
        let config = config.unwrap_or(SerialConfig {
            allowed_ports: Vec::new(),
        });

        let allowed_ports: HashMap<String, AllowedPort> = config
            .allowed_ports
            .into_iter()
            .map(|p| (p.path.clone(), p))
            .collect();
        
        println!("(SerialServer): Initialized with {} allowed ports", allowed_ports.len());

        Self {
            allowed_ports,
            connections: HashMap::new(),
        }
    }

    fn reset(&mut self) {
        self.connections.clear();
        println!("(SerialServer): Reset, all ports closed");
    }

    /// Validates if a client is attempting to access a configured port.
    fn new_client(&self, client: &SerialClient) -> Result<(), String> {
        if !self.allowed_ports.contains_key(&client.port_path) {
            return Err(format!(
                "Port '{}' is not in the allowlist. Available: {:?}",
                client.port_path,
                self.allowed_ports.keys().collect::<Vec<_>>()
            ));
        }
        Ok(())
    }

    /// Opens the serial port using tokio-serial's native async builder.
    fn open(&mut self, client: &SerialClient) -> Result<(), String> {
        // 1. Check if allowed
        let port_config = self.allowed_ports
            .get(&client.port_path)
            .ok_or_else(|| format!("Port {} not allowed", client.port_path))?;

        // 2. Check if already open
        if self.connections.contains_key(&client.port_path) {
            return Ok(()); 
        }

        // 3. Open using the working builder pattern
        let port = tokio_serial::new(&port_config.path, port_config.baud_rate)
            .timeout(std::time::Duration::from_millis(port_config.timeout_ms))
            .open_native_async()
            .map_err(|e| format!("Failed to open '{}': {}", port_config.path, e))?;

        self.connections.insert(client.port_path.clone(), port);
        
        println!(
            "(SerialServer): Opened port '{}' at {} baud",
            client.port_path, port_config.baud_rate
        );
        
        Ok(())
    }

    fn close(&mut self, client: &SerialClient) -> Result<(), String> {
        if self.connections.remove(&client.port_path).is_some() {
            println!("(SerialServer): Closed port '{}'", client.port_path);
        }
        Ok(())
    }

    async fn write(&mut self, client: &SerialClient, data: Vec<u8>) -> Result<usize, String> {
        let port = self.connections
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;

        let len = data.len();
        
        // Using write_all ensures complete transmission
        port.write_all(&data)
            .await
            .map_err(|e| format!("Write failed: {}", e))?;
            
        println!("(SerialServer): Wrote {} bytes to '{}'", len, client.port_path);
        Ok(len)
    }

    async fn read(&mut self, client: &SerialClient, max_bytes: usize) -> Result<Vec<u8>, String> {
        let port = self.connections
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;

        let mut buf = vec![0u8; max_bytes];
        
        // Reads as much as is currently available up to max_bytes
        let n = port.read(&mut buf)
            .await
            .map_err(|e| format!("Read failed: {}", e))?;

        buf.truncate(n);
        println!("(SerialServer): Read {} bytes from '{}'", n, client.port_path);
        Ok(buf)
    }

    // --- Helper Utilities (Wrappers around read/write) ---

    async fn write_line(&mut self, client: &SerialClient, line: String) -> Result<usize, String> {
        let mut data = line.into_bytes();
        data.push(b'\n');
        self.write(client, data).await
    }

    async fn read_line(&mut self, client: &SerialClient) -> Result<String, String> {
        let port = self.connections
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;

        let mut result = Vec::new();
        let mut byte = [0u8; 1];

        // Byte-by-byte read looking for newline
        loop {
            match port.read(&mut byte).await {
                Ok(1) => {
                    if byte[0] == b'\n' {
                        break;
                    }
                    result.push(byte[0]);
                }
                Ok(_) => break, // EOF
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(e) => return Err(format!("Read line failed: {}", e)),
            }
        }

        String::from_utf8(result)
            .map_err(|e| format!("Invalid UTF-8 in response: {}", e))
    }
    
    async fn flush(&mut self, client: &SerialClient) -> Result<(), String> {
        let port = self.connections
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;
        
        port.flush().await.map_err(|e| format!("Flush failed: {}", e))
    }
}