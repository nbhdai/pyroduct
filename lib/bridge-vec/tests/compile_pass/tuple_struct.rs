use bridge_vec::{bridgeable, Bridgeable};

#[bridgeable]
struct TupleStruct(u32, String);

fn main() {
    let t = TupleStruct(10, "tuple".to_string());
    let _ = t;
}