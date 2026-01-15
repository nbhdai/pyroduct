//! Serial Port Capability
//! 
//! Pattern: Both client and host state
//! - Host: SerialPoolServer manages actual port connections
//! - Client: SerialHandle identifies which port the client is using

use capability_derive::*;

// ============================================================================
// SHARED TYPES
// ============================================================================

/// Configuration for allowed serial connections
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct SerialConnection {
    pub port_name: String,
    pub baud_rate: u32,
}

/// Server configuration - list of permitted connections
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SerialConfig {
    /// If empty, all connections are permitted
    pub permitted: Vec<SerialConnection>,
}

/// Client-side handle to an open serial port.
/// 
/// This gets serialized and sent with each request so the server
/// knows which port the client is referring to.
#[capability_client]
#[derive(Debug, Clone)]
pub struct SerialHandle {
    port_id: u64,
}

// ============================================================================
// CAPABILITY DEFINITION
// ============================================================================

/// Serial port capability.
/// 
/// This demonstrates the most complex pattern:
/// - Server has state (pool of open ports)
/// - Client has state (handle to specific port)
/// - Some methods return client state (open)
/// - Some methods consume client state conceptually (close)
#[capability]
pub trait SerialPool {
    /// Open a new serial port. Returns a handle for future operations.
    /// No client state input (we're creating new state).
    async fn open(port_name: String, baud_rate: u32) -> Result<SerialHandle, String>;
    
    /// Write data to a serial port. Requires client handle.
    async fn write(
        #[client_state] handle: &SerialHandle, 
        data: Vec<u8>
    ) -> Result<usize, String>;
    
    /// Read data from a serial port. Requires client handle.
    async fn read(
        #[client_state] handle: &SerialHandle, 
        max_bytes: usize
    ) -> Result<Vec<u8>, String>;
    
    /// Close a serial port. Requires client handle.
    /// After this, the handle should not be used.
    fn close(#[client_state] handle: &SerialHandle) -> Result<(), String>;
}

// ============================================================================
// CLIENT-SIDE API
// ============================================================================

#[cfg(target_arch = "wasm32")]
impl SerialHandle {
    /// Open a new serial port connection
    pub fn open(port_name: &str, baud_rate: u32) -> Result<Self, String> {
        serial_pool_client::open(port_name.to_string(), baud_rate)
    }
    
    /// Write data to this port
    pub fn write(&self, data: &[u8]) -> Result<usize, String> {
        serial_pool_client::write(self, data.to_vec())
    }
    
    /// Read up to max_bytes from this port
    pub fn read(&self, max_bytes: usize) -> Result<Vec<u8>, String> {
        serial_pool_client::read(self, max_bytes)
    }
    
    /// Close this port (consumes the handle)
    pub fn close(self) -> Result<(), String> {
        serial_pool_client::close(&self)
    }
}

// ============================================================================
// SERVER IMPLEMENTATION
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod server {
    use super::*;
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Server-side state managing multiple open serial ports.
    /// 
    /// Config attached here to make different configs possible
    #[capability_server(service = SerialPool, config = SerialConfig)]
    pub struct SerialPoolServer {
        permitted: Vec<SerialConnection>,
        ports: HashMap<u64, tokio_serial::SerialStream>,
        next_id: u64,
    }

    impl SerialPoolServer {
        fn check_permitted(&self, port_name: &str, baud_rate: u32) -> Result<(), String> {
            if self.permitted.is_empty() {
                return Ok(());
            }
            
            let conn = SerialConnection {
                port_name: port_name.to_string(),
                baud_rate,
            };
            
            if self.permitted.contains(&conn) {
                Ok(())
            } else {
                Err(format!("Connection to {}@{} not permitted", port_name, baud_rate))
            }
        }
    }

    /// Generated trait by the "capability_server" macro
    impl SerialPoolServerInit for SerialPoolServer {
        fn new() -> Self {
            tracing::info!("SerialPoolServer initialized (no restrictions)");
            Self {
                permitted: Vec::new(),
                ports: HashMap::new(),
                next_id: 1,
            }
        }

        fn with_config(config: SerialConfig) -> Self {
            tracing::info!("SerialPoolServer initialized with {} permitted connections", 
                config.permitted.len());
            Self {
                permitted: config.permitted,
                ports: HashMap::new(),
                next_id: 1,
            }
        }

        fn reset(&mut self) {
            let count = self.ports.len();
            self.ports.clear();
            tracing::debug!("SerialPoolServer reset, closed {} ports", count);
        }
    }

    // These are generated by the configuration trait
    impl SerialPool for SerialPoolServer {
        async fn open(&mut self, port_name: String, baud_rate: u32) -> Result<SerialHandle, String> {
            use tokio_serial::SerialPortBuilderExt;
            
            self.check_permitted(&port_name, baud_rate)?;
            
            let stream = tokio_serial::new(&port_name, baud_rate)
                .open_native_async()
                .map_err(|e| format!("Failed to open '{}': {}", port_name, e))?;
            
            let id = self.next_id;
            self.next_id += 1;
            self.ports.insert(id, stream);
            
            tracing::info!("Opened port '{}' at {} baud (id={})", port_name, baud_rate, id);
            Ok(SerialHandle { port_id: id })
        }

        async fn write(&mut self, handle: &SerialHandle, data: Vec<u8>) -> Result<usize, String> {
            let port = self.ports.get_mut(&handle.port_id)
                .ok_or_else(|| format!("Port {} not found", handle.port_id))?;
            
            let len = data.len();
            port.write_all(&data)
                .await
                .map_err(|e| format!("Write error: {}", e))?;
            
            tracing::debug!("Wrote {} bytes to port {}", len, handle.port_id);
            Ok(len)
        }

        async fn read(&mut self, handle: &SerialHandle, max_bytes: usize) -> Result<Vec<u8>, String> {
            let port = self.ports.get_mut(&handle.port_id)
                .ok_or_else(|| format!("Port {} not found", handle.port_id))?;
            
            let mut buf = vec![0u8; max_bytes];
            let n = port.read(&mut buf)
                .await
                .map_err(|e| format!("Read error: {}", e))?;
            
            buf.truncate(n);
            tracing::debug!("Read {} bytes from port {}", n, handle.port_id);
            Ok(buf)
        }

        fn close(&mut self, handle: &SerialHandle) -> Result<(), String> {
            match self.ports.remove(&handle.port_id) {
                Some(_) => {
                    tracing::info!("Closed port {}", handle.port_id);
                    Ok(())
                }
                None => Err(format!("Port {} not found", handle.port_id)),
            }
        }
    }

    capability_export!(env = "serial_client", SerialPoolServer);
}