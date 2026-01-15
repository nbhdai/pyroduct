use pyroduct::capability_function;
async fn async_op(x: u32) -> u32 {
    x
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_async_op(
    client_state_ptr: *const u8,
    client_state_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    host_state_ptr: *mut std::ffi::c_void,
) -> ::pyroduct::capability_host::ffi::FfiBorrowedFutureResult<'static> {
    ::pyroduct::capability::safe_async::async_i_call::<
        u32,
        u32,
        _,
        _,
    >(
        client_state_ptr,
        client_state_len,
        input_ptr,
        input_len,
        host_state_ptr,
        |input| async move { async_op(input).await },
    )
}
