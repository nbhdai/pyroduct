use async_trait::async_trait;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::PyroError;
use crate::ffi::host::ForeignCapability;
use crate::format::PyroView;
use crate::transport::socket::capability::client::PyroClient;

pub struct SocketForeignCapability {
    class_name: String,
    lib_name: String,
    methods: Vec<String>,
    client: Arc<Mutex<PyroClient>>,
}

impl SocketForeignCapability {
    /// Connect to a TCP address (e.g. `"127.0.0.1:9000"`).
    pub async fn connect_tcp(
        lib_name: String,
        class_name: String,
        addr: impl tokio::net::ToSocketAddrs + fmt::Debug,
    ) -> Result<Self, PyroError> {
        let client = PyroClient::connect_tcp(addr).await?;
        Self::new(lib_name, class_name, client)
    }

    /// Connect to a Unix domain socket path.
    pub async fn connect_unix(
        lib_name: String,
        class_name: String,
        path: impl AsRef<Path> + fmt::Debug,
    ) -> Result<Self, PyroError> {
        let client = PyroClient::connect_unix(path).await?;
        Self::new(lib_name, class_name, client)
    }

    fn new(lib_name: String, class_name: String, client: PyroClient) -> Result<Self, PyroError> {
        let class_spec = client
            .interface()
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

        Ok(Self {
            class_name,
            lib_name,
            methods,
            client: Arc::new(Mutex::new(client)),
        })
    }
}

#[async_trait]
impl ForeignCapability for SocketForeignCapability {
    fn name(&self) -> &str {
        &self.class_name
    }

    fn lib_name(&self) -> &str {
        &self.lib_name
    }

    fn method_names(&self) -> Vec<String> {
        self.methods.clone()
    }

    async fn call(
        &self,
        method_name: &str,
        client_data: PyroView,
        input_data: PyroView,
    ) -> Result<PyroView, PyroError> {
        let mut client = self.client.lock().await;

        client
            .call(&self.class_name, method_name, client_data, input_data)
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
