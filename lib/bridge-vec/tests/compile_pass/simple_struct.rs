use bridge_vec::{bridgeable, Bridgeable};

#[bridgeable]
struct SimpleStruct {
    id: u32,
    name: String,
}

fn main() {
    let s = SimpleStruct {
        id: 42,
        name: "test".to_string(),
    };
    let _ = s;
}