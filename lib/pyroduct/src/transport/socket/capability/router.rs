use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use dashmap::DashMap;

use crate::PyroError;
use crate::ffi::host::ForeignObject;
use crate::format::header::{PyroData, PyroHeader};
use crate::format::{Bridgeable, PyroVec, PyroView, SpecWire};
use crate::module::capability::CapabilityLibrary;

/// A router that dispatches requests to foreign objects loaded from a library.
///
/// The router maintains a [`CapabilityLibrary`] and a vector of instantiated
/// [`ForeignObject`]s. Requests are routed based on their `class_id` and `fn_id`:
///
/// - `fn_id == 0`: Configures the class identified by `class_id`. The request
///   payload is used as the configuration. A single object is created for the class.
/// - `fn_id > 0`: Calls the method at index `fn_id - 1` on the object associated
///   with `class_id`.
pub struct PyroRouter {
    library: Arc<CapabilityLibrary>,
    objects: Vec<Option<ForeignObject>>,
    client_id: AtomicU32,
    clients: DashMap<u32, PyroView>,
}

impl PyroRouter {
    /// Load a library and create a new [`PyroRouter`].
    pub fn load(name: String, path: impl AsRef<Path> + fmt::Debug) -> Result<Self, PyroError> {
        tracing::info!(%name, ?path, "Loading capability library");
        let library = CapabilityLibrary::load(name, path.as_ref())
            .map_err(|e| PyroError::NotFound(e.to_string()))?;
        let len = library.capabilities.len();
        Ok(Self {
            library,
            objects: vec![None; len],
            client_id: AtomicU32::new(1),
            clients: DashMap::new(),
        })
    }

    /// Configure a capability class by instantiating it and storing it in the objects vector.
    pub async fn configure(&mut self, class_id: u8, request: PyroView) -> Result<(), PyroError> {
        tracing::info!(%class_id, "Instantiating class");
        let object = self
            .library
            .instantiate_class_raw(class_id, request)
            .await
            .map_err(|e| PyroError::NotFound(e.to_string()))?;

        if (class_id as usize) >= self.objects.len() {
            return Err(PyroError::NotFound(format!(
                "Class ID {} is out of range for library capabilities (length {})",
                class_id,
                self.objects.len()
            )));
        }
        self.objects[class_id as usize] = Some(object);
        Ok(())
    }

    /// Handle an incoming request.
    ///
    /// This dispatches the request to the appropriate object or configures a new one.
    pub async fn handle(&self, request: PyroView) -> Result<PyroView, PyroError> {
        let class_id = request.class_id();
        let fn_id = request.fn_id();

        tracing::debug!(%class_id, %fn_id, "Handling request");

        match fn_id {
            0 => self.library.interface.to_wire(),
            1 => Err(PyroError::not_permitted("Cannot configure remote object")),
            2 => {
                tracing::info!("Registering new client");
                let id = self
                    .client_id
                    .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                self.clients.insert(id, request.clone_to_vec().view());
                id.ship().map(|v| v.view())
            }
            3 => {
                tracing::info!(%class_id, "Resetting object");
                let object = self
                    .objects
                    .get(class_id as usize)
                    .and_then(|o| o.as_ref())
                    .ok_or_else(|| {
                        let err = format!("Object for class ID {} not configured", class_id);
                        tracing::warn!(%err);
                        PyroError::NotFound(err)
                    })?;

                object.reset().await?;
                Ok(PyroVec::ok().view())
            }
            other => {
                // Routing: Dispatch to the already-instantiated object.
                let object = self
                    .objects
                    .get(class_id as usize)
                    .and_then(|o| o.as_ref())
                    .ok_or_else(|| {
                        let err = format!("Object for class ID {} not configured", class_id);
                        tracing::warn!(%err);
                        PyroError::NotFound(err)
                    })?;
                // fn_id 4 maps to methods[0], etc.
                let method_index = (other - 4) as usize;
                let client_id = request.client_id();

                tracing::debug!(%class_id, %method_index, %client_id, "Dispatching method call");

                let client_data = self.clients.get(&client_id).ok_or_else(|| {
                    let err = format!("Object for client ID {} not configured", client_id);
                    tracing::warn!(%err);
                    PyroError::NotFound(err)
                })?;
                object
                    .call_index(method_index, client_data.py_ref(), request.py_ref())
                    .await
            }
        }
    }
}
