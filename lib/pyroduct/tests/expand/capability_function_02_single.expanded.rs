use pyroduct::capability_function;
fn single(x: u32) -> u32 {
    x
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_single(
    client_state_ptr: *const u8,
    client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    host_state_ptr: *mut std::ffi::c_void,
) -> ::pyroduct::capability_host::ffi::FfiResult {
    ::pyroduct::capability::safe_call::i_call::<
        u32,
        u32,
        _,
    >(
        client_state_ptr,
        client_state_len,
        input_ptr,
        input_len,
        host_state_ptr,
        |input| single(input),
    )
}
