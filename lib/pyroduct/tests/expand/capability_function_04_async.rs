use pyroduct::capability_function;

// Should use safe_async::async_i_call
#[capability_function]
async fn async_op(x: u32) -> u32 {
    x
}