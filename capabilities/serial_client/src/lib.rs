#[pyroduct::config]
pub struct SerialConfig {
    pub allowed_ports: Vec<AllowedPort>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct AllowedPort {
    pub path: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
}

#[pyroduct::client]
pub struct SerialClient {
    pub port_path: String,
}

use std::collections::HashMap;
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct SerialServer {
    allowed_ports: HashMap<String, AllowedPort>,
    connections: Mutex<HashMap<String, tokio_serial::SerialStream>>,
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
        
        Self {
            allowed_ports,
            connections: Mutex::new(HashMap::new()),
        }
    }
    
    fn reset(&mut self) {
        let mut conns = self.connections.lock().unwrap();
        conns.clear();
    }
    
    fn new_client(&self, client: &SerialClient) -> Result<(), String> {
        if !self.allowed_ports.contains_key(&client.port_path) {
            return Err(format!(
                "Port {} is not in the allowlist. Allowed ports: {:?}",
                client.port_path,
                self.allowed_ports.keys().collect::<Vec<_>>()
            ));
        }
        Ok(())
    }
    
    fn open(&self, client: &SerialClient) -> Result<(), String> {
        let port_config = self.allowed_ports
            .get(&client.port_path)
            .ok_or_else(|| format!("Port {} not allowed", client.port_path))?;
        
        let mut conns = self.connections.lock().unwrap();
        
        if conns.contains_key(&client.port_path) {
            return Ok(()); // Already open
        }
        
        let port = tokio_serial::new(&port_config.path, port_config.baud_rate)
            .timeout(std::time::Duration::from_millis(port_config.timeout_ms))
            .open_native_async()
            .map_err(|e| format!("Failed to open port {}: {}", port_config.path, e))?;
        
        conns.insert(client.port_path.clone(), port);
        Ok(())
    }
    
    fn close(&self, client: &SerialClient) -> Result<(), String> {
        let mut conns = self.connections.lock().unwrap();
        conns.remove(&client.port_path);
        Ok(())
    }
    
    async fn write(&self, client: &SerialClient, data: Vec<u8>) -> Result<usize, String> {
        let mut conns = self.connections.lock().unwrap();
        let port = conns
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;
        
        port.write(&data)
            .await
            .map_err(|e| format!("Write failed: {}", e))
    }
    
    async fn read(&self, client: &SerialClient, max_bytes: usize) -> Result<Vec<u8>, String> {
        let mut conns = self.connections.lock().unwrap();
        let port = conns
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;
        
        let mut buf = vec![0u8; max_bytes];
        let n = port.read(&mut buf)
            .await
            .map_err(|e| format!("Read failed: {}", e))?;
        
        buf.truncate(n);
        Ok(buf)
    }
    
    async fn write_line(&self, client: &SerialClient, line: String) -> Result<usize, String> {
        let mut data = line.into_bytes();
        data.push(b'\n');
        self.write(client, data).await
    }
    
    async fn read_line(&self, client: &SerialClient) -> Result<String, String> {
        let mut conns = self.connections.lock().unwrap();
        let port = conns
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;
        
        let mut result = Vec::new();
        let mut byte = [0u8; 1];
        
        loop {
            match port.read(&mut byte).await {
                Ok(1) => {
                    if byte[0] == b'\n' {
                        break;
                    }
                    result.push(byte[0]);
                }
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(e) => return Err(format!("Read failed: {}", e)),
            }
        }
        
        String::from_utf8(result)
            .map_err(|e| format!("Invalid UTF-8: {}", e))
    }
    
    fn available(&self, client: &SerialClient) -> Result<usize, String> {
        let conns = self.connections.lock().unwrap();
        let port = conns
            .get(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;
        
        port.bytes_to_read()
            .map(|n| n as usize)
            .map_err(|e| format!("Failed to check available bytes: {}", e))
    }
    
    async fn flush(&self, client: &SerialClient) -> Result<(), String> {
        let mut conns = self.connections.lock().unwrap();
        let port = conns
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;
        
        port.flush()
            .await
            .map_err(|e| format!("Flush failed: {}", e))
    }
}