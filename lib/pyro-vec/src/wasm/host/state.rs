use std::ops::Deref;

use wasmtime::{ExternType, Instance, Module, Store, TypedFunc, ValType};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Wasm module is missing required export: {0}")]
    MissingExport(String),
    #[error("Wasm module export '{0}' has the wrong signature")]
    IncorrectSignature(String),
    #[error("Wasm export '{0}' signature mismatch")]
    SignatureMismatch(String),
}

// ---------------------------------------------------------------------------
// PyroModule — validated wrapper around wasmtime::Module
// ---------------------------------------------------------------------------

pub struct PyroModule {
    module: Module,
}

impl PyroModule {
    /// Validates and wraps a Wasmtime Module.
    ///
    /// Checks that the module exports `new_input`, `grow_input`, `free_output`,
    /// and `memory` with the correct signatures.
    pub fn new(module: Module) -> Result<Self, ModuleError> {
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
            return Err(ModuleError::MissingExport("memory".to_string()));
        }

        Ok(Self { module })
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
    ) -> Result<(), ModuleError> {
        let export = module
            .get_export(name)
            .ok_or_else(|| ModuleError::MissingExport(name.to_string()))?;

        match export {
            ExternType::Func(func_type) => {
                if 
                    func_type.params().zip(params).all(|(f,p)| !f.matches(p))
                    || func_type.params().len() != params.len()
                    || func_type.results().zip(results).any(|(f,p)| !f.matches(p))
                    || func_type.results().len() != results.len()
                {
                    return Err(ModuleError::SignatureMismatch(
                        name.to_string(),
                    ));
                }
            }
            _ => {
                return Err(ModuleError::SignatureMismatch(
                    name.to_string(),
                ));
            }
        }
        Ok(())
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

pub struct PyroState<T: 'static> {
    state: T,
    methods: Option<PyroMethods>,
}

impl<T> PyroState<T> {
    /// Create an un-linked state (methods not yet resolved).
    pub fn new(state: T) -> Self {
        Self {
            state,
            methods: None,
        }
    }

   pub fn link(
        store: &mut Store<Self>,
        instance: &Instance,
    ) -> Result<(), ModuleError> {
        let new_input = instance
            .get_typed_func::<i32, i32>(&mut *store, "new_input")
            .map_err(|_| ModuleError::IncorrectSignature("new_input".to_string()))?;

        let grow_input = instance
            .get_typed_func::<(i32, i32), i32>(&mut *store, "grow_input")
            .map_err(|_| ModuleError::IncorrectSignature("grow_input".to_string()))?;

        let free_output = instance
            .get_typed_func::<i32, ()>(&mut *store, "free_output")
            .map_err(|_| ModuleError::IncorrectSignature("free_output".to_string()))?;
        store.data_mut().methods =
        Some(PyroMethods {
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
}

impl<T> Deref for PyroState<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}