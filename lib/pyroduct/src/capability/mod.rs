// Capability functions
pub mod logger;
pub mod safe_async;
pub mod safe_call;
pub mod safe_lifecycle;
use std::ffi::c_void;

use bridge_vec::BridgeError;
pub use logger::init_logging;


#[tracing::instrument]
pub unsafe fn get_capability_state<'a, T>(
    host_state_ptr: *mut c_void,
) -> Result<&'a mut T, BridgeError> {
    if host_state_ptr.is_null() {
        tracing::error!("get_capability_state: host_state_ptr is null");
        return Err(BridgeError::NullPointer);
    }
    Ok(unsafe { &mut *(host_state_ptr as *mut T) })
}
