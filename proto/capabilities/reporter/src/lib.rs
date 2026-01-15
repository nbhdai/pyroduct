// PATTERN 2: Host state only - methods on host struct, client is unaware
// ============================================================================

// ============================================================================
// DEVELOPER WRITES: Host state and methods
// ============================================================================

#[cfg(any(not(target_arch = "wasm32")))]
use std::collections::VecDeque;

#[cfg(any(not(target_arch = "wasm32")))]
pub struct ReporterServer {
    logs: VecDeque<String>,
    max_history: usize,
}

#[cfg(any(not(target_arch = "wasm32")))]
impl ReporterServer {
    pub fn new() -> Self {
        println!("   (Plugin): Initialized new state.");
        ReporterServer {
            logs: VecDeque::new(),
            max_history: 10,
        }
    }

    pub fn with_config(max_history: usize) -> Self {
        println!("   (Plugin): Initialized new state.");
        ReporterServer {
            logs: VecDeque::new(),
            max_history,
        }
    }

    pub fn reset(&mut self) {
        self.logs.clear();
        println!("   (Plugin): State reset.");
    }

    pub fn report(&mut self, message: &str) -> String {
        let history_len = self.logs.len();
        println!(
            "   (Plugin): Processing '{}' (History count: {})",
            message, history_len
        );
        self.logs.push_back(message.to_string());
        if self.logs.len() > self.max_history {
            self.logs.pop_front();
        }
        format!("Processed: '{}' | History: {:?}", message, self.logs)
    }
}

// ============================================================================
// GENERATED/BOILERPLATE: FFI layer (host side)
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
mod reporter_ffi {

    use crate::ReporterServer;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct __ReporterConfig {
        max_history: usize,
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn host_report(
        client_state_ptr: *const u8, // ignored - no client state
        client_state_len: usize,
        input_ptr: *const u8,
        input_len: usize,
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiResult {
        ::pyroduct::capability::safe_call::si_call::<crate::ReporterServer, String, String, _>(
            client_state_ptr,
            client_state_len,
            input_ptr,
            input_len,
            host_state_ptr,
            |state, input| state.report(&input),
        )
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plugin_init(
        config: *const u8,
        config_len: usize,
    ) -> ::pyroduct::capability_host::ffi::FfiInitResult {
        unsafe {
            ::pyroduct::capability::safe_lifecycle::execute_safe_init::<
                __ReporterConfig,
                crate::ReporterServer,
                _,
            >(config, config_len, |config: __ReporterConfig| {
                crate::ReporterServer::with_config(config.max_history)
            })
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn plugin_drop(state: *mut std::ffi::c_void) {
        drop(unsafe { Box::from_raw(state as *mut crate::ReporterServer) });
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn plugin_reset(
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiResult {
        let state: &mut ReporterServer = match unsafe {
            ::pyroduct::capability::safe_io::get_capability_state(host_state_ptr)
        } {
            Ok(state) => state,
            Err(err) => return ::pyroduct::capability::safe_io::make_error_output(err),
        };

        state.reset();
        ::pyroduct::capability_host::ffi::FfiResult::ok_null()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plugin_manifest<'a>(
        id: u64,
        log_callback: ::pyroduct::capability_host::ffi::LogCallback,
    ) -> ::pyroduct::capability_host::ffi::PluginExports<'a> {
        static MOD_NAME: &str = "env";
        static FN_NAME: &str = "host_report";
        ::pyroduct::capability::init_logging(id, log_callback);

        let mut export_vec = vec![::pyroduct::capability_host::ffi::PluginExport {
            module: MOD_NAME.as_ptr(),
            module_len: MOD_NAME.len(),
            name: FN_NAME.as_ptr(),
            name_len: FN_NAME.len(),
            func: ::pyroduct::capability_host::ffi::PluginFunction::Sync(host_report),
        }];

        let exports = ::pyroduct::capability_host::ffi::PluginExports {
            len: export_vec.len(),
            cap: export_vec.capacity(),
            ptr: export_vec.as_mut_ptr(),
            reset: ::pyroduct::capability_host::ffi::PluginResetFn::Sync(plugin_reset),
            init: ::pyroduct::capability_host::ffi::PluginInitFn::Sync(plugin_init),
            drop: ::pyroduct::capability_host::ffi::PluginDropFn::Sync(plugin_drop),
        };
        std::mem::forget(export_vec);
        exports
    }
}

// ============================================================================
// GENERATED/BOILERPLATE: FFI layer (client/WASM side)
// ============================================================================

#[cfg(feature = "module")]
pub fn report(message: String) -> String {
    __functions_client::report(&message)
}

#[cfg(feature = "module")]
mod __functions_client {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        #[link_name = "host_report"]
        fn host_report(
            client_state_ptr: *const u8,
            client_state_len: usize,
            input_ptr: *const u8,
            input_len: usize,
        ) -> *const u8;
    }

    pub fn report(message: &String) -> String {
        ::pyroduct::module_capability::access::call_from_wasm::<(), String, String, _>(
            "serial_client",
            None,
            Some(message),
            |client_state_ptr: *const u8,
             client_state_len: usize,
             input_ptr: *const u8,
             input_len: usize| {
                unsafe { host_report(client_state_ptr, client_state_len, input_ptr, input_len) }
            },
        )
    }
}
