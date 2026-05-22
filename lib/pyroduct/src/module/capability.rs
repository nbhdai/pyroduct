use dashmap::DashMap;
use indexmap::IndexMap;
use std::ops::Deref;
use std::path::Path;
use std::sync::Weak;
use std::sync::{
    Arc, LazyLock,
    atomic::{AtomicI64, AtomicU64, Ordering},
};
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::mpsc;

use libloading::{Library, Symbol};
use object::{Object, ObjectSymbol, SymbolKind};
use thiserror::Error;

use crate::ffi::{
    CapabilityRegisterFn, ClassExport,
    host::{ForeignClass, ForeignObject},
};
use crate::format::header::PyroData;
use crate::format::{
    PyroVec, PyroView,
    format::{PyroFormat, Writer},
    json::Json,
};
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

    Ok(symbols)
}

// =============================================================================
// Library loader
// =============================================================================

static LOADED_LIBRARIES: LazyLock<Mutex<HashMap<String, Weak<CapabilityLibrary>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct CapabilityLibrary {
    pub id: i64,
    pub name: String,
    pub capabilities: IndexMap<String, Arc<ForeignClass>>,
    pub interface: InterfaceSpec<'static>,
}

impl CapabilityLibrary {
    pub fn load(name: String, path: &Path) -> Result<Arc<Self>, CapabilityError> {
        let mut libraries = LOADED_LIBRARIES.lock().unwrap_or_else(|e| {
            tracing::error!(
                "LOADED_LIBRARIES mutex was poisoned (likely a panic in another thread)"
            );
            e.into_inner()
        });
        if let Some(lib) = libraries.get(&name).map(|w| w.upgrade()).flatten() {
            Ok(lib)
        } else {
            let lib = Arc::new(Self::load_inter(&name, path)?);
            libraries.insert(name, Arc::downgrade(&lib));
            Ok(lib)
        }
    }

    fn load_inter(name: &String, path: &Path) -> Result<Self, CapabilityError> {
        let path_str = path.display().to_string();

        // 1. Scan
        let pyro_symbols = scan_pyro_symbols(path)?;

        if pyro_symbols.is_empty() {
            return Err(CapabilityError::NoCapabilitiesFound { path: path_str });
        }

        // 2. Load Interface Spec
        let interface_path = path
            .parent()
            .unwrap_or(Path::new("."))
            .join("interface.json");
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
        let library =
            Arc::new(
                unsafe { Library::new(path) }.map_err(|e| CapabilityError::LibraryOpen {
                    path: path_str.clone(),
                    reason: e.to_string(),
                })?,
            );
        let id = NEXT_LIB_ID.fetch_add(1, Ordering::SeqCst);

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

            let class = unsafe { ForeignClass::from_export(name.clone(), library.clone(), export) }
                .map_err(|e| CapabilityError::Registration {
                    path: path_str.clone(),
                    symbol: sym.name.clone(),
                    reason: e.to_string(),
                })?;

            capabilities.insert(class.name().to_string(), Arc::new(class));
        }

        if capabilities.is_empty() {
            return Err(CapabilityError::NoCapabilitiesFound { path: path_str });
        }

        Ok(Self {
            id,
            name: name.clone(),
            capabilities,
            interface,
        })
    }

    /// Iterates through the provided configuration map, serializes the config data,
    /// and instantiates the requested capabilities.
    pub async fn instantiate_from_config(
        &self,
        config: &HashMap<String, serde_json::Value>,
    ) -> Result<Capability, CapabilityError> {
        let mut objects = HashMap::new();
        for class_name in config.keys() {
            self.capabilities.get(class_name).ok_or_else(|| {
                CapabilityError::CapabilityNotFound {
                    name: class_name.clone(),
                }
            })?;
        }

        for (cap_name, cap_class) in &self.capabilities {
            // 2. Serialize the config value to a PyroVec using JSON format
            let vec = if let Some(config_val) = config.get(cap_name) {
                let writer = Json::<serde_json::Value>::new_writer(PyroVec::with_capacity(300));

                writer
                    .write(config_val)
                    .map_err(|e| CapabilityError::ConfigSerialization {
                        class: cap_name.clone(),
                        reason: e.to_string(),
                    })?
            } else {
                PyroVec::ok()
            };
            let object_id = NEXT_OBJECT_ID.fetch_add(1, Ordering::SeqCst);
            let log_channel = create_log(self.id, object_id, 100);
            // 3. Call create_instance on the ForeignClass
            let handle = cap_class
                .create_instance(vec.py_ref(), object_id, log_channel)
                .await
                .map_err(|e| CapabilityError::Instantiation {
                    class: cap_name.clone(),
                    reason: e.to_string(),
                })?;

            objects.insert(cap_name.clone(), handle);
        }

        Ok(Capability {
            lib_name: self.name.clone(),
            objects,
        })
    }

    /// Iterates through the provided configuration map, serializes the config data,
    /// and instantiates the requested capabilities.
    pub async fn instantiate_class(
        &self,
        class: &str,
        config: Option<&serde_json::Value>,
    ) -> Result<ForeignObject, CapabilityError> {
        let cap_class = self.capabilities.get_index_of(class).ok_or_else(|| {
            CapabilityError::CapabilityNotFound {
                name: class.to_string(),
            }
        })?;

        let vec = if let Some(config_val) = config {
            let writer = Json::<serde_json::Value>::new_writer(PyroVec::with_capacity(300));

            writer
                .write(config_val)
                .map_err(|e| CapabilityError::ConfigSerialization {
                    class: class.to_string(),
                    reason: e.to_string(),
                })?
        } else {
            PyroVec::ok()
        };

        self.instantiate_class_raw(cap_class as u8, vec.view())
            .await
    }

    pub async fn instantiate_class_raw(
        &self,
        class: u8,
        config: PyroView,
    ) -> Result<ForeignObject, CapabilityError> {
        let (_, cap_class) = self.capabilities.get_index(class as usize).ok_or_else(|| {
            CapabilityError::CapabilityNotFound {
                name: class.to_string(),
            }
        })?;

        let object_id = NEXT_OBJECT_ID.fetch_add(1, Ordering::SeqCst);
        let log_channel = create_log(self.id, object_id, 100);

        let handle = cap_class
            .create_instance(config.py_ref(), object_id, log_channel)
            .await
            .map_err(|e| CapabilityError::Instantiation {
                class: class.to_string(),
                reason: e.to_string(),
            })?;

        Ok(handle)
    }
}

#[derive(Debug)]
pub struct Capability {
    lib_name: String,
    objects: HashMap<String, ForeignObject>,
}

impl Capability {
    pub fn name(&self) -> &str {
        &self.lib_name
    }
}

impl Deref for Capability {
    type Target = HashMap<String, ForeignObject>;

    fn deref(&self) -> &Self::Target {
        &self.objects
    }
}
