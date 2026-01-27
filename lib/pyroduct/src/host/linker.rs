use crate::capability_host::ffi::{ClassDropFn, Function};
use crate::errors::PyroductError;
use crate::host::class::CapClass;
use crate::host::ffi_bridge::{AsyncExecFuture, ExecutionResultBridge};
use crate::host::function::CapFunction;
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr;
use tracing::{error, info};
use wasmtime::Caller;

// --- Type definitions ---
pub type WasmArgs = (i32, i32, i32, i32); // (wasm_state_ptr, wasm_state_len, ptr, len)



pub trait Capability: Send + Sync {
    fn functions(&self) -> Vec<CapFunction>;
    fn classes(&self) -> Vec<CapClass>;

    fn path(&self) -> Option<&Path>;
    fn name(&self) -> String;
}

/// Holds the state for a loaded capability, which may contain multiple classes.
pub struct CapabilityState {
    /// States for each class in the capability.
    /// Index corresponds to the index in `DynamicCapability.classes`.
    pub classes: Vec<ClassState>,
}

unsafe impl Send for CapabilityState {}

impl CapabilityState {
    pub fn get_class_ptr(&self, index: usize) -> *mut c_void {
        self.classes
            .get(index)
            .map(|s| s.ptr)
            .unwrap_or(ptr::null_mut())
    }
}

pub struct HarnessState {
    // Map capability index -> CapabilityState
    pub cap_states: Vec<CapabilityState>,
    /// Shared slot for an error that occurred during a host function call
    pub error_slot: Option<PyroductError>,
}

/// Describes a single function import from a capability
pub struct CapabilityImport {
    pub module: String,
    pub name: String,
    pub func: Function<'static>,
    /// If Some, this function belongs to the class at this index
    pub class_index: Option<usize>,
}

// Implement extension for all Capabilities
impl<T: Capability + ?Sized> CapabilityExt for T {}

