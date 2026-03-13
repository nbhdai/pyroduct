use std::sync::Mutex;

use wasmtime::{ExternType, Instance, Module, Store, TypedFunc, ValType};

use crate::module::WasmError;

// ---------------------------------------------------------------------------
// PyroModule — validated wrapper around wasmtime::Module
// ---------------------------------------------------------------------------

pub struct PyroModule {
    module: Module,
    classes: Vec<String>,
}

impl PyroModule {
    /// Validates and wraps a Wasmtime Module.
    ///
    /// Checks that the module exports `new_input`, `grow_input`, `free_output`,
    /// and `memory` with the correct signatures.
    pub fn new(module: Module) -> Result<Self, WasmError> {
        Self::validate_import(
            &module,
            "env",
            "host_log",
            &[ValType::I32, ValType::I32],
            &[],
        )?;
        Self::validate_export(&module, "new_input", &[ValType::I32], &[ValType::I32])?;
        Self::validate_export(
            &module,
            "grow_input",
            &[ValType::I32, ValType::I32],
            &[ValType::I32],
        )?;
        Self::validate_export(&module, "free_output", &[ValType::I32], &[])?;

        // Ensure memory is exported
        if module.get_export("memory").is_none() {
            return Err(WasmError::MissingExport("memory".to_string()));
        }
        let classes = Self::gather_classes(&module)?;

        Ok(Self { module, classes })
    }

    /// Access the inner wasmtime `Module`.
    pub fn module(&self) -> &Module {
        &self.module
    }

    fn validate_export(
        module: &Module,
        name: &str,
        params: &[ValType],
        results: &[ValType],
    ) -> Result<(), WasmError> {
        let export = module
            .get_export(name)
            .ok_or_else(|| WasmError::MissingExport(name.to_string()))?;

        match export {
            ExternType::Func(func_type) => {
                if func_type.params().zip(params).all(|(f, p)| !f.matches(p))
                    || func_type.params().len() != params.len()
                    || func_type.results().zip(results).any(|(f, p)| !f.matches(p))
                    || func_type.results().len() != results.len()
                {
                    return Err(WasmError::SignatureMismatch(name.to_string()));
                }
            }
            _ => {
                return Err(WasmError::SignatureMismatch(name.to_string()));
            }
        }
        Ok(())
    }

    fn validate_import(
        module: &Module,
        cap: &str,
        name: &str,
        params: &[ValType],
        results: &[ValType],
    ) -> Result<(), WasmError> {
        let imports = module.imports();

        for import in imports {
            if !(cap == import.module() && name == import.name()) {
                continue;
            }
            match import.ty() {
                ExternType::Func(func_type) => {
                    if func_type.params().zip(params).all(|(f, p)| !f.matches(p))
                        || func_type.params().len() != params.len()
                        || func_type.results().zip(results).any(|(f, p)| !f.matches(p))
                        || func_type.results().len() != results.len()
                    {
                        return Err(WasmError::SignatureMismatch(name.to_string()));
                    }
                }
                _ => return Err(WasmError::SignatureMismatch(name.to_string())),
            }
            return Ok(());
        }
        Err(WasmError::MissingImport(format!("{cap}:{name}")))
    }

    fn gather_classes(module: &Module) -> Result<Vec<String>, WasmError> {
        let imports = module.imports();
        let mut pyro_classes = Vec::new();

        for import in imports {
            if import.module() == "env" {
                continue;
            }
            if import.name() == "register" {
                match import.ty() {
                    ExternType::Func(func_type) => {
                        if func_type.params().len() != 1 {
                            return Err(WasmError::SignatureMismatch(format!(
                                "Register function didn't have the correct number of parameters"
                            )));
                        }
                        if !matches!(func_type.param(0), Some(ValType::I32)) {
                            return Err(WasmError::SignatureMismatch(format!(
                                "Register function didn't take a pointer: {:?}",
                                func_type.param(0)
                            )));
                        }

                        if func_type.results().len() != 1 {
                            return Err(WasmError::SignatureMismatch(format!(
                                "Register function didn't have the correct number of returns"
                            )));
                        }
                        if !matches!(func_type.result(0), Some(ValType::I32)) {
                            return Err(WasmError::SignatureMismatch(format!(
                                "Register function didn't return a pointer"
                            )));
                        }
                    }
                    _ => {
                        return Err(WasmError::SignatureMismatch(format!(
                            "Register function didn't return a pointer"
                        )));
                    }
                }
                pyro_classes.push(import.module().to_string());
            }
        }
        Ok(pyro_classes)
    }

    pub fn has_class(&self, class: &str) -> bool {
        self.classes.iter().any(|s| s.as_str() == class)
    }
}

// ---------------------------------------------------------------------------
// PyroMethods — typed handles to the wasm-exported pyro functions
// ---------------------------------------------------------------------------

pub(crate) struct PyroMethods {
    new_input: TypedFunc<i32, i32>,
    _grow_input: TypedFunc<(i32, i32), i32>,
    free_output: TypedFunc<i32, ()>,
    last_error: Option<anyhow::Error>,
}

impl PyroMethods {
    /// The `new_input` typed function handle.
    pub fn new_input(&self) -> TypedFunc<i32, i32> {
        self.new_input.clone()
    }

    /// The `free_output` typed function handle.
    pub fn free_output(&self) -> TypedFunc<i32, ()> {
        self.free_output.clone()
    }

    /// Record an error that will be surfaced after the wasm call completes.
    pub fn set_error(&mut self, err: anyhow::Error) -> wasmtime::Error {
        let msg = err.to_string();
        self.last_error = Some(err);
        wasmtime::Error::msg(msg)
    }

    /// Take the last recorded error, if any.
    pub fn take_error(&mut self) -> Option<anyhow::Error> {
        self.last_error.take()
    }
}

// ---------------------------------------------------------------------------
// PyroState<T> — the Store data, wrapping user state + optional methods
// ---------------------------------------------------------------------------

pub struct PyroState {
    methods: Option<PyroMethods>,
    module_log: Mutex<Vec<String>>,
}

impl PyroState {
    /// Create an un-linked state (methods not yet resolved).
    pub fn new() -> Self {
        Self {
            methods: None,
            module_log: Mutex::new(Vec::new()),
        }
    }

    pub fn link(store: &mut Store<Self>, instance: &Instance) -> Result<(), WasmError> {
        let new_input = instance
            .get_typed_func::<i32, i32>(&mut *store, "new_input")
            .map_err(|_| WasmError::SignatureMismatch("new_input".to_string()))?;

        let grow_input = instance
            .get_typed_func::<(i32, i32), i32>(&mut *store, "grow_input")
            .map_err(|_| WasmError::SignatureMismatch("grow_input".to_string()))?;

        let free_output = instance
            .get_typed_func::<i32, ()>(&mut *store, "free_output")
            .map_err(|_| WasmError::SignatureMismatch("free_output".to_string()))?;
        store.data_mut().methods = Some(PyroMethods {
            new_input,
            _grow_input: grow_input,
            free_output,
            last_error: None,
        });
        Ok(())
    }

    /// Whether the pyro methods have been linked.
    pub fn linked(&self) -> bool {
        self.methods.is_some()
    }

    /// Access the pyro methods (panics if not linked).
    pub(crate) fn methods(&self) -> Option<&PyroMethods> {
        self.methods.as_ref()
    }

    /// Record an error that will be surfaced after the wasm call completes.
    pub fn set_error(&mut self, err: anyhow::Error) -> wasmtime::Error {
        self.methods
            .as_mut()
            .expect("PyroState not linked")
            .set_error(err)
    }

    /// Take the last recorded error, if any.
    pub fn take_error(&mut self) -> Option<anyhow::Error> {
        self.methods.as_mut().and_then(|m| m.take_error())
    }

    pub fn module_log(&self, log: &[u8]) {
        let log_msg = String::from_utf8_lossy(log).trim_end().to_string();
        self.module_log.lock().unwrap().push(log_msg);
    }

    pub fn log(&self) -> Vec<String> {
        let mut log = self.module_log.lock().unwrap();
        let mut new_log = Vec::with_capacity(log.capacity());
        std::mem::swap(&mut *log, &mut new_log);
        new_log
    }
}
