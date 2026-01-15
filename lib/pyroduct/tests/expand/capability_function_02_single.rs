use pyroduct::capability_function;

// Should use safe_call::i_call with the type directly, no __Input struct
#[capability_function]
fn single(x: u32) -> u32 {
    x
}