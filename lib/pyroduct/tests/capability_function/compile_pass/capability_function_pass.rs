use pyroduct::capability_function;

#[capability_function]
fn simple(x: u32) -> u32 {
    x
}

#[capability_function]
async fn simple_async(x: u32) -> u32 {
    x
}

#[capability_function]
fn multiple(x: u32, y: String) -> u32 {
    x
}

#[capability_function]
fn no_args() {}

fn main() {
    // Just verifying compilation
}
