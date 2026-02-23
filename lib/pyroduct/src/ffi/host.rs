use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use dashmap::DashMap;
use std::sync::{atomic::{AtomicI64, AtomicU64, Ordering}, LazyLock, Arc};
use tokio::sync::mpsc;

use libloading::{Library, Symbol};
use object::{Object, ObjectSymbol, SymbolKind};
use thiserror::Error;

use crate::PyroVec;
use crate::ffi::{CapabilityRegisterFn, ClassExport, ForeignClass, ForeignObject};
use crate::format::{PyroFormat, Writer};
use crate::json::Json;

// =============================================================================
// Error
// =============================================================================

#[derive(Error, Debug)]
pub enum CapabilityLoading {
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



static CATCH_LOG_SENDER: LazyLock<DashMap<i64, mpsc::Sender<String>>> = LazyLock::new(DashMap::new);
static LOG_SENDERS: LazyLock<DashMap<(i64, u64), mpsc::Sender<String>>> = LazyLock::new(DashMap::new);
static NEXT_LIB_ID: AtomicI64 = AtomicI64::new(1);
static NEXT_OBJECT_ID: AtomicU64 = AtomicU64::new(1);

/// Create a new log channel. Returns the numeric ID (to pass to the C side)
/// and a receiver you can poll to build up your logs.
///
/// `buffer` controls how many messages can queue before back-pressure kicks in.
pub fn create_log(library_id: i64, span_id: u64, buffer: usize) -> mpsc::Receiver<String> {
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
pub unsafe extern "C" fn log_callback(library_id: i64, span_id: u64, msg: *const u8, msg_len: usize) {
    let data = unsafe { std::slice::from_raw_parts(msg, msg_len) };
    let log_msg = String::from_utf8_lossy(data).trim_end().to_string();
    if span_id == 0 {
        if let Some(tx) = CATCH_LOG_SENDER.get(&library_id) {
            match tx.try_send(log_msg) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    eprintln!("[log] channel full for id=({library_id},{span_id}), dropping message");
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {}
            }
        } else {
            tracing::debug!(log_msg, "Uncaught Capability Log");
        }
    } else {
        if let Some(tx) = LOG_SENDERS.get(&(library_id, span_id)) {
            match tx.try_send(log_msg) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    eprintln!("[log] channel full for id=({library_id},{span_id}), dropping message");
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    LOG_SENDERS.remove(&(library_id, span_id));
                }
            }
        } else {
            tracing::debug!(log_msg, "Uncaught Capability Log");
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

pub fn scan_pyro_symbols(path: &Path) -> Result<Vec<ScannedSymbol>, CapabilityLoading> {
    let path_str = path.display().to_string();

    let bin_data = std::fs::read(path).map_err(|e| CapabilityLoading::FileRead {
        path: path_str.clone(),
        reason: e.to_string(),
    })?;

    let file = object::File::parse(&*bin_data).map_err(|e| CapabilityLoading::BinaryParse {
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


pub struct CapabilityLibrary {
    pub id: i64,
    pub name: String,
    pub capabilities: HashMap<String, Arc<ForeignClass>>,
}

impl CapabilityLibrary {
    pub fn load(name: String, path: &Path) -> Result<Self, CapabilityLoading> {
        let path_str = path.display().to_string();

        // 1. Scan
        let pyro_symbols = scan_pyro_symbols(path)?;

        if pyro_symbols.is_empty() {
            return Err(CapabilityLoading::NoCapabilitiesFound { path: path_str });
        }

        // 2. Load
        let library = Arc::new(unsafe { Library::new(path) }.map_err(|e| {
            CapabilityLoading::LibraryOpen {
                path: path_str.clone(),
                reason: e.to_string(),
            }
        })?);
        let id = NEXT_LIB_ID.fetch_add(1, Ordering::SeqCst);

        // 3. Register
        let mut capabilities = HashMap::with_capacity(pyro_symbols.len());
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
                unsafe { ForeignClass::from_export(library.clone(), export) }.map_err(|e| {
                    CapabilityLoading::Registration {
                        path: path_str.clone(),
                        symbol: sym.name.clone(),
                        reason: e.to_string(),
                    }
                })?;

            capabilities.insert(class.name().to_string(), Arc::new(class));
        }

        if capabilities.is_empty() {
            return Err(CapabilityLoading::NoCapabilitiesFound { path: path_str });
        }

        Ok(Self { id, name, capabilities })
    }

    /// Iterates through the provided configuration map, serializes the config data,
    /// and instantiates the requested capabilities.
    pub async fn instantiate_from_config(
        &self,
        config: &HashMap<String, serde_json::Value>,
    ) -> Result<Capability, CapabilityLoading> {
        let mut objects = HashMap::new();
        for class_name in config.keys() {
            self.capabilities.get(class_name).ok_or_else(|| {
                CapabilityLoading::CapabilityNotFound {
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
                        .map_err(|e| CapabilityLoading::ConfigSerialization {
                            class: cap_name.clone(),
                            reason: e.to_string(),
                        })?
            } else {
                PyroVec::ok()
            };
            let object_id = NEXT_OBJECT_ID.fetch_add(1, Ordering::SeqCst);
            let log_channel = create_log(self.id, object_id, 100);
            // 3. Call create_instance on the ForeignClass
            let handle = cap_class.create_instance(vec.view(), object_id, log_channel).await.map_err(|e| {
                CapabilityLoading::Instantiation {
                    class: cap_name.clone(),
                    reason: e.to_string(),
                }
            })?;

            objects.insert(cap_name.clone(), handle);
        }

        Ok(Capability { lib_name: self.name.clone(), objects })
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
