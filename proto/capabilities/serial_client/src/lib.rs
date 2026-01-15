// proto/serial_client/src/lib.rs
//
// PATTERN 4: Both client and host state
//
// Host state: SerialPool - manages actual serial port connections
// Client state: SerialHandle - identifies which port the client is using
// ============================================================================

// ============================================================================
// SHARED: Client state definition (rkyv serializable)
// ============================================================================

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct SerialHandle {
    pub port_id: u64,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    not(target_arch = "wasm32"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct Connection {
    pub port_name: String,
    pub baud_rate: u32,
}

// ============================================================================
// DEVELOPER WRITES: Host state and methods
// ============================================================================

use std::collections::HashMap;

pub struct SerialPool {
    permitted: Vec<Connection>,
    ports: HashMap<u64, tokio_serial::SerialStream>,
    next_id: u64,
}

#[cfg(not(target_arch = "wasm32"))]
impl SerialPool {
    pub fn new() -> Self {
        println!("   (Plugin): SerialPool initialized");
        SerialPool {
            permitted: Vec::new(),
            ports: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn with_config(permitted: Vec<Connection>) -> Self {
        println!("   (Plugin): SerialPool initialized");
        SerialPool {
            permitted,
            ports: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn reset(&mut self) {
        self.ports.clear();
        println!("   (Plugin): SerialPool reset, all ports closed");
    }

    /// Open a new serial port - returns a handle for the client
    /// No client state needed for this call (creating new connection)
    pub async fn open(
        &mut self,
        port_name: String,
        baud_rate: u32,
    ) -> Result<SerialHandle, String> {
        use tokio_serial::SerialPortBuilderExt;
        let connection = Connection {
            port_name,
            baud_rate,
        };
        if !self.permitted.is_empty() && self.permitted.iter().all(|c| c != &connection) {
            return Err("Not permitted".to_string());
        }

        match tokio_serial::new(&connection.port_name, connection.baud_rate).open_native_async() {
            Ok(stream) => {
                let id = self.next_id;
                self.next_id += 1;
                self.ports.insert(id, stream);
                println!(
                    "   (Plugin): Opened port '{}' at {} baud (id={})",
                    connection.port_name, baud_rate, id
                );
                Ok(SerialHandle { port_id: id })
            }
            Err(e) => Err(format!("Failed to open '{}': {}", connection.port_name, e)),
        }
    }

    /// Write data to a serial port - requires client handle
    pub async fn write(&mut self, client: &SerialHandle, data: &[u8]) -> Result<usize, String> {
        use tokio::io::AsyncWriteExt;

        match self.ports.get_mut(&client.port_id) {
            Some(port) => {
                let len = data.len();
                match port.write_all(&data).await {
                    Ok(_) => {
                        println!(
                            "   (Plugin): Wrote {} bytes to port {}",
                            len, client.port_id
                        );
                        Ok(len)
                    }
                    Err(e) => Err(format!("Write error: {}", e)),
                }
            }
            None => Err(format!("Port {} not found", client.port_id)),
        }
    }

    /// Read data from a serial port - requires client handle
    pub async fn read(
        &mut self,
        client: &SerialHandle,
        max_bytes: usize,
    ) -> Result<Vec<u8>, String> {
        use tokio::io::AsyncReadExt;

        match self.ports.get_mut(&client.port_id) {
            Some(port) => {
                let mut buf = vec![0u8; max_bytes];
                match port.read(&mut buf).await {
                    Ok(n) => {
                        buf.truncate(n);
                        println!("   (Plugin): Read {} bytes from port {}", n, client.port_id);
                        Ok(buf)
                    }
                    Err(e) => Err(format!("Read error: {}", e)),
                }
            }
            None => Err(format!("Port {} not found", client.port_id)),
        }
    }

    /// Close a serial port - requires client handle
    pub fn close(&mut self, client: &SerialHandle) -> Result<(), String> {
        match self.ports.remove(&client.port_id) {
            Some(_) => {
                println!("   (Plugin): Closed port {}", client.port_id);
                Ok(())
            }
            None => Err(format!("Port {} not found", client.port_id)),
        }
    }
}

// ============================================================================
// DEVELOPER WRITES: Client interface (methods on client handle)
// ============================================================================

#[cfg(target_arch = "wasm32")]
impl SerialHandle {
    /// Open a new serial port
    pub fn open(port_name: String, baud_rate: u32) -> Result<Self, String> {
        serial_pool_client::call_open(port_name, baud_rate)
    }

    /// Write data to this port
    pub fn write(&self, data: &[u8]) -> Result<usize, String> {
        serial_pool_client::call_write(self, data.to_vec())
    }

    /// Read up to max_bytes from this port
    pub fn read(&self, max_bytes: usize) -> Result<Vec<u8>, String> {
        serial_pool_client::call_read(self, max_bytes)
    }

    /// Close this port
    pub fn close(self) -> Result<(), String> {
        serial_pool_client::call_close(&self)
    }
}

// ============================================================================
// SHARED: Input/Output types for FFI serialization
// ============================================================================

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
#[rkyv(compare(PartialEq), derive(Debug))]
struct OpenInput {
    pub port_name: String,
    pub baud_rate: u32,
}

// ============================================================================
// GENERATED/BOILERPLATE: FFI layer (host side)
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
pub mod ffi {
    

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct __SerialPoolConfig {
        permitted: Vec<crate::Connection>,
    }

    // --- open: no client state, returns handle ---
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn host_serial_open<'a>(
        client_state_ptr: *const u8,
        client_state_len: usize,
        input_ptr: *const u8,
        input_len: usize,
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
        ::pyroduct::capability::safe_async::async_si_call::<
            crate::SerialPool,
            crate::OpenInput,
            Result<crate::SerialHandle, String>,
            _,
            _,
        >(
            client_state_ptr,
            client_state_len,
            input_ptr,
            input_len,
            host_state_ptr,
            |state, input| async move { state.open(input.port_name, input.baud_rate).await },
        )
    }

    // --- write: requires client state ---
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn host_serial_write<'a>(
        client_state_ptr: *const u8,
        client_state_len: usize,
        input_ptr: *const u8,
        input_len: usize,
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
        ::pyroduct::capability::safe_async::async_sci_call::<
            crate::SerialPool,
            crate::SerialHandle,
            Vec<u8>,
            Result<usize, String>,
            _,
            _,
        >(
            client_state_ptr,
            client_state_len,
            input_ptr,
            input_len,
            host_state_ptr,
            |state, client, input| async move { state.write(&client, &input).await },
        )
    }

    // --- read: requires client state ---
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn host_serial_read<'a>(
        client_state_ptr: *const u8,
        client_state_len: usize,
        input_ptr: *const u8,
        input_len: usize,
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'a> {
        ::pyroduct::capability::safe_async::async_sci_call::<
            crate::SerialPool,
            crate::SerialHandle,
            usize,
            Result<Vec<u8>, String>,
            _,
            _,
        >(
            client_state_ptr,
            client_state_len,
            input_ptr,
            input_len,
            host_state_ptr,
            |state, client, input| async move { state.read(&client, input).await },
        )
    }

    // --- close: requires client state, sync ---
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn host_serial_close<'a>(
        client_state_ptr: *const u8,
        client_state_len: usize,
        input_ptr: *const u8,
        input_len: usize,
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiResult {
        ::pyroduct::capability::safe_call::sc_call::<
            crate::SerialPool,
            crate::SerialHandle,
            Result<(), String>,
            _,
        >(
            client_state_ptr,
            client_state_len,
            input_ptr,
            input_len,
            host_state_ptr,
            |state, client| state.close(&client),
        )
    }

    // --- Lifecycle ---

    #[unsafe(no_mangle)]
    pub extern "C" fn plugin_init(
        config: *const u8,
        config_len: usize,
    ) -> ::pyroduct::capability_host::ffi::FfiInitResult {
        unsafe {
            ::pyroduct::capability::safe_lifecycle::execute_safe_init::<
                __SerialPoolConfig,
                crate::SerialPool,
                _,
            >(config, config_len, |config: __SerialPoolConfig| {
                crate::SerialPool::with_config(config.permitted)
            })
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn plugin_drop(state: *mut std::ffi::c_void) {
        drop(unsafe { Box::from_raw(state as *mut crate::SerialPool) });
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn plugin_reset<'a>(
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiResult {
        unsafe {
            ::pyroduct::capability::safe_lifecycle::execute_safe_reset::<crate::SerialPool, _>(
                host_state_ptr,
                |state| state.reset(),
            )
        }
    }

    // --- Manifest ---

    #[unsafe(no_mangle)]
    pub extern "C" fn plugin_manifest<'a>(
        id: u64,
        log_callback: ::pyroduct::capability_host::ffi::LogCallback,
    ) -> ::pyroduct::capability_host::ffi::PluginExports<'a> {
        static MOD: &str = "env";
        static FN_OPEN: &str = "host_serial_open";
        static FN_WRITE: &str = "host_serial_write";
        static FN_READ: &str = "host_serial_read";
        static FN_CLOSE: &str = "host_serial_close";
        ::pyroduct::capability::init_logging(id, log_callback);

        let mut exports = vec![
            ::pyroduct::capability_host::ffi::PluginExport {
                module: MOD.as_ptr(),
                module_len: MOD.len(),
                name: FN_OPEN.as_ptr(),
                name_len: FN_OPEN.len(),
                func: ::pyroduct::capability_host::ffi::PluginFunction::Async(host_serial_open),
            },
            ::pyroduct::capability_host::ffi::PluginExport {
                module: MOD.as_ptr(),
                module_len: MOD.len(),
                name: FN_WRITE.as_ptr(),
                name_len: FN_WRITE.len(),
                func: ::pyroduct::capability_host::ffi::PluginFunction::Async(host_serial_write),
            },
            ::pyroduct::capability_host::ffi::PluginExport {
                module: MOD.as_ptr(),
                module_len: MOD.len(),
                name: FN_READ.as_ptr(),
                name_len: FN_READ.len(),
                func: ::pyroduct::capability_host::ffi::PluginFunction::Async(host_serial_read),
            },
            ::pyroduct::capability_host::ffi::PluginExport {
                module: MOD.as_ptr(),
                module_len: MOD.len(),
                name: FN_CLOSE.as_ptr(),
                name_len: FN_CLOSE.len(),
                func: ::pyroduct::capability_host::ffi::PluginFunction::Sync(host_serial_close),
            },
        ];

        let result = ::pyroduct::capability_host::ffi::PluginExports {
            len: exports.len(),
            cap: exports.capacity(),
            ptr: exports.as_mut_ptr(),
            reset: ::pyroduct::capability_host::ffi::PluginResetFn::Sync(plugin_reset),
            init: ::pyroduct::capability_host::ffi::PluginInitFn::Sync(plugin_init),
            drop: ::pyroduct::capability_host::ffi::PluginDropFn::Sync(plugin_drop),
        };
        std::mem::forget(exports);
        result
    }
}

// ============================================================================
// GENERATED/BOILERPLATE: FFI layer (client/WASM side)
// ============================================================================

pub mod __serial_pool_client {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        fn host_serial_open(
            client_state_ptr: *const u8,
            client_state_len: usize,
            input_ptr: *const u8,
            input_len: usize,
        ) -> *const u8;

        fn host_serial_write(
            client_state_ptr: *const u8,
            client_state_len: usize,
            input_ptr: *const u8,
            input_len: usize,
        ) -> *const u8;

        fn host_serial_read(
            client_state_ptr: *const u8,
            client_state_len: usize,
            input_ptr: *const u8,
            input_len: usize,
        ) -> *const u8;

        fn host_serial_close(
            client_state_ptr: *const u8,
            client_state_len: usize,
            input_ptr: *const u8,
            input_len: usize,
        ) -> *const u8;
    }

    pub fn call_open(port_name: String, baud_rate: u32) -> Result<crate::SerialHandle, String> {
        let input = crate::OpenInput {
            port_name,
            baud_rate,
        };
        ::pyroduct::module_capability::access::call_from_wasm::<
            (),
            crate::OpenInput,
            Result<crate::SerialHandle, String>,
            _,
        >(
            "serial_client",
            None,
            Some(&input),
            |client_state_ptr: *const u8,
             client_state_len: usize,
             input_ptr: *const u8,
             input_len: usize| {
                unsafe {
                    host_serial_open(client_state_ptr, client_state_len, input_ptr, input_len)
                }
            },
        )
    }

    pub fn call_write(handle: &crate::SerialHandle, data: Vec<u8>) -> Result<usize, String> {
        ::pyroduct::module_capability::access::call_from_wasm::<
            crate::SerialHandle,
            Vec<u8>,
            Result<usize, String>,
            _,
        >(
            "serial_client",
            Some(handle),
            Some(&data),
            |client_state_ptr: *const u8,
             client_state_len: usize,
             input_ptr: *const u8,
             input_len: usize| {
                unsafe {
                    host_serial_write(client_state_ptr, client_state_len, input_ptr, input_len)
                }
            },
        )
    }

    pub fn call_read(handle: &crate::SerialHandle, max_bytes: usize) -> Result<Vec<u8>, String> {
        ::pyroduct::module_capability::access::call_from_wasm::<
            crate::SerialHandle,
            usize,
            Result<Vec<u8>, String>,
            _,
        >(
            "serial_client",
            Some(handle),
            Some(&max_bytes),
            |client_state_ptr: *const u8,
             client_state_len: usize,
             input_ptr: *const u8,
             input_len: usize| {
                unsafe {
                    host_serial_read(client_state_ptr, client_state_len, input_ptr, input_len)
                }
            },
        )
    }

    pub fn call_close(handle: &crate::SerialHandle) -> Result<(), String> {
        ::pyroduct::module_capability::access::call_from_wasm::<
            crate::SerialHandle,
            (),
            Result<(), String>,
            _,
        >(
            "serial_client",
            Some(handle),
            None,
            |client_state_ptr: *const u8,
             client_state_len: usize,
             input_ptr: *const u8,
             input_len: usize| {
                unsafe {
                    host_serial_close(client_state_ptr, client_state_len, input_ptr, input_len)
                }
            },
        )
    }
}
