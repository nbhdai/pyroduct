use async_trait::async_trait;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::PyroError;
use crate::ffi::host::ForeignCapability;
use crate::format::PyroView;
use crate::transport::socket::capability::client::PyroClient;

pub struct RemoteLibrary {
    lib_name: String,
    interface: pyro_spec::InterfaceSpec<'static>,
    client: Arc<Mutex<PyroClient>>,
}

impl RemoteLibrary {
    /// Connect to a TCP address (e.g. `"127.0.0.1:9000"`).
    pub async fn connect_tcp(
        lib_name: String,
        addr: impl tokio::net::ToSocketAddrs + fmt::Debug,
    ) -> Result<Self, PyroError> {
        tracing::info!(lib = %lib_name, ?addr, "Connecting to remote capability library via TCP");
        let client = PyroClient::connect_tcp(addr).await?;
        let interface = client.interface().clone();
        Ok(Self {
            lib_name,
            interface,
            client: Arc::new(Mutex::new(client)),
        })
    }

    /// Connect to a Unix domain socket path.
    pub async fn connect_unix(
        lib_name: String,
        path: impl AsRef<Path> + fmt::Debug,
    ) -> Result<Self, PyroError> {
        tracing::info!(lib = %lib_name, ?path, "Connecting to remote capability library via Unix Socket");
        let client = PyroClient::connect_unix(path).await?;
        let interface = client.interface().clone();
        Ok(Self {
            lib_name,
            interface,
            client: Arc::new(Mutex::new(client)),
        })
    }

    /// Get an iterator over all classes in the remote library.
    pub fn classes(&self) -> impl Iterator<Item = RemoteClass> {
        let lib_name = self.lib_name.clone();
        let client = self.client.clone();
        self.interface.classes.clone().into_iter().map(move |class_spec| {
            let methods = class_spec
                .methods
                .iter()
                .map(|m| m.name.to_string())
                .collect();
            RemoteClass {
                class_name: class_spec.name.to_string(),
                lib_name: lib_name.clone(),
                methods,
                client: client.clone(),
            }
        })
    }

    /// Retrieve a class from the remote library by name.
    pub async fn get_class(&self, class_name: &str) -> Result<RemoteClass, PyroError> {
        tracing::debug!(lib = %self.lib_name, class = %class_name, "Retrieving remote class");
        let class_spec = self
            .interface
            .classes
            .iter()
            .find(|c| c.name == class_name)
            .ok_or_else(|| {
                PyroError::NotFound(format!(
                    "Class '{}' not found in remote interface spec",
                    class_name
                ))
            })?;

        let methods = class_spec
            .methods
            .iter()
            .map(|m| m.name.to_string())
            .collect();

        Ok(RemoteClass {
            class_name: class_name.to_string(),
            lib_name: self.lib_name.clone(),
            methods,
            client: self.client.clone(),
        })
    }
}

pub struct RemoteClass {
    class_name: String,
    lib_name: String,
    methods: Vec<String>,
    client: Arc<Mutex<PyroClient>>,
}

#[async_trait]
impl ForeignCapability for RemoteClass {
    fn name(&self) -> &str {
        &self.class_name
    }

    fn lib_name(&self) -> &str {
        &self.lib_name
    }

    fn method_names(&self) -> Vec<String> {
        self.methods
            .iter()
            .map(|m| format!("p__{}__{}__wasm", self.class_name, m))
            .collect()
    }

    async fn call(
        &self,
        method_name: &str,
        client_data: PyroView,
        input_data: PyroView,
    ) -> Result<PyroView, PyroError> {
        let mut client = self.client.lock().await;

        let prefix = format!("p__{}__", self.class_name);
        let suffix = "__wasm";
        let simple_name = if method_name.starts_with(&prefix) && method_name.ends_with(suffix) {
            &method_name[prefix.len()..(method_name.len() - suffix.len())]
        } else {
            method_name
        };

        client
            .call(&self.class_name, simple_name, client_data, input_data)
            .await
    }

    async fn register(&self, client_state: PyroView) -> Result<PyroView, PyroError> {
        let mut client = self.client.lock().await;
        let id = client.register_client_id(client_state).await?;

        use crate::format::Bridgeable;
        id.ship().map(|v| v.view())
    }

    fn take_logs(&self) -> Vec<String> {
        Vec::new()
    }

    fn clone_box(&self) -> Box<dyn ForeignCapability> {
        Box::new(Self {
            class_name: self.class_name.clone(),
            lib_name: self.lib_name.clone(),
            methods: self.methods.clone(),
            client: self.client.clone(),
        })
    }
}
