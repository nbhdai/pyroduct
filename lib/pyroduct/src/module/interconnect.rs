use crate::CapturedError;

use crate::format::PyroVec;
use crate::format::header::{PyroData, PyroHeader, PyroHeaderMut};
use crate::format::{
    Bridgeable, PyroRow,
    format::{PyroFormat, Writer},
    json::Json,
};
use crate::module::call::PyroCallIo;
use crate::module::{PyroInstance, PyroState, WasmError};
use pyro_spec::ModuleFunc;
use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::{FuncType, Linker, Val, ValType};

#[async_trait::async_trait]
pub trait PlaybookInterconnect: Send + Sync + 'static {
    fn playbooks(&self) -> &HashMap<String, ModuleFunc<'static>>;

    async fn call(
        &self,
        name: &str,
        row: PyroRow<'_>,
    ) -> Result<(u32, PyroRow<'static>), CapturedError>;

    async fn call_session(
        &self,
        name: &str,
        client_id: u32,
        row: PyroRow<'_>,
    ) -> Result<PyroRow<'static>, CapturedError>;
}

pub async fn add_playbooks(
    playbooks: &HashMap<String, ModuleFunc<'static>>,
    instance: &mut PyroInstance,
) -> Result<(), WasmError> {
    let writer =
        Json::<HashMap<String, ModuleFunc<'static>>>::new_writer(PyroVec::with_capacity(1024));
    let vec = writer.write(&playbooks).map_err(|e| {
        WasmError::InstantiationFailed(format!("Failed to serialize playbooks: {}", e))
    })?;
    let view = vec.view();

    let mut io = PyroCallIo::new(&mut instance.store, instance.memory);
    let input_ptr = io.new_input(&view).await.map_err(|e| {
        WasmError::InstantiationFailed(format!("Failed to allocate interconnect specs: {}", e))
    })?;

    if let Ok(populate_fn) = instance
        .instance
        .get_typed_func::<i32, i32>(&mut instance.store, "populate_interconnect_specs")
    {
        let output_ptr = populate_fn
            .call_async(&mut instance.store, input_ptr)
            .await
            .map_err(|e| {
                WasmError::InstantiationFailed(format!(
                    "Failed to populate interconnect specs: {}",
                    e
                ))
            })?;

        let mut io = PyroCallIo::new(&mut instance.store, instance.memory);
        let result_vec = io.get_output(output_ptr).await.map_err(|e| {
            WasmError::InstantiationFailed(format!("Failed to read populate result: {}", e))
        })?;
        result_vec.parse_as_error().map_err(|e| {
            WasmError::InstantiationFailed(format!(
                "Populate interconnect specs returned error: {}",
                e
            ))
        })?;
    }

    Ok(())
}

pub struct SessionIdGuard<'a> {
    state: &'a std::sync::Mutex<Option<u32>>,
}

impl<'a> SessionIdGuard<'a> {
    pub fn new(state: &'a std::sync::Mutex<Option<u32>>, session_id: u32) -> Self {
        *state.lock().unwrap() = Some(session_id);
        Self { state }
    }
}

impl<'a> Drop for SessionIdGuard<'a> {
    fn drop(&mut self) {
        *self.state.lock().unwrap() = None;
    }
}

pub fn link_interconnect(
    linker: &mut Linker<PyroState>,
    interconnect: Arc<dyn PlaybookInterconnect>,
) -> Result<(), WasmError> {
    let ty = FuncType::new(
        linker.engine(),
        [ValType::I32, ValType::I32, ValType::I32],
        [ValType::I32],
    );
    let interconnect = interconnect.clone();
    linker
        .func_new_async(
            "env",
            "call_interconnect",
            ty,
            move |caller, params, results| {
                let name_ptr = params[0].unwrap_i32();
                let name_len = params[1].unwrap_i32();
                let input_ptr = params[2].unwrap_i32();
                let interconnect = interconnect.clone();

                Box::new(async move {
                    let current_session_id = { *caller.data().current_session_id.lock().unwrap() };
                    let mut io = PyroCallIo::from_caller(caller)?;
                    let name = io.get_name(name_ptr, name_len)?;
                    tracing::debug!(name = %name, "link_interconnect: guest requested playbook call");

                    let input_view_ref = io.borrow_argument(input_ptr).await?;
                    let session_id = if input_view_ref.mux_id() != 0 {
                        Some(input_view_ref.mux_id())
                    } else {
                        current_session_id
                    };
                    let input_row_ref = PyroRow::expose_view(input_view_ref)
                        .map_err(|e| WasmError::InputMemory(wasmtime::Error::msg(e.to_string())))?;
                    let input = PyroRow::from(&*input_row_ref);

                    let result_row = if let Some(session_id) = session_id {
                        interconnect
                            .call_session(&name, session_id, input)
                            .await
                            .map(|row| (session_id, row))
                    } else {
                        interconnect.call(&name, input).await
                    };

                    match result_row {
                        Ok((session_id, row)) => {
                            let mut result_vec = row.ship().map_err(|e| {
                                WasmError::OutputMemory(wasmtime::Error::msg(e.to_string()))
                            })?;
                            result_vec.set_mux_id(session_id);
                            tracing::debug!(
                                header = ?result_vec.header(),
                                status = ?result_vec.status(),
                                len = result_vec.len(),
                                "link_interconnect: result_vec details"
                            );
                            let view = result_vec.view();
                            let ptr = io.new_input(&view).await?;
                            results[0] = Val::I32(ptr);
                        }
                        Err(e) => {
                            tracing::error!(error = ?e, "link_interconnect: host error");
                            let mut vec = e.encode();
                            vec.set_status(crate::format::header::DataStatus::RkyvError);
                            let view = vec.view();
                            let ptr = io.new_input(&view).await?;
                            results[0] = Val::I32(ptr);
                        }
                    }
                    Ok(())
                })
            },
        )
        .map_err(|e| {
            WasmError::LinkFunctionFailed(
                "env".to_string(),
                "call_interconnect".to_string(),
                e.to_string(),
            )
        })?;
    Ok(())
}
