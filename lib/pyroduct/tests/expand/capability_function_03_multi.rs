use pyroduct::capability_function;

// Should generate __multi_Input struct and use it in safe_call::i_call
#[capability_function]
fn multi(x: u32, y: String) -> u32 {
    x
}