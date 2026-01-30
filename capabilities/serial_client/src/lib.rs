// Serial port capability
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
use std::io::{Read, Write};

pub struct SerialServer {
    allowed_ports: HashMap<String, AllowedPort>,
    connections: Mutex<HashMap<String, Box<dyn SerialPort + Send>>>,
}

// Trait to abstract over serial port implementations
trait SerialPort: Read + Write {
    fn bytes_to_read(&self) -> std::io::Result<usize>;
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
        
        let port = serialport::new(&port_config.path, port_config.baud_rate)
            .timeout(std::time::Duration::from_millis(port_config.timeout_ms))
            .open()
            .map_err(|e| format!("Failed to open port {}: {}", port_config.path, e))?;
        
        conns.insert(client.port_path.clone(), Box::new(PortWrapper(port)));
        Ok(())
    }
    
    fn close(&self, client: &SerialClient) -> Result<(), String> {
        let mut conns = self.connections.lock().unwrap();
        conns.remove(&client.port_path);
        Ok(())
    }
    
    fn write(&self, client: &SerialClient, data: Vec<u8>) -> Result<usize, String> {
        let mut conns = self.connections.lock().unwrap();
        let port = conns
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;
        
        port.write(&data)
            .map_err(|e| format!("Write failed: {}", e))
    }
    
    fn read(&self, client: &SerialClient, max_bytes: usize) -> Result<Vec<u8>, String> {
        let mut conns = self.connections.lock().unwrap();
        let port = conns
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;
        
        let mut buf = vec![0u8; max_bytes];
        let n = port.read(&mut buf)
            .map_err(|e| format!("Read failed: {}", e))?;
        
        buf.truncate(n);
        Ok(buf)
    }
    
    fn write_line(&self, client: &SerialClient, line: String) -> Result<usize, String> {
        let mut data = line.into_bytes();
        data.push(b'\n');
        self.write(client, data)
    }
    
    fn read_line(&self, client: &SerialClient) -> Result<String, String> {
        let mut conns = self.connections.lock().unwrap();
        let port = conns
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;
        
        let mut result = Vec::new();
        let mut byte = [0u8; 1];
        
        loop {
            match port.read(&mut byte) {
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
            .map_err(|e| format!("Failed to check available bytes: {}", e))
    }
    
    fn flush(&self, client: &SerialClient) -> Result<(), String> {
        let mut conns = self.connections.lock().unwrap();
        let port = conns
            .get_mut(&client.port_path)
            .ok_or_else(|| format!("Port {} is not open", client.port_path))?;
        
        port.flush()
            .map_err(|e| format!("Flush failed: {}", e))
    }
}

// Wrapper to implement our trait for serialport
struct PortWrapper(Box<dyn serialport::SerialPort>);

impl Read for PortWrapper {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for PortWrapper {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl SerialPort for PortWrapper {
    fn bytes_to_read(&self) -> std::io::Result<usize> {
        self.0.bytes_to_read().map(|n| n as usize)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}