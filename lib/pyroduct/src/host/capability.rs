use futures::future::try_join_all;
use libloading::{Library, Symbol};
use tracing::Span;
use wasmtime::Linker;

use crate::capability_host::ffi::CapabilityRegisterFn;
use crate::errors::PyroductError;
use crate::host::harness::HarnessState;
use crate::{CapIdentity, ModIdentity, PyroductResult};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

mod class;
pub use class::{CapClass, ClassState};

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
    class: CapClass,
    span_id: usize,
}

impl Capability {
    pub unsafe fn load<P: AsRef<Path>>(path: P) -> PyroductResult<Self> {
        let ident = CapIdentity {
            path: path.as_ref().into(),
        };
        let library = unsafe { Library::new(path.as_ref()) }
            .map_err(|e| PyroductError::from_capability_linking(&ident, e.to_string()))?;

        let capability_span =
            tracing::span!(tracing::Level::INFO, "CAPABILITY", name = &ident.name());
        let mut all_spans = LOG_CALLBACK_SPAN.write().unwrap();
        let span_id = all_spans.len();
        all_spans.push(capability_span);

        let manifest_fn: Symbol<CapabilityRegisterFn> = unsafe { library.get(b"plugin_manifest") }
            .map_err(|e| PyroductError::from_capability_linking(&ident, e.to_string()))?;

        let export = unsafe { manifest_fn(span_id as u64, log_callback) };

        let class = CapClass::new(&ident, &export);

        Ok(Self {
            _library: library,
            class,
            span_id,
        })
    }

    pub async fn init(&self, config: Option<&serde_json::Value>) -> PyroductResult<ClassState> {
        let class = self.class.init(config)?.await?;

        Ok(class)
    }

    pub fn link(&self, linker: &mut Linker<HarnessState>) -> PyroductResult<()> {
        self.class.link(linker, self.span_id)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Capabilities {
    pub capabilities: Vec<Arc<Capability>>,
}

impl Capabilities {
    pub fn load<'a>(paths: impl Iterator<Item = &'a Path>) -> PyroductResult<Self> {
        let capabilities = paths
            .map(|p| unsafe { Capability::load(p) }.map(|c| Arc::new(c)))
            .collect::<PyroductResult<Vec<_>>>()?;
        Ok(Self { capabilities })
    }

    pub fn link(&self, linker: &mut Linker<HarnessState>) -> PyroductResult<()> {
        for cap in self.capabilities.iter() {
            cap.link(linker)?;
        }
        Ok(())
    }

    pub async fn init<'a>(
        &self,
        module: &ModIdentity,
        configs: impl Iterator<Item = Option<&'a serde_json::Value>>,
    ) -> PyroductResult<HarnessState> {
        let inits: Vec<_> = self
            .capabilities
            .iter()
            .zip(configs)
            .map(|(c, config)| c.init(config))
            .collect();

        let states = try_join_all(inits).await?;
        Ok(HarnessState {
            module: module.clone(),
            cap_states: states,
            error_slot: Mutex::new(None),
            capabilities: self.clone(),
        })
    }
}
