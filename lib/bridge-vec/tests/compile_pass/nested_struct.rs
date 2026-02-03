use bridge_vec::{bridgeable, Bridgeable};

#[bridgeable]
struct Inner {
    value: i64,
}

#[bridgeable]
struct Outer {
    inner: Inner,
    count: u32,
}

fn main() {
    let o = Outer {
        inner: Inner { value: 100 },
        count: 5,
    };
    let _ = o;
}