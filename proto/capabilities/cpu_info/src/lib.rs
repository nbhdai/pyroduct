// proto/cpu_client/src/lib.rs
//
// PATTERN 1: No state - naked functions on both sides
// ============================================================================

// ============================================================================
// DEVELOPER WRITES: The actual logic (capability side)
// ============================================================================

// pub fn get_cpu_count() -> u32 {
//     std::thread::available_parallelism()
//         .map(|n| n.get() as u32)
//         .unwrap_or(1)
// }

// ============================================================================
// GENERATED/BOILERPLATE: FFI layer (host side)
// ============================================================================

#[cfg(features = "capability")]
mod ffi {

    pub fn get_cpu_count() -> u32 {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1)
    }

    // Consistent signature - client_state and host_state ignored for this pattern
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn host_get_cpu_count(
        client_state_ptr: *const u8,
        client_state_len: usize,
        input_ptr: *const u8,
        input_len: usize,
        host_state_ptr: *mut std::ffi::c_void,
    ) -> ::pyroduct::capability_host::ffi::FfiResult {
        ::pyroduct::capability::safe_call::empty_call(
            client_state_ptr,
            client_state_len,
            input_ptr,
            input_len,
            host_state_ptr,
            || get_cpu_count(),
        )
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plugin_init(_config: *const u8, _config_len: usize) -> *mut std::ffi::c_void {
        std::ptr::null_mut()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn plugin_manifest<'a>(
        id: u64,
        log_callback: ::pyroduct::capability_host::ffi::LogCallback,
    ) -> ::pyroduct::capability_host::ffi::PluginExports<'a> {
        static MOD_NAME: &str = "env";
        static FN_NAME: &str = "host_get_cpu_count";

        ::pyroduct::capability::init_logging(id, log_callback);

        let mut export_vec = vec![::pyroduct::capability_host::ffi::PluginExport {
            module: MOD_NAME.as_ptr(),
            module_len: MOD_NAME.len(),
            name: FN_NAME.as_ptr(),
            name_len: FN_NAME.len(),
            func: ::pyroduct::capability_host::ffi::PluginFunction::Sync(host_get_cpu_count),
        }];

        let exports = ::pyroduct::capability_host::ffi::PluginExports {
            len: export_vec.len(),
            cap: export_vec.capacity(),
            ptr: export_vec.as_mut_ptr(),
            reset: ::pyroduct::capability_host::ffi::PluginResetFn::Null,
            init: ::pyroduct::capability_host::ffi::PluginInitFn::Null,
            drop: ::pyroduct::capability_host::ffi::PluginDropFn::Null,
        };
        std::mem::forget(export_vec);
        exports
    }
}

// ============================================================================
// GENERATED/BOILERPLATE: FFI layer (client/WASM side)
// ============================================================================

#[cfg(features = "module")]
pub fn get_cpu_count() -> u32 {
    // Calls through FFI - see generated section below
    ::pyroduct::module_capability::access::call_from_wasm::<(), (), u32, _>(
        "serial_client",
        None,
        None,
        |client_state_ptr: *const u8,
         client_state_len: usize,
         input_ptr: *const u8,
         input_len: usize| {
            unsafe {
                __functions_client::host_get_cpu_count(
                    client_state_ptr,
                    client_state_len,
                    input_ptr,
                    input_len,
                )
            }
        },
    )
}

#[cfg(features = "module")]
mod __functions_client {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        #[link_name = "host_get_cpu_count"]
        fn host_get_cpu_count(
            client_state_ptr: *const u8,
            client_state_len: usize,
            input_ptr: *const u8,
            input_len: usize,
        ) -> *const u8;
    }
}
