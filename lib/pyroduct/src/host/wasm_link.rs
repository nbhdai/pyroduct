use libloading::{Library, Symbol};
use pin_project::pin_project;
use std::ffi::c_void;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::{Arc, RwLock};
use tracing::{Span, error, info};
use wasmtime::{Caller, Linker};

use crate::capability_host::ffi::{
    AsyncPluginProcessFn, PluginDropFn, PluginFunction, PluginInitFn, PluginRegisterFn,
    PluginResetFn, SyncPluginProcessFn,
};
use crate::errors::{FfiError, PyroductError};
use crate::host::ffi_bridge::{
    AsyncExecFuture, AsyncInitFuture, ExecutionResultBridge, InitResultBridge,
};

// --- Type definitions for clarity ---
type WasmArgs = (i32, i32, i32, i32); // (wasm_state_ptr, wasm_state_len, ptr, len)

pub struct StatePtr {
    ptr: *mut c_void,
    destroy_fn: PluginDropFn,
}

unsafe impl Send for StatePtr {}

impl Drop for StatePtr {
    fn drop(&mut self) {
        match self.destroy_fn {
            PluginDropFn::Sync(destroy_fn) => {
                if !self.ptr.is_null() {
                    unsafe { (destroy_fn)(self.ptr) }
                } else {
                    tracing::error!("Drop function exists, but state pointer is null");
                }
            }
            PluginDropFn::Null => {
                if !self.ptr.is_null() {
                    tracing::error!("Drop function does not exist, and pointer is non-null");
                }
            }
        }
    }
}

pub struct HarnessState {
    // Map capability index -> Opaque State Pointer
    cap_states: Vec<StatePtr>,
    /// Shared slot for an error that occurred during a host function call
    pub error_slot: Option<PyroductError>,
}

pub struct Capabilities {
    wasm_name: String,
    wasm_path: PathBuf,
    caps: Vec<Arc<dyn Capability>>,
}

type PyroductResult<T> = Result<T, PyroductError>;

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

trait CapabilityExt: Capability {
    /// Handles the "Preparation" phase: getting host state, validating memory bounds,
    /// and calculating raw pointers.
    fn prepare_io(
        &self,
        caller: &mut Caller<'_, HarnessState>,
        cap_index: usize,
        args: WasmArgs,
        wasm_name: &str,
        wasm_path: &Path,
    ) -> PyroductResult<(*mut c_void, *mut u8, *const u8)> {
        let (wasm_state_ptr, wasm_state_len, ptr, len) = args;

        let host_state_ptr = caller
            .data()
            .cap_states
            .get(cap_index)
            .map(|s| s.ptr)
            .unwrap_or(std::ptr::null_mut());

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

        // Calculate pointers
        // SAFETY: Bounds checked above.
        let input_ptr = mem_slice.as_ptr().wrapping_add(ptr as usize);
        let w_state_ptr = mem_slice.as_mut_ptr().wrapping_add(wasm_state_ptr as usize);

        Ok((host_state_ptr, w_state_ptr, input_ptr))
    }

    /// Handles the "Finalization" phase: allocating memory in WASM and writing the result.
    async fn finalize_io(
        &self,
        mut caller: &mut Caller<'_, HarnessState>,
        output_vec: Vec<u8>,
        wasm_name: &str,
        wasm_path: &Path,
    ) -> PyroductResult<i32> {
        let total_len = 4 + output_vec.len();

        // Get alloc function
        let alloc = caller
            .get_export("alloc")
            .ok_or_else(|| {
                PyroductError::from_module_linking(
                    wasm_name.to_string(),
                    wasm_path.to_path_buf(),
                    "Missing alloc",
                )
            })?
            .into_func()
            .ok_or_else(|| {
                PyroductError::from_module_linking(
                    wasm_name.to_string(),
                    wasm_path.to_path_buf(),
                    "Alloc is not a function",
                )
            })?;

        let alloc_typed = alloc.typed::<i32, i32>(&mut caller).map_err(|err| {
            PyroductError::from_module_linking(
                wasm_name.to_string(),
                wasm_path.to_path_buf(),
                format!("Alloc has incorrect function signature: {err}"),
            )
        })?;

        // Call alloc (async)
        let result_ptr = alloc_typed
            .call_async(&mut caller, total_len as i32)
            .await
            .map_err(|err| {
                PyroductError::from_module_linking(
                    wasm_name.to_string(),
                    wasm_path.to_path_buf(),
                    format!("Allocation failed: {err}"),
                )
            })?;

        // Re-acquire memory (invalidated by call_async)
        let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
        let (mem_slice, _) = memory.data_and_store_mut(&mut caller);

        // Write result
        unsafe {
            let dest_ptr = mem_slice.as_mut_ptr().add(result_ptr as usize);
            *(dest_ptr as *mut u32) = output_vec.len() as u32;
            std::ptr::copy_nonoverlapping(output_vec.as_ptr(), dest_ptr.add(4), output_vec.len());
        }

        Ok(result_ptr)
    }

    /// Executes a Sync capability call
    async fn process_sync_call(
        &self,
        mut caller: Caller<'_, HarnessState>,
        raw_fn: SyncPluginProcessFn,
        args: WasmArgs,
        cap_index: usize,
        wasm_name: String,
        wasm_path: PathBuf,
    ) -> i32 {
        let (_, wasm_state_len, _, input_len) = args;

        // 1. Prepare Pointers
        let (host_state_ptr, w_state_ptr, input_ptr) =
            match self.prepare_io(&mut caller, cap_index, args, &wasm_name, &wasm_path) {
                Ok(ptrs) => ptrs,
                Err(e) => {
                    caller.data_mut().error_slot = Some(e);
                    return 0;
                }
            };

        // 2. Execute unsafe FFI
        info!("Entering unsafe plugin function...");
        let result = unsafe {
            raw_fn(
                w_state_ptr,
                wasm_state_len as usize,
                input_ptr,
                input_len as usize,
                host_state_ptr,
            )
        };
        info!("Exited unsafe plugin function.");

        let output_vec = match unsafe {
            ExecutionResultBridge::from_ffi(result, self.name(), self.path().unwrap().to_path_buf())
        } {
            Ok(v) => v,
            Err(e) => {
                caller.data_mut().error_slot = Some(e);
                return 0;
            }
        };

        // 3. Finalize (Alloc & Write)
        match self
            .finalize_io(&mut caller, output_vec, &wasm_name, &wasm_path)
            .await
        {
            Ok(ptr) => ptr,
            Err(e) => {
                caller.data_mut().error_slot = Some(e);
                0
            }
        }
    }

    /// Executes an Async capability call
    async fn process_async_call(
        &self,
        mut caller: Caller<'_, HarnessState>,
        raw_fn: AsyncPluginProcessFn<'static>,
        args: WasmArgs,
        cap_index: usize,
        wasm_name: String,
        wasm_path: PathBuf,
    ) -> i32 {
        let (_, wasm_state_len, _, input_len) = args;

        // 1. Prepare Pointers
        let (host_state_ptr, w_state_ptr, input_ptr) =
            match self.prepare_io(&mut caller, cap_index, args, &wasm_name, &wasm_path) {
                Ok(ptrs) => ptrs,
                Err(e) => {
                    caller.data_mut().error_slot = Some(e);
                    return 0;
                }
            };

        // 2. Execute unsafe FFI
        info!("Entering unsafe async plugin function...");
        let fut = unsafe {
            raw_fn(
                w_state_ptr,
                wasm_state_len as usize,
                input_ptr,
                input_len as usize,
                host_state_ptr,
            )
        };

        let exec_fut = AsyncExecFuture::new(fut, self.name(), self.path().unwrap().to_path_buf());
        let output_vec = match exec_fut.await {
            Ok(v) => v,
            Err(e) => {
                info!("Exited unsafe async plugin function (Error).");
                caller.data_mut().error_slot = Some(e);
                return 0;
            }
        };
        info!("Exited unsafe async plugin function (Success).");

        // 3. Finalize (Alloc & Write)
        match self
            .finalize_io(&mut caller, output_vec, &wasm_name, &wasm_path)
            .await
        {
            Ok(ptr) => ptr,
            Err(e) => {
                caller.data_mut().error_slot = Some(e);
                0
            }
        }
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
                    PluginFunction::Sync(raw_fn) => {
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
                    PluginFunction::Async(raw_fn) => {
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

    pub async fn reset_states(&self, states: &mut HarnessState) -> PyroductResult<()> {
        for (i, cap) in self.caps.iter().enumerate() {
            if let Some(state) = states.cap_states.get_mut(i) {
                cap.reset(state).await?;
            }
        }
        Ok(())
    }
}

#[pin_project(project = CapInit)]
pub enum CapabilityInit<'a> {
    Sync {
        state: Option<*mut c_void>,
        destroy_fn: PluginDropFn,
    },
    Async {
        config_bytes: Vec<u8>,
        #[pin]
        future: AsyncInitFuture<'a>,
        destroy_fn: PluginDropFn,
    },
    Null,
}

impl<'a> Future for CapabilityInit<'a> {
    type Output = PyroductResult<StatePtr>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            CapInit::Sync { state, destroy_fn } => match state.take() {
                Some(state) => std::task::Poll::Ready(Ok(StatePtr {
                    ptr: state,
                    destroy_fn: *destroy_fn,
                })),
                None => panic!("Double await!"),
            },
            CapInit::Async {
                config_bytes: _,
                future,
                destroy_fn,
            } => match future.poll(cx) {
                std::task::Poll::Ready(result) => match result {
                    Ok(pointer) => std::task::Poll::Ready(Ok(StatePtr {
                        ptr: pointer,
                        destroy_fn: *destroy_fn,
                    })),
                    Err(e) => std::task::Poll::Ready(Err(e)),
                },
                std::task::Poll::Pending => std::task::Poll::Pending,
            },
            CapInit::Null => std::task::Poll::Ready(Ok(StatePtr {
                ptr: ptr::null_mut(),
                destroy_fn: PluginDropFn::Null,
            })),
        }
    }
}

#[pin_project(project = CapReset)]
pub enum CapabilityReset<'a> {
    Async(#[pin] AsyncExecFuture<'a>),
    SyncOrNull(Option<PyroductResult<()>>),
}

impl<'a> Future for CapabilityReset<'a> {
    type Output = PyroductResult<()>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            CapReset::Async(this) => match this.poll(cx) {
                std::task::Poll::Ready(Ok(_)) => std::task::Poll::Ready(Ok(())),
                std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
                std::task::Poll::Pending => std::task::Poll::Pending,
            },
            CapReset::SyncOrNull(result) => {
                match result.take() {
                    Some(result) => std::task::Poll::Ready(result),
                    None => std::task::Poll::Ready(Err(FfiError::FuturePolledAfterCompletion
                        .to_capability_error("unknown", "unknown"))),
                }
            }
        }
    }
}

pub trait Capability: Send + Sync {
    fn init(&self, config: &Option<&serde_json::Value>) -> PyroductResult<CapabilityInit<'static>>;
    fn imports(&self) -> Vec<(String, String, PluginFunction<'static>)>;
    fn reset(&self, state: &mut StatePtr) -> CapabilityReset<'static>;

    fn path(&self) -> Option<&Path>;
    fn name(&self) -> String;
}

pub struct DynamicCapability {
    name: String,
    path: PathBuf,
    #[allow(dead_code)]
    library: Arc<Library>,
    imports: Vec<(String, String, PluginFunction<'static>)>,
    init_fn: PluginInitFn<'static>,
    reset_fn: PluginResetFn<'static>,
    destroy_fn: PluginDropFn,
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

            let manifest_fn: Symbol<PluginRegisterFn> =
                library.get(b"plugin_manifest").map_err(|e| {
                    PyroductError::from_capability_linking(
                        name.clone(),
                        path.as_ref(),
                        e.to_string(),
                    )
                })?;

            let export = manifest_fn(span_id, log_callback);
            let exports = Vec::from_raw_parts(export.ptr, export.len, export.cap);

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
                    std::mem::transmute::<PluginFunction<'_>, PluginFunction<'static>>(export.func);

                imports.push((mod_name, func_name, func));
            }

            let init_fn =
                std::mem::transmute::<PluginInitFn<'_>, PluginInitFn<'static>>(export.init);
            let reset_fn =
                std::mem::transmute::<PluginResetFn<'_>, PluginResetFn<'static>>(export.reset);

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
            PluginInitFn::Sync(func) => {
                let res = unsafe { func(config_ptr, config_len) };
                let state = unsafe {
                    InitResultBridge::from_ffi(res, self.name.clone(), self.path.clone())?
                };
                CapabilityInit::Sync {
                    state: Some(state),
                    destroy_fn: self.destroy_fn,
                }
            }
            PluginInitFn::Async(func) => {
                let fut_res = unsafe { func(config_ptr, config_len) };
                let future = AsyncInitFuture::new(fut_res, self.name.clone(), self.path.clone());
                let future: AsyncInitFuture<'static> = unsafe { std::mem::transmute(future) };

                CapabilityInit::Async {
                    config_bytes,
                    future,
                    destroy_fn: self.destroy_fn,
                }
            }
            PluginInitFn::Null => CapabilityInit::Null,
        };

        Ok(capability_init)
    }

    fn imports(&self) -> Vec<(String, String, PluginFunction<'static>)> {
        unsafe { std::mem::transmute(self.imports.clone()) }
    }

    fn reset(&self, state: &mut StatePtr) -> CapabilityReset<'static> {
        match self.reset_fn {
            PluginResetFn::Sync(func) => {
                let res = unsafe { func(state.ptr) };
                CapabilityReset::SyncOrNull(Some(unsafe {
                    ExecutionResultBridge::expected_null_from_ffi(
                        res,
                        self.name.clone(),
                        self.path.clone(),
                    )
                }))
            }
            PluginResetFn::Async(func) => {
                let fut = unsafe { func(state.ptr) };
                let future = AsyncExecFuture::new(fut, self.name.clone(), self.path.clone());
                let future: AsyncExecFuture<'static> = unsafe { std::mem::transmute(future) };
                CapabilityReset::Async(future)
            }
            PluginResetFn::Null => CapabilityReset::SyncOrNull(Some(Ok(()))),
        }
    }

    fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}
