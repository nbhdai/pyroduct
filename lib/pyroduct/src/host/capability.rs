use futures::future::try_join_all;
use libloading::{Library, Symbol};
use tracing::Span;
use wasmtime::Linker;

use crate::{CapIdentity, ModIdentity, PyroductResult};
use crate::capability_host::ffi::{CapabilityRegisterFn, ClassExport, FunctionExport};
use crate::errors::PyroductError;
use crate::host::class::{CapClass, ClassState};
use crate::host::function::CapFunction;
use crate::host::harness::HarnessState;
use std::ffi::c_void;
use std::path::Path;
use std::ptr;
use std::sync::{Arc, Mutex, RwLock};

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

// --- Type definitions ---
pub type WasmArgs = (i32, i32, i32, i32); // (wasm_state_ptr, wasm_state_len, ptr, len)

pub struct Capability {
    _library: Library,
    functions: Vec<CapFunction>,
    classes: Vec<CapClass>,
    ident: CapIdentity,
    span_id: usize,
}

/// Holds the state for a loaded capability, which may contain multiple classes.
pub struct CapabilityState {
    pub ident: CapIdentity,
    /// States for each class in the capability.
    /// Index corresponds to the index in `DynamicCapability.classes`.
    pub classes: Vec<ClassState>,
}

unsafe impl Send for CapabilityState {}

impl CapabilityState {
    pub fn get_class_ptr(&self, index: usize) -> *mut c_void {
        self.classes
            .get(index)
            .map(|s| s.ptr)
            .unwrap_or(ptr::null_mut())
    }

    pub async fn reset(&mut self) -> PyroductResult<()> {
        let futures = self.classes.iter_mut()
            .map(|state| state.reset(&self.ident));

        try_join_all(futures).await?;
        Ok(())
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct CapabilityConfig {
    class_config: Vec<Option<serde_json::Value>>,
}

impl Capability {
    pub unsafe fn load<P: AsRef<Path>>(path: P) -> PyroductResult<Self> {
        let ident = CapIdentity {
            path: path.as_ref().into()
        };
            let library = unsafe { Library::new(path.as_ref()) }.map_err(|e| {
                PyroductError::from_capability_linking(&ident, e.to_string())
            })?;



            let capability_span =
                tracing::span!(tracing::Level::INFO, "CAPABILITY", name = &ident.name());
            let mut all_spans = LOG_CALLBACK_SPAN.write().unwrap();
            let span_id = all_spans.len();
            all_spans.push(capability_span);

            let manifest_fn: Symbol<CapabilityRegisterFn> =
                unsafe { library.get(b"plugin_manifest") }.map_err(|e| {
                    PyroductError::from_capability_linking(
                        &ident,
                        e.to_string(),
                    )
                })?;

            let export = unsafe { manifest_fn(span_id as u64, log_callback) };
            let functions: &[FunctionExport<'static>] = unsafe {
                std::slice::from_raw_parts(export.functions, export.len_functions)
            };

            let classes: &[ClassExport<'static>] = unsafe {
                std::slice::from_raw_parts(export.classes, export.len_classes)
            };

            let functions = functions.iter().map(|f| CapFunction::new(&ident, f)).collect::<Vec<_>>();
            let classes = classes.iter().map(|c| CapClass::new(&ident, c)).collect::<Vec<_>>();

            Ok(Self {
                ident,
                _library: library,
                functions,
                classes,
                span_id,
            })
    }

    async fn init_set(&self, config: &CapabilityConfig) -> PyroductResult<CapabilityState> {
        let init_futures: Vec<_> = self.classes
            .iter()
            .zip(config.class_config.iter())
            .map(|(class, config)| class.init(config.as_ref()))
            .collect::<PyroductResult<_>>()?;

        let classes = try_join_all(init_futures).await?;

        Ok(CapabilityState {
            ident: self.ident.clone(),
            classes,
        })
    }

    async fn init_default(&self) -> PyroductResult<CapabilityState> {
        let init_futures: Vec<_> = self.classes
            .iter()
            .map(|class| class.init(None))
            .collect::<PyroductResult<_>>()?;

        let classes = try_join_all(init_futures).await?;

        Ok(CapabilityState {
            ident: self.ident.clone(),
            classes,
        })
    }

    pub async fn init(&self, config: Option<&CapabilityConfig>) -> PyroductResult<CapabilityState> {
        match config {
            Some(config) => self.init_set(config).await,
            None => self.init_default().await,
        }
    }

    pub fn link(&self, linker: &mut Linker<HarnessState>) -> PyroductResult<()> {
        for func in self.functions.iter() {
            func.link(linker, self.span_id)?;
        }
        for class in self.classes.iter() {
            class.link(linker, self.span_id)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Capabilities {
    pub capabilities: Vec<Arc<Capability>>,
}

impl Capabilities {
    pub fn load<'a>(paths: impl Iterator<Item = &'a Path>) -> PyroductResult<Self> {
        let capabilities = paths.map(|p| unsafe { Capability::load(p) }.map(|c| Arc::new(c))).collect::<PyroductResult<Vec<_>>>()?;
        Ok(Self {
            capabilities
        })
    }

    pub fn link(&self, linker: &mut Linker<HarnessState>) -> PyroductResult<()> {
        for cap in self.capabilities.iter() {
            cap.link(linker)?;
        }
        Ok(())
    }

    pub async fn init<'a>(&self, module: &ModIdentity, configs: impl Iterator<Item = Option<&'a CapabilityConfig>>) -> PyroductResult<HarnessState> {
        let inits: Vec<_> = self.capabilities.iter().zip(configs).map(|(c, config)| c.init(config)).collect();

        let states = try_join_all(inits).await?;
        Ok(HarnessState {
            module: module.clone(),
            cap_states: states,
            error_slot: Mutex::new(None),
            capabilities: self.clone(),
        })
    }
}
