use std::{
    fmt,
    ops::DerefMut,
    slice,
    sync::{Arc, Mutex},
};

use libloading::Library;
use tokio::sync::oneshot;

use crate::{
    CapturedError, PyroError,
    ffi::{
        ClassExport, ClassInitFn, ClassResetFn, ClientRegisterFn, Function, MethodExport,
        PyroObject,
        host::{ClientRegisterFuture, MethodCallFuture, ObjectInitFuture, ObjectResetFuture},
    },
    format::{PyroVec, PyroView, header::PyroData},
};

pub struct ForeignClass {
    name: String,
    lib_name: String,
    _library: Option<Arc<Library>>,
    methods: Vec<ForeignMethod>,
    init: ClassInitFn,
    reset: ClassResetFn,
    register: ClientRegisterFn,
}
impl fmt::Debug for ForeignClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForeignClass")
            .field("name", &self.name)
            .field("library", &self._library)
            .field("methods", &self.methods.len())
            .finish()
    }
}

struct ForeignMethod {
    pub name: String,
    pub pointer: Function,
}

impl ForeignMethod {
    fn new(method: &MethodExport) -> Result<Self, CapturedError> {
        if method.name.is_null() {
            return Err(CapturedError::new(
                "Found a method with an empty name (null pointer)",
            ));
        }
        if method.name_len == 0 {
            return Err(CapturedError::new("Found a method with an empty name"));
        }
        let name_bytes = unsafe { slice::from_raw_parts(method.name, method.name_len) };
        let func_name = std::str::from_utf8(name_bytes).map_err(|err| {
            CapturedError::new(format!("Method does not have a valid utf8 name: {err}"))
        })?;
        let pointer = method.func;
        Ok(Self {
            name: func_name.to_string(),
            pointer,
        })
    }
}

impl ForeignClass {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn method_names(&self) -> impl Iterator<Item = &str> {
        self.methods.iter().map(|m| m.name.as_str())
    }

    pub unsafe fn from_export(
        lib_name: String,
        library: Arc<Library>,
        export: ClassExport,
    ) -> Result<Self, CapturedError> {
        unsafe { Self::from_export_inter(lib_name, Some(library), export) }
    }

    pub(crate) unsafe fn from_export_inter(
        lib_name: String,
        library: Option<Arc<Library>>,
        export: ClassExport,
    ) -> Result<Self, CapturedError> {
        let name = if export.name.is_null() || export.name_len == 0 {
            return Err(CapturedError::new("Empty name, unable to link"));
        } else {
            let name_bytes = unsafe { slice::from_raw_parts(export.name, export.name_len) };
            let name_str = std::str::from_utf8(name_bytes).map_err(|err| {
                CapturedError::new(format!("Class does not have a valid utf8 name: {err}"))
            })?;
            name_str.to_string()
        };

        let methods_slice = if export.ptr.is_null() || export.len == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(export.ptr, export.len) }
        };

        let methods = methods_slice
            .iter()
            .map(ForeignMethod::new)
            .collect::<Result<Vec<_>, CapturedError>>()?;

        Ok(Self {
            name,
            lib_name,
            methods,
            _library: library,
            init: export.init,
            reset: export.reset,
            register: export.register,
        })
    }

    pub async fn create_instance(
        self: &Arc<Self>,
        config: PyroView<'_>,
        object_id: u64,
        mut log_channel: tokio::sync::mpsc::Receiver<String>,
    ) -> Result<ForeignObject, PyroError> {
        let obj = match self.init {
            ClassInitFn::Sync(f) => unsafe { (f)(config.ptr(), object_id) }.process(),
            ClassInitFn::Async(f) => {
                ObjectInitFuture::from_async(unsafe { (f)(config.ptr(), object_id) }).await
            }
        }?;

        let log_buffer = Arc::new(Mutex::new(Vec::new()));
        let task_buffer = Arc::clone(&log_buffer);

        // 1. Create the oneshot channel for the kill signal
        let (kill_tx, mut kill_rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            loop {
                // 2. Use select! to wait on either the mpsc receiver OR the kill receiver
                tokio::select! {
                    msg = log_channel.recv() => {
                        match msg {
                            Some(log_msg) => {
                                if let Ok(mut buffer) = task_buffer.lock() {
                                    buffer.push(log_msg);
                                }
                            }
                            None => break, // Exit if the log channel naturally closes
                        }
                    }
                    _ = &mut kill_rx => {
                        // Exit the loop immediately if the kill signal is received
                        // (or if the sender is dropped)
                        break;
                    }
                }
            }
        });

        Ok(ForeignObject {
            obj: Arc::new(obj),
            class: self.clone(),
            log_buffer,
            _log_task: Arc::new(LogTaskHandle {
                kill_tx: Some(kill_tx),
            }),
        })
    }

    fn find_method(&self, name: &str) -> Option<&ForeignMethod> {
        self.methods.iter().find(|m| m.name == name)
    }
}

#[derive(Clone)]
pub struct ForeignObject {
    class: Arc<ForeignClass>,
    obj: Arc<PyroObject>,
    pub log_buffer: Arc<Mutex<Vec<String>>>,
    // Wrapped in Option so we can take() it to send the signal,
    // and Arc<Mutex> so ForeignObject remains Cloneable.
    _log_task: Arc<LogTaskHandle>,
}

impl fmt::Debug for ForeignObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForeignClass")
            .field("class", &self.class)
            .field("obj", &self.obj.object_id)
            .finish()
    }
}

impl ForeignObject {
    pub fn name(&self) -> &str {
        &self.class.name
    }

    pub fn lib_name(&self) -> &str {
        &self.class.lib_name
    }

    pub fn method_names(&self) -> impl Iterator<Item = &str> {
        self.class.methods.iter().map(|m| m.name.as_str())
    }

    /// Resets the object. Locks the individual object, allowing others to be used.
    pub async fn reset(&self) -> Result<(), PyroError> {
        match self.class.reset {
            ClassResetFn::Sync(f) => {
                let vec_ptr = unsafe { f(self.obj.ref_ptr()) };
                unsafe { PyroVec::from_raw(vec_ptr) }.and_then(|v| v.parse_as_error())
            }
            ClassResetFn::Async(f) => {
                let fut_res = unsafe { f(self.obj.ref_ptr()) };
                ObjectResetFuture::from_async(fut_res).await
            }
            ClassResetFn::Null => Ok(()),
        }
    }
    /// Registers a client
    pub async fn register(&self, client_state: PyroView<'_>) -> Result<PyroVec, PyroError> {
        match self.class.register {
            ClientRegisterFn::Sync(f) => {
                let vec_ptr = unsafe { f(self.obj.ref_ptr(), client_state.ptr()) };
                let vec = unsafe { PyroVec::from_raw(vec_ptr) }?;
                vec.parse_as_error()?;
                Ok(vec)
            }
            ClientRegisterFn::Async(f) => {
                let fut_res = unsafe { f(self.obj.ref_ptr(), client_state.ptr()) };
                ClientRegisterFuture::from_async(fut_res).await
            }
            ClientRegisterFn::Null => Ok(PyroVec::ok()),
        }
    }

    pub async fn call(
        &self,
        method_name: &str,
        client_data: PyroView<'_>,
        input_data: PyroView<'_>,
    ) -> Result<PyroVec, PyroError> {
        let method = self
            .class
            .find_method(method_name)
            .ok_or_else(|| PyroError::NotFound(format!("Object method {method_name} not found")))?;

        match method.pointer {
            Function::Sync(f) => unsafe {
                PyroVec::from_raw((f)(self.obj.ref_ptr(), client_data.ptr(), input_data.ptr()))
            },
            Function::Async(f) => {
                MethodCallFuture::from_async(unsafe {
                    (f)(self.obj.ref_ptr(), client_data.ptr(), input_data.ptr())
                })
                .await
            }
        }
    }

    pub fn take_logs(&self) -> Vec<String> {
        let mut logs = self.log_buffer.lock().unwrap();
        let log_cap = logs.capacity();
        let mut fresh_logs = Vec::with_capacity(log_cap);
        std::mem::swap(logs.deref_mut(), &mut fresh_logs);
        fresh_logs
    }
}

struct LogTaskHandle {
    kill_tx: Option<oneshot::Sender<()>>,
}

impl Drop for LogTaskHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.kill_tx.take() {
            let _ = tx.send(());
        }
    }
}
