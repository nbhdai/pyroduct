use async_trait::async_trait;
use pyro_artifacts::cargo::CapabilityIdent;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use wasmtime::{FuncType, Linker, Val, ValType};

use crate::format::header::PyroData;
use crate::format::{Bridgeable, PyroVec};
use crate::module::PyroState;
use crate::module::WasmError;
use crate::module::call::PyroCallIo;
use crate::module::capability::ForeignCapability;
use crate::transport::socket::capability::client::PyroClient;
use crate::{CapturedError, PyroError};

pub struct RemoteCapability {
    lib_ident: CapabilityIdent,
    interface: pyro_spec::InterfaceSpec<'static>,
    client: Arc<Mutex<PyroClient>>,
}

impl RemoteCapability {
    /// Connect to a TCP address (e.g. `"127.0.0.1:9000"`).
    pub async fn connect_tcp(
        lib_ident: CapabilityIdent,
        addr: impl tokio::net::ToSocketAddrs + fmt::Debug,
    ) -> Result<Self, PyroError> {
        tracing::info!(lib = ?lib_ident, ?addr, "Connecting to remote capability library via TCP");
        let client = PyroClient::connect_tcp(addr).await?;
        let interface = client.interface().clone();
        Ok(Self {
            lib_ident,
            interface,
            client: Arc::new(Mutex::new(client)),
        })
    }

    /// Connect to a Unix domain socket path.
    pub async fn connect_unix(
        lib_ident: CapabilityIdent,
        path: impl AsRef<Path> + fmt::Debug,
    ) -> Result<Self, PyroError> {
        tracing::info!(lib = ?lib_ident, ?path, "Connecting to remote capability library via Unix Socket");
        let client = PyroClient::connect_unix(path).await?;
        let interface = client.interface().clone();
        Ok(Self {
            lib_ident,
            interface,
            client: Arc::new(Mutex::new(client)),
        })
    }
}

#[async_trait]
impl ForeignCapability for RemoteCapability {
    fn name(&self) -> &str {
        self.lib_ident.package.as_str()
    }

    fn lib_ident(&self) -> &CapabilityIdent {
        &self.lib_ident
    }

    fn take_logs(&self) -> HashMap<String, Vec<String>> {
        HashMap::new()
    }

    fn link(&self, linker: &mut Linker<PyroState>) -> Result<(), WasmError> {
        for class_spec in &self.interface.classes {
            let class_name = class_spec.name.to_string();

            // Link all methods
            for method_spec in &class_spec.methods {
                let method_name = method_spec.name.to_string();
                let client = self.client.clone();

                let class_name_for_closure = class_name.clone();
                let method_name_for_closure = method_name.clone();

                let class_name_for_err = class_name.clone();
                let method_name_for_err = method_name.clone();

                let ty = FuncType::new(
                    linker.engine(),
                    [ValType::I32, ValType::I32],
                    [ValType::I32],
                );

                tracing::debug!(class_name, method_name, "Linking remote method");
                let wasm_method_name = format!("p__{}__{}__wasm", class_name, method_name);
                linker
                    .func_new_async(
                        &class_name,
                        &wasm_method_name,
                        ty,
                        move |caller, params, results| {
                            let client_ptr = params[0].unwrap_i32();
                            let input_ptr = params[1].unwrap_i32();

                            let client = client.clone();
                            let class_name = class_name_for_closure.clone();
                            let method_name = method_name_for_closure.clone();

                            Box::new(async move {
                                let mut io = PyroCallIo::from_caller(caller)?;
                                let client_view_ref = io.borrow_argument(client_ptr).await?;
                                let input_view_ref = io.borrow_argument(input_ptr).await?;

                                // Clone references into owned PyroViews to pass to remote call
                                let client_view = PyroVec::clone_from_pyro(&client_view_ref).view();
                                let input_view = PyroVec::clone_from_pyro(&input_view_ref).view();

                                let output_view = client
                                    .lock()
                                    .await
                                    .call(&class_name, &method_name, client_view, input_view)
                                    .await
                                    .map_err(|e| {
                                        PyroError::transport(CapturedError::new(e.to_string()))
                                    })?;
                                output_view.parse_as_error()?;

                                let ptr = io.new_input(&output_view).await?;

                                results[0] = Val::I32(ptr);

                                Ok(())
                            })
                        },
                    )
                    .map_err(|e| {
                        WasmError::LinkFunctionFailed(
                            class_name_for_err,
                            method_name_for_err,
                            format!("Error: {:#}", e),
                        )
                    })?;
            }

            // Link the register method
            let class_name = class_name.clone();
            let client = self.client.clone();
            let ty = FuncType::new(linker.engine(), [ValType::I32], [ValType::I32]);
            linker
                .func_new_async(
                    &class_name,
                    "register",
                    ty,
                    move |caller, params, results| {
                        let client = client.clone();
                        Box::new(async move {
                            let mut io = PyroCallIo::from_caller(caller)?;
                            let client_ptr = params[0].unwrap_i32();

                            // Read input and get state
                            let client_view_ref = io.borrow_argument(client_ptr).await?;
                            let client_view = PyroVec::clone_from_pyro(&client_view_ref).view();

                            // Call register on the remote client
                            let client_id = client
                                .lock()
                                .await
                                .register_client_id(client_view)
                                .await
                                .map_err(|e| {
                                    PyroError::transport(CapturedError::new(e.to_string()))
                                })?;
                            let output_view = client_id.ship()?.view(); // Serialize client_id to PyroVec
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

    fn clone_box(&self) -> Box<dyn ForeignCapability> {
        Box::new(Self {
            lib_ident: self.lib_ident.clone(),
            client: self.client.clone(),
            interface: self.interface.clone(),
        })
    }
}
