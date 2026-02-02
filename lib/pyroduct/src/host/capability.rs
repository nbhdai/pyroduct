use futures::future::try_join_all;
use libloading::{Library, Symbol};
use tracing::Span;
use wasmtime::Linker;

use crate::capability_host::ffi::CapabilityRegisterFn;
use crate::errors::PyroductError;
use crate::host::class::{CapabilityInit, CapClass};
use crate::host::pipeline::CapabilityDef;
use crate::host::wasm_bridge::HarnessState;
use crate::{CapIdentity, ModIdentity, PyroductResult};
use std::collections::HashMap;
use std::path::Path;
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
    class: CapClass,
    span_id: usize,
}

impl Capability {
    pub unsafe fn load<P: AsRef<Path>>(path: P) -> PyroductResult<Self> {
        let ident = CapIdentity {
            path: path.as_ref().into(),
        };
        let library = unsafe { Library::new(path.as_ref()) }
            .map_err(|e| PyroductError::from_capability_loading(&ident, e.to_string()))?;

        let capability_span =
            tracing::span!(tracing::Level::INFO, "CAPABILITY", name = &ident.name());
        let mut all_spans = LOG_CALLBACK_SPAN.write().unwrap();
        let span_id = all_spans.len();
        all_spans.push(capability_span);

        let manifest_fn: Symbol<CapabilityRegisterFn> =
            unsafe { library.get(b"capability_manifest") }.map_err(|e| {
                PyroductError::from_capability_loading(
                    &ident,
                    format!("Unable to get the manifest symbol: {e}"),
                )
            })?;

        let export = unsafe { manifest_fn(span_id as u64, log_callback) };

        let class = CapClass::new(&ident, &export)?;

        Ok(Self {
            _library: library,
            class,
            span_id,
        })
    }

    pub fn init(
        &self,
        config: Option<&serde_json::Value>,
    ) -> PyroductResult<CapabilityInit<'static>> {
        self.class.init(config)
    }

    pub fn link(&self, linker: &mut Linker<HarnessState>) -> PyroductResult<()> {
        self.class.link(linker, self.span_id)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Capabilities {
    pub capabilities: HashMap<String, Arc<Capability>>,
}

impl Capabilities {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    pub fn load(&mut self, name: &str, path: &Path) -> PyroductResult<()> {
        let cap = unsafe { Arc::new(Capability::load(path)?) };
        self.capabilities.insert(name.to_string(), cap);
        Ok(())
    }

    pub fn load_many<'a>(
        &mut self,
        names: impl Iterator<Item = &'a str>,
        paths: impl Iterator<Item = &'a Path>,
    ) -> PyroductResult<()> {
        let capabilities = names
            .zip(paths)
            .map(|(n, p)| {
                let cap = unsafe { Arc::new(Capability::load(p)?) };
                Ok((n.to_string(), cap))
            })
            .collect::<PyroductResult<HashMap<_, _>>>()?;
        self.capabilities.extend(capabilities);
        Ok(())
    }

    pub fn link<'a>(
        &self,
        names: impl Iterator<Item = &'a str>,
        linker: &mut Linker<HarnessState>,
    ) -> PyroductResult<()> {
        for name in names {
            let cap = self
                .capabilities
                .get(name)
                .ok_or(PyroductError::missing_cap(name))?;
            cap.link(linker)?;
        }
        Ok(())
    }

    pub async fn init<'a>(
        &self,
        module: &ModIdentity,
        configs: &[CapabilityDef],
    ) -> PyroductResult<HarnessState> {
        let mut inits = Vec::new();
        let mut capabilities = Vec::new();

        for config in configs {
            let cap = self
                .capabilities
                .get(&config.name)
                .ok_or(PyroductError::missing_cap(&config.name))?;
            inits.push(cap.init(config.config.as_ref())?);
            capabilities.push((config.name.clone(), cap.clone()));
        }

        let states = try_join_all(inits).await?;
        Ok(HarnessState {
            module: module.clone(),
            cap_states: states,
            error_slot: Mutex::new(None),
            capabilities,
        })
    }
}
