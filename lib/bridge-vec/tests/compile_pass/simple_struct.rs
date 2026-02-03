use bridge_vec::{bridgeable, Bridgeable};

#[bridgeable]
struct SimpleStruct {
    id: u32,
    name: String,
}

fn main() {}