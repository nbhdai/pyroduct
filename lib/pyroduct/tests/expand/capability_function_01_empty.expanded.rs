use pyroduct::capability_function;
fn empty() {}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_empty(
    client_state_ptr: *const u8,
    client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    host_state_ptr: *mut std::ffi::c_void,
) -> ::pyroduct::capability_host::ffi::FfiResult {
    ::pyroduct::capability::safe_call::empty_call::<
        (),
        _,
    >(
        client_state_ptr,
        client_state_len,
        input_ptr,
        input_len,
        host_state_ptr,
        || empty(),
    )
}
