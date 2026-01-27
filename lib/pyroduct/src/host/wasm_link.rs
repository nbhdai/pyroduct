use libloading::{Library, Symbol};
use pin_project::pin_project;
use std::ffi::c_void;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Arc, RwLock};
use tracing::{Span, error, info};
use wasmtime::{Caller, Linker};

use crate::PyroductResult;
use crate::capability_host::ffi::{
    AsyncFn, ClassDropFn, FunctionExport, Function, ClassInitFn, CapabilityRegisterFn, ClassResetFn, SyncFn
};
use crate::errors::{FfiError, PyroductError};
use crate::host::class::CapClass;
use crate::host::ffi_bridge::{
    AsyncExecFuture, AsyncInitFuture, CapabilityInit, CapabilityReset, ExecutionResultBridge, InitResultBridge
};
use crate::host::function::CapFunction;

// --- Type definitions for clarity ---
type WasmArgs = (i32, i32, i32, i32); // (wasm_state_ptr, wasm_state_len, ptr, len)

pub struct StatePtr {
    pub(super) ptr: *mut c_void,
    pub(super) destroy_fn: ClassDropFn,
}

unsafe impl Send for StatePtr {}

impl Drop for StatePtr {
    fn drop(&mut self) {
        match self.destroy_fn {
            ClassDropFn::Sync(destroy_fn) => {
                if !self.ptr.is_null() {
                    unsafe { (destroy_fn)(self.ptr) }
                } else {
                    tracing::error!("Drop function exists, but state pointer is null");
                }
            }
            ClassDropFn::Null => {
                if !self.ptr.is_null() {
                    tracing::error!("Drop function does not exist, and pointer is non-null");
                }
            }
        }
    }
}

pub struct HarnessState {
    // Map capability index -> Opaque State Pointer
    pub cap_states: Vec<StatePtr>,
    /// Shared slot for an error that occurred during a host function call
    pub error_slot: Option<PyroductError>,
}

pub struct Capabilities {
    wasm_name: String,
    wasm_path: PathBuf,
    caps: Vec<Arc<dyn Capability>>,
}


impl HarnessState {
    pub async fn new(
        wasm_name: String,
        wasm_path: PathBuf,
        caps: Vec<Arc<dyn Capability>>,
        configs: Vec<Option<&serde_json::Value>>,
    ) -> PyroductResult<(Self, Capabilities)> {
        let mut cap_states = Vec::new();
        for (cap, config) in caps.iter().zip(configs.iter()) {
            let state_ptr = cap.init(config)?.await?;
            cap_states.push(state_ptr);
        }
        Ok((
            HarnessState {
                cap_states,
                error_slot: None,
            },
            Capabilities {
                wasm_name,
                wasm_path,
                caps,
            },
        ))
    }

    pub fn take_error(&mut self) -> Option<PyroductError> {
        self.error_slot.take()
    }

    pub fn set_error(&mut self, error: PyroductError) -> anyhow::Error {
        let ret_error = anyhow::anyhow!("Error: {error}");
        self.error_slot = Some(error);
        ret_error
    }
}


// Implement the extension trait for all Capabilities
impl<T: Capability + ?Sized> CapabilityExt for T {}

// -------------------------------------------------------------------------
// REFACTORED Capabilities Implementation
// -------------------------------------------------------------------------

impl Capabilities {
    pub fn attach_imports(&self, linker: &mut Linker<HarnessState>) {
        for (index, cap) in self.caps.iter().enumerate() {
            for (mod_name, func_name, func_enum) in cap.imports().into_iter() {
                // Capture strict copies for the closures
                let mod_name_log = mod_name.clone();
                let func_name_log = func_name.clone();
                let index_log = index;
                let cap_clone = cap.clone();
                let wasm_name = self.wasm_name.clone();
                let wasm_path = self.wasm_path.clone();

                match func_enum {
                    Function::Sync(raw_fn) => {
                        linker.func_wrap_async(
                            &mod_name,
                            &func_name,
                            move |caller, args: (i32, i32, i32, i32)| {
                                let mod_name = mod_name_log.clone();
                                let func_name = func_name_log.clone();
                                let cap = cap_clone.clone();
                                let w_name = wasm_name.clone();
                                let w_path = wasm_path.clone();

                                Box::new(async move {
                                    info!(
                                        "[Plugin -> Capability] Sync Call: {}::{} (CapIdx: {}) | Ptr: {:#x}, Len: {}", 
                                        mod_name, func_name, index_log, args.2, args.3
                                    );

                                    // DELEGATE TO CAPABILITY EXTENSION
                                    cap.process_sync_call(caller, raw_fn, args, index_log, w_name, w_path).await
                                })
                            },
                        ).expect("Failed to link sync function");
                    }
                    Function::Async(raw_fn) => {
                        linker.func_wrap_async(
                            &mod_name,
                            &func_name,
                            move |caller, args: (i32, i32, i32, i32)| {
                                let mod_name = mod_name_log.clone();
                                let func_name = func_name_log.clone();
                                let cap = cap_clone.clone();
                                let w_name = wasm_name.clone();
                                let w_path = wasm_path.clone();

                                Box::new(async move {
                                    info!(
                                        "[Host -> Plugin] Async Call: {}::{} (CapIdx: {}) | Ptr: {:#x}, Len: {}", 
                                        mod_name, func_name, index_log, args.2, args.3
                                    );

                                    // DELEGATE TO CAPABILITY EXTENSION
                                    cap.process_async_call(caller, raw_fn, args, index_log, w_name, w_path).await
                                })
                            },
                        ).expect("Failed to link async function");
                    }
                }
            }
        }
    }

    fn get_memory(&self) -> PyroductResult<> {
                let memory = caller
            .get_export("memory")
            .ok_or_else(|| {
                PyroductError::from_module_linking(
                    wasm_name.to_string(),
                    wasm_path.to_path_buf(),
                    "Missing 'memory'",
                )
            })?
            .into_memory()
            .ok_or_else(|| {
                PyroductError::from_module_linking(
                    wasm_name.to_string(),
                    wasm_path.to_path_buf(),
                    "The 'memory' in the module isn't actually memory",
                )
            })?;

        let (mem_slice, _) = memory.data_and_store_mut(caller);

        // Validate bounds
        if ptr as usize + len as usize > mem_slice.len() {
            error!("Segfault Risk: Input pointer out of WASM memory bounds!");
            return Err(PyroductError::from_module_linking(
                wasm_name.to_string(),
                wasm_path.to_path_buf(),
                "Returned memory pointer out of bounds",
            ));
        }

        if wasm_state_ptr as usize + wasm_state_len as usize > mem_slice.len() {
            error!("Segfault Risk: Client pointer out of WASM memory bounds!");
            return Err(PyroductError::from_module_linking(
                wasm_name.to_string(),
                wasm_path.to_path_buf(),
                "Returned memory pointer out of bounds",
            ));
        }
    }

    pub async fn reset_states(&self, states: &mut HarnessState) -> PyroductResult<()> {
        for (i, cap) in self.caps.iter().enumerate() {
            if let Some(state) = states.cap_states.get_mut(i) {
                cap.reset(state).await?;
            }
        }
        Ok(())
    }
}


pub struct DynamicCapability {
    name: String,
    path: PathBuf,
    #[allow(dead_code)]
    library: Arc<Library>,
    functions: Vec<CapFunction>,
    classes: Vec<CapClass>,
}

static LOG_CALLBACK_SPAN: RwLock<Vec<Span>> = RwLock::new(Vec::new());

unsafe extern "C" fn log_callback(id: u64, msg: *const u8, msg_len: usize) {
    let data = unsafe { std::slice::from_raw_parts(msg, msg_len) };
    let log_msg = String::from_utf8_lossy(data);
    let msg = log_msg.trim_end();
    let span = LOG_CALLBACK_SPAN.read().unwrap();

    if let Some(s) = span.get(id as usize) {
        let _enter = s.enter();
        tracing::debug!("{}", msg);
    }
}

impl DynamicCapability {
    pub unsafe fn load<P: AsRef<Path>>(path: P) -> PyroductResult<Self> {
        unsafe {
            let library = Arc::new(Library::new(path.as_ref()).map_err(|e| {
                PyroductError::from_capability_linking("unknown", path.as_ref(), e.to_string())
            })?);

            let name = path
                .as_ref()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.strip_prefix("lib").unwrap_or(s).to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let capability_name = name.clone();
            let capability_span =
                tracing::span!(tracing::Level::INFO, "CAPABILITY", name = capability_name);
            let mut all_spans = LOG_CALLBACK_SPAN.write().unwrap();
            let span_id = all_spans.len() as u64;
            all_spans.push(capability_span);

            let manifest_fn: Symbol<CapabilityRegisterFn> =
                library.get(b"plugin_manifest").map_err(|e| {
                    PyroductError::from_capability_linking(
                        name.clone(),
                        path.as_ref(),
                        e.to_string(),
                    )
                })?;

            let export = manifest_fn(span_id, log_callback);

            let mut imports = Vec::new();
            for export in exports {
                let mod_name = std::str::from_utf8(std::slice::from_raw_parts(
                    export.module,
                    export.module_len,
                ))
                .unwrap_or("unknown_mod")
                .to_string();

                let func_name =
                    std::str::from_utf8(std::slice::from_raw_parts(export.name, export.name_len))
                        .unwrap_or("unknown_func")
                        .to_string();

                let func =
                    std::mem::transmute::<Function<'_>, Function<'static>>(export.func);

                imports.push((mod_name, func_name, func));
            }

            let init_fn =
                std::mem::transmute::<ClassInitFn<'_>, ClassInitFn<'static>>(export.init);
            let reset_fn =
                std::mem::transmute::<ClassResetFn<'_>, ClassResetFn<'static>>(export.reset);

            Ok(Self {
                name,
                path: path.as_ref().to_path_buf(),
                library,
                imports,
                init_fn,
                reset_fn,
                destroy_fn: export.drop,
            })
        }
    }
}

impl Capability for DynamicCapability {
    fn init(&self, config: &Option<&serde_json::Value>) -> PyroductResult<CapabilityInit<'static>> {
        let (config_ptr, config_len, config_bytes) = match config {
            Some(value) => {
                let config_bytes = serde_json::to_vec(value).expect("AARRGG");
                (config_bytes.as_ptr(), config_bytes.len(), config_bytes)
            }
            None => (ptr::null(), 0, Vec::new()),
        };

        let capability_init = match self.init_fn {
            ClassInitFn::Sync(func) => {
                let res = unsafe { func(config_ptr, config_len) };
                let state = unsafe {
                    InitResultBridge::from_ffi(res, self.name.clone(), self.path.clone())?
                };
                CapabilityInit::Sync {
                    state: Some(state),
                    destroy_fn: self.destroy_fn,
                }
            }
            ClassInitFn::Async(func) => {
                let fut_res = unsafe { func(config_ptr, config_len) };
                let future = AsyncInitFuture::new(fut_res, self.name.clone(), self.path.clone());
                let future: AsyncInitFuture<'static> = unsafe { std::mem::transmute(future) };

                CapabilityInit::Async {
                    config_bytes,
                    future,
                    destroy_fn: self.destroy_fn,
                }
            }
            ClassInitFn::Null => CapabilityInit::Null,
        };

        Ok(capability_init)
    }

    fn imports(&self) -> Vec<(String, String, Function<'static>)> {
        unsafe { std::mem::transmute(self.imports.clone()) }
    }

    fn reset(&self, state: &mut StatePtr) -> CapabilityReset<'static> {
        match self.reset_fn {
            ClassResetFn::Sync(func) => {
                let res = unsafe { func(state.ptr) };
                CapabilityReset::SyncOrNull(Some(unsafe {
                    ExecutionResultBridge::expected_null_from_ffi(
                        res,
                        self.name.clone(),
                        self.path.clone(),
                    )
                }))
            }
            ClassResetFn::Async(func) => {
                let fut = unsafe { func(state.ptr) };
                let future = AsyncExecFuture::new(fut, self.name.clone(), self.path.clone());
                let future: AsyncExecFuture<'static> = unsafe { std::mem::transmute(future) };
                CapabilityReset::Async(future)
            }
            ClassResetFn::Null => CapabilityReset::SyncOrNull(Some(Ok(()))),
        }
    }

    fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}
