use bridge_vec::bridgeable;

#[bridgeable]
struct SimpleStruct {
    id: u32,
    name: String,
}

fn main() {}