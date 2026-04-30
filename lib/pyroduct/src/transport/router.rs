use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use dashmap::DashMap;

use crate::PyroError;
use crate::ffi::host::ForeignObject;
use crate::format::header::{PyroHeader};
use crate::format::{Bridgeable, PyroVec, PyroView, SpecWire};
use crate::module::capability::CapabilityLibrary;

/// A router that dispatches requests to foreign objects loaded from a library.
///
/// The router maintains a [`CapabilityLibrary`] and a map of instantiated
/// [`ForeignObject`]s. Requests are routed based on their `class_id` and `fn_id`:
///
/// - `fn_id == 0`: Configures the class identified by `class_id`. The request
///   payload is used as the configuration. A single object is created for the class.
/// - `fn_id > 0`: Calls the method at index `fn_id - 1` on the object associated
///   with `class_id`.
pub struct PyroRouter {
    library: Arc<CapabilityLibrary>,
    objects: DashMap<u8, ForeignObject>,
    client_id: AtomicU32,
    clients: DashMap<u32, PyroVec>,
}

impl PyroRouter {
    /// Load a library and create a new [`PyroRouter`].
    pub fn load(name: String, path: impl AsRef<Path>) -> Result<Self, PyroError> {
        let library = CapabilityLibrary::load(name, path.as_ref())
            .map_err(|e| PyroError::NotFound(e.to_string()))?;
        Ok(Self {
            library,
            objects: DashMap::new(),
            client_id: AtomicU32::new(0),
            clients: DashMap::new(),
        })
    }

    /// Handle an incoming request.
    ///
    /// This dispatches the request to the appropriate object or configures a new one.
    pub async fn handle(&self, request: PyroView) -> Result<PyroVec, PyroError> {
        let class_id = request.class_id();
        let fn_id = request.fn_id();

        match fn_id {
            0 => {
                self.library.interface.to_wire()
            }
            1 => {
                let object = self
                    .library
                    .instantiate_class_raw(class_id, request)
                    .await
                    .map_err(|e| PyroError::NotFound(e.to_string()))?;

                self.objects.insert(class_id, object);
                Ok(PyroVec::ok())
            }
            2 => {
                let id = self
                    .client_id
                    .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                self.clients.insert(id, request.clone_to_vec());
                id.ship()
            }
            3 => {
                let object = self.objects.get(&class_id).ok_or_else(|| {
                    PyroError::NotFound(format!("Object for class ID {} not configured", class_id))
                })?;

                object.reset().await?;
                Ok(PyroVec::ok())
            }
            other => {
                // Routing: Dispatch to the already-instantiated object.
                let object = self.objects.get(&(class_id as u8)).ok_or_else(|| {
                    PyroError::NotFound(format!("Object for class ID {} not configured", class_id))
                })?;
                // fn_id 4 maps to methods[0], etc.
                let method_index = (other - 4) as usize;
                let client_id = request.client_id();

                let client_data = self.clients.get(&client_id).ok_or_else(|| {
                    PyroError::NotFound(format!(
                        "Object for client ID {} not configured",
                        client_id
                    ))
                })?;
                object
                    .call_index(method_index, client_data.view(), request)
                    .await
            }
        }
    }
}
