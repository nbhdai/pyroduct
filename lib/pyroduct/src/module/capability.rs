use axum::async_trait;
use dashmap::DashMap;
use indexmap::IndexMap;
use pyro_artifacts::artifacts::CapabilityConfig;
use pyro_artifacts::cargo::CapabilityIdent;
use std::path::Path;
use std::sync::Weak;
use std::sync::{
    Arc, LazyLock,
    atomic::{AtomicI64, AtomicU64, Ordering},
};
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::mpsc;
use wasmtime::{FuncType, Linker, Val, ValType};

use libloading::{Library, Symbol};
use object::{Object, ObjectSymbol, SymbolKind};
use thiserror::Error;

use crate::ffi::host::{CapabilityClass, CapabilityObject};
use crate::ffi::{CapabilityRegisterFn, ClassExport};
use crate::format::header::PyroData;
use crate::format::{
    PyroVec,
    format::{PyroFormat, Writer},
    json::Json,
};
use crate::module::call::PyroCallIo;
use crate::module::{PyroState, WasmError};
use pyro_spec::InterfaceSpec;

// =============================================================================
// Error
// =============================================================================

#[derive(Error, Debug)]
pub enum CapabilityError {
    #[error("Failed to load library '{path}': {reason}")]
    LibraryOpen { path: String, reason: String },

    #[error("Failed to read library file '{path}': {reason}")]
    FileRead { path: String, reason: String },

    #[error("Failed to parse library binary '{path}': {reason}")]
    BinaryParse { path: String, reason: String },

    #[error("Failed to register capability '{symbol}' from '{path}': {reason}")]
    Registration {
        path: String,
        symbol: String,
        reason: String,
    },

    #[error("No symbols with the 'pyro_' prefix found in '{path}'")]
    NoCapabilitiesFound { path: String },

    // --- Configuration Stage Errors ---
    #[error("Configuration provided for unknown capability '{name}'")]
    CapabilityNotFound { name: String },

    #[error("Failed to serialize configuration for '{class}': {reason}")]
    ConfigSerialization { class: String, reason: String },

    #[error("Failed to instantiate capability '{class}': {reason}")]
    Instantiation { class: String, reason: String },
}

// =============================================================================
// Logging
// =============================================================================

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub library_id: i64,
    pub object_id: u64,
    pub mux_id: u32,
    pub message: String,
    pub timestamp: std::time::SystemTime,
}

static CATCH_LOG_SENDER: LazyLock<DashMap<i64, mpsc::Sender<LogEntry>>> =
    LazyLock::new(DashMap::new);
static LOG_SENDERS: LazyLock<DashMap<(i64, u64), mpsc::Sender<LogEntry>>> =
    LazyLock::new(DashMap::new);
static NEXT_LIB_ID: AtomicI64 = AtomicI64::new(1);
static NEXT_OBJECT_ID: AtomicU64 = AtomicU64::new(1);

/// Create a new log channel. Returns the numeric ID (to pass to the C side)
/// and a receiver you can poll to build up your logs.
///
/// `buffer` controls how many messages can queue before back-pressure kicks in.
pub fn create_log(library_id: i64, span_id: u64, buffer: usize) -> mpsc::Receiver<LogEntry> {
    let (tx, rx) = mpsc::channel(buffer);
    LOG_SENDERS.insert((library_id, span_id), tx);
    rx
}

/// Tear down a log channel. Drops the sender so the receiver will see the
/// channel close on next poll.
pub fn destroy_log(library_id: i64, span_id: u64) {
    LOG_SENDERS.remove(&(library_id, span_id));
}

/// # Safety
/// `msg` must point to `msg_len` valid bytes for the duration of this call.
pub unsafe extern "C" fn log_callback(
    library_id: i64,
    span_id: u64,
    mux_id: u32,
    msg: *const u8,
    msg_len: usize,
) {
    let data = unsafe { std::slice::from_raw_parts(msg, msg_len) };
    let log_msg = String::from_utf8_lossy(data).trim_end().to_string();
    let entry = LogEntry {
        library_id,
        object_id: span_id,
        mux_id,
        message: log_msg,
        timestamp: std::time::SystemTime::now(),
    };
    tracing::trace!(
        library_id,
        span_id,
        mux_id,
        msg = entry.message,
        "[CAPABILITY]"
    );
    if span_id == 0 {
        if let Some(tx) = CATCH_LOG_SENDER.get(&library_id) {
            match tx.try_send(entry) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    eprintln!(
                        "[log] channel full for id=({library_id},{span_id}), dropping message"
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {}
            }
        } else {
            tracing::debug!(log_msg = entry.message, "Uncaught Capability Log");
        }
    } else {
        if let Some(tx) = LOG_SENDERS.get(&(library_id, span_id)) {
            match tx.try_send(entry) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    eprintln!(
                        "[log] channel full for id=({library_id},{span_id}), dropping message"
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    LOG_SENDERS.remove(&(library_id, span_id));
                }
            }
        } else {
            tracing::debug!(log_msg = entry.message, "Uncaught Capability Log");
        }
    }
}

// =============================================================================
// Symbol scanning
// =============================================================================

pub struct ScannedSymbol {
    pub name: String,
    pub address: u64,
}

pub fn scan_pyro_symbols(path: &Path) -> Result<Vec<ScannedSymbol>, CapabilityError> {
    let path_str = path.display().to_string();
    tracing::debug!(path = %path_str, "Scanning library for pyro symbols");

    let bin_data = std::fs::read(path).map_err(|e| CapabilityError::FileRead {
        path: path_str.clone(),
        reason: e.to_string(),
    })?;

    let file = object::File::parse(&*bin_data).map_err(|e| CapabilityError::BinaryParse {
        path: path_str.clone(),
        reason: e.to_string(),
    })?;

    let mut symbols = Vec::new();

    for symbol in file.symbols() {
        if symbol.kind() != SymbolKind::Text || !symbol.is_global() || symbol.is_undefined() {
            continue;
        }
        let name = match symbol.name() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let clean = name.strip_prefix('_').unwrap_or(name);

        if clean.starts_with("pyro_") {
            symbols.push(ScannedSymbol {
                name: clean.to_string(),
                address: symbol.address(),
            });
        }
    }

    tracing::debug!(
        path = %path_str,
        count = symbols.len(),
        "Found {} pyro symbols",
        symbols.len()
    );

    Ok(symbols)
}

// =============================================================================
// Library loader
// =============================================================================

static LOADED_LIBRARIES: LazyLock<Mutex<HashMap<CapabilityIdent, Weak<CapabilityLibrary>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct CapabilityLibrary {
    pub id: i64,
    pub ident: CapabilityIdent,
    pub capabilities: IndexMap<String, Arc<CapabilityClass>>,
    pub interface: InterfaceSpec<'static>,
}

impl CapabilityLibrary {
    pub fn load(ident: CapabilityIdent, path: &Path) -> Result<Arc<Self>, CapabilityError> {
        tracing::info!(ident = ?ident, path = %path.display(), "Loading capability library");
        let mut libraries = LOADED_LIBRARIES.lock().unwrap_or_else(|e| {
            tracing::error!(
                "LOADED_LIBRARIES mutex was poisoned (likely a panic in another thread)"
            );
            e.into_inner()
        });
        if let Some(lib) = libraries.get(&ident).and_then(|w| w.upgrade()) {
            tracing::debug!(ident = ?ident, "Capability library found in cache");
            Ok(lib)
        } else {
            let lib = Arc::new(Self::load_inter(&ident, path)?);
            libraries.insert(ident, Arc::downgrade(&lib));
            tracing::info!(ident = ?lib.ident, id = lib.id, "Successfully loaded and cached capability library");
            Ok(lib)
        }
    }

    fn load_inter(ident: &CapabilityIdent, path: &Path) -> Result<Self, CapabilityError> {
        let path_str = path.display().to_string();
        tracing::debug!(ident = ?ident, path = %path_str, "Loading library from disk");

        // 1. Scan
        let pyro_symbols = scan_pyro_symbols(path)?;

        if pyro_symbols.is_empty() {
            tracing::warn!(path = %path_str, "No capabilities found in scanned library");
            return Err(CapabilityError::NoCapabilitiesFound { path: path_str });
        }

        // 2. Load Interface Spec
        let interface_path = path
            .parent()
            .unwrap_or(Path::new("."))
            .join("interface.json");
        tracing::debug!(interface_path = %interface_path.display(), "Reading interface specification");
        let interface_data =
            std::fs::read(&interface_path).map_err(|e| CapabilityError::FileRead {
                path: interface_path.display().to_string(),
                reason: e.to_string(),
            })?;
        let interface: InterfaceSpec<'static> =
            serde_json::from_slice(&interface_data).map_err(|e| CapabilityError::BinaryParse {
                path: interface_path.display().to_string(),
                reason: e.to_string(),
            })?;

        // 3. Load Library
        tracing::debug!(path = %path_str, "Loading native shared library");
        let library =
            Arc::new(
                unsafe { Library::new(path) }.map_err(|e| CapabilityError::LibraryOpen {
                    path: path_str.clone(),
                    reason: e.to_string(),
                })?,
            );
        let id = NEXT_LIB_ID.fetch_add(1, Ordering::SeqCst);
        tracing::debug!(path = %path_str, id, "Assigned library ID");

        // 4. Register
        let mut capabilities = IndexMap::with_capacity(pyro_symbols.len());
        for sym in &pyro_symbols {
            let sym_cstr = format!("{}\0", sym.name);

            let register_fn: Symbol<CapabilityRegisterFn> =
                match unsafe { library.get(sym_cstr.as_bytes()) } {
                    Ok(f) => f,
                    Err(e) => {
                        tracing::warn!(
                            "Symbol '{}' found in binary but could not be loaded: {}",
                            sym.name,
                            e
                        );
                        continue;
                    }
                };

            let export: ClassExport = unsafe { register_fn(id, log_callback) };

            let class =
                unsafe { CapabilityClass::from_export(ident.clone(), library.clone(), export) }
                    .map_err(|e| CapabilityError::Registration {
                        path: path_str.clone(),
                        symbol: sym.name.clone(),
                        reason: e.to_string(),
                    })?;
            tracing::debug!("Loaded capability class {}", class.name());
            capabilities.insert(class.name().to_string(), Arc::new(class));
        }

        if capabilities.is_empty() {
            tracing::warn!(path = %path_str, "Zero capability classes successfully registered");
            return Err(CapabilityError::NoCapabilitiesFound { path: path_str });
        }

        Ok(Self {
            id,
            ident: ident.clone(),
            capabilities,
            interface,
        })
    }

    /// Iterates through the provided configuration map, serializes the config data,
    /// and instantiates the requested capabilities.
    pub async fn instantiate_from_config(
        &self,
        config: &CapabilityConfig,
    ) -> Result<Capability, CapabilityError> {
        tracing::debug!(ident = ?self.ident, library_id = self.id, "Instantiating capabilities from config");
        let mut objects = Vec::new();
        for class_name in config.classes.keys() {
            self.capabilities.get(class_name).ok_or_else(|| {
                tracing::warn!(class_name = %class_name, "Requested configuration for unknown capability");
                CapabilityError::CapabilityNotFound {
                    name: class_name.clone(),
                }
            })?;
        }

        for (cap_name, cap_class) in &self.capabilities {
            // 2. Serialize the config value to a PyroVec using JSON format
            let vec = if let Some(Some(config_val)) = config.classes.get(cap_name) {
                let writer = Json::<serde_json::Value>::new_writer(PyroVec::with_capacity(300));

                writer.write(config_val).map_err(|e| {
                    tracing::error!(
                        class = %cap_name,
                        error = %e,
                        "Failed to serialize configuration"
                    );
                    CapabilityError::ConfigSerialization {
                        class: cap_name.to_string(),
                        reason: e.to_string(),
                    }
                })?
            } else {
                PyroVec::ok()
            };
            let object_id = NEXT_OBJECT_ID.fetch_add(1, Ordering::SeqCst);
            let log_channel = create_log(self.id, object_id, 100);
            tracing::debug!(
                library_id = self.id,
                object_id,
                class = %cap_name,
                "Instantiating capability class"
            );
            // 3. Call create_instance on the ForeignClass
            let handle = cap_class
                .create_instance(vec.py_ref(), object_id, log_channel)
                .await
                .map_err(|e| {
                    tracing::error!(
                        class = %cap_name,
                        object_id,
                        error = %e,
                        "Failed to instantiate capability"
                    );
                    CapabilityError::Instantiation {
                        class: cap_name.clone(),
                        reason: e.to_string(),
                    }
                })?;

            objects.push((cap_name.clone(), handle));
        }

        tracing::info!(
            ident = ?self.ident,
            library_id = self.id,
            count = objects.len(),
            "Successfully instantiated all capabilities from config"
        );

        Ok(Capability {
            ident: self.ident.clone(),
            objects,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Capability {
    pub ident: CapabilityIdent,
    pub objects: Vec<(String, CapabilityObject)>,
}

impl Capability {
    pub fn ident(&self) -> &CapabilityIdent {
        &self.ident
    }

    pub fn get_index(&self, index: usize) -> Option<&CapabilityObject> {
        self.objects.get(index).map(|(_, obj)| obj)
    }

    pub fn get(&self, name: &str) -> Option<&CapabilityObject> {
        self.objects
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, obj)| obj)
    }

    pub fn iter(&self) -> impl Iterator<Item = &CapabilityObject> {
        self.objects.iter().map(|(_, obj)| obj)
    }

    pub fn iter_with_name(&self) -> impl Iterator<Item = (&str, &CapabilityObject)> {
        self.objects.iter().map(|(n, obj)| (n.as_str(), obj))
    }
}

#[async_trait]
pub trait ForeignCapability: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn lib_ident(&self) -> &CapabilityIdent;
    fn link(&self, linker: &mut Linker<PyroState>) -> Result<(), WasmError>;
    fn take_logs(&self) -> HashMap<String, Vec<String>>;
    fn clone_box(&self) -> Box<dyn ForeignCapability>;
}

impl ForeignCapability for Capability {
    fn name(&self) -> &str {
        self.ident.package.as_str()
    }

    fn lib_ident(&self) -> &CapabilityIdent {
        &self.ident
    }

    fn link(&self, linker: &mut Linker<PyroState>) -> Result<(), WasmError> {
        for (class_name, object) in self.objects.iter() {
            // Capture lib for the closures (Arc clone is cheap)

            for method_name in object.method_names() {
                let method_name = method_name.to_string();
                let class_name = class_name.to_string();
                let fn_name = method_name.clone();
                let object = object.clone();

                let ty = FuncType::new(
                    linker.engine(),
                    [ValType::I32, ValType::I32],
                    [ValType::I32],
                );

                tracing::debug!(class_name, method_name, "Linking");
                linker
                    .func_new_async(
                        &class_name,
                        &method_name,
                        ty,
                        move |caller, params, results| {
                            let client_ptr = params[0].unwrap_i32();
                            let input_ptr = params[1].unwrap_i32();

                            let object = object.clone();
                            let fn_name = fn_name.clone();
                            Box::new(async move {
                                tracing::debug!(
                                    class_name = object.name(),
                                    fn_name,
                                    "Calling function"
                                );

                                let mut io = PyroCallIo::from_caller(caller)?;
                                let client_view_ref = io.borrow_argument(client_ptr).await?;
                                let input_view_ref = io.borrow_argument(input_ptr).await?;

                                let output_view = object
                                    .call(&fn_name, client_view_ref, input_view_ref)
                                    .await?;
                                output_view.parse_as_error()?;

                                let ptr = io.new_input(&output_view).await?;

                                results[0] = Val::I32(ptr);

                                Ok(())
                            })
                        },
                    )
                    .map_err(|e| {
                        WasmError::LinkFunctionFailed(
                            class_name,
                            method_name,
                            format!("Error: {:#}\nBacktrace: {}", e, e.backtrace()),
                        )
                    })?;
            }

            let class_name = class_name.to_string();
            let object = object.clone();
            let ty = FuncType::new(linker.engine(), [ValType::I32], [ValType::I32]);
            linker
                .func_new_async(
                    &class_name,
                    "register",
                    ty,
                    move |caller, params, results| {
                        let object = object.clone();
                        Box::new(async move {
                            let mut io = PyroCallIo::from_caller(caller)?;
                            let client_ptr = params[0].unwrap_i32();

                            // Read input and get state — both are &self borrows.
                            let client_view_ref = io.borrow_argument(client_ptr).await?;
                            let client_view = PyroVec::clone_from_pyro(&client_view_ref).view();

                            // Call user function — consumes both borrows on return.
                            let output_view = object.register(client_view.py_ref()).await?;
                            output_view.parse_as_error()?;

                            // Write output back into wasm memory.
                            let ptr = io.new_input(&output_view).await?;
                            results[0] = Val::I32(ptr);
                            Ok(())
                        })
                    },
                )
                .map_err(|e| {
                    WasmError::LinkFunctionFailed(class_name, "register".to_string(), e.to_string())
                })?;
        }
        Ok(())
    }

    fn take_logs(&self) -> HashMap<String, Vec<String>> {
        let mut logs = HashMap::new();
        for (name, obj) in &self.objects {
            logs.insert(name.clone(), obj.take_logs());
        }
        logs
    }

    fn clone_box(&self) -> Box<dyn ForeignCapability> {
        Box::new(self.clone())
    }
}
