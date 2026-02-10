use pyro_vec::bridgeable;

#[bridgeable]
struct Inner {
    value: i64,
}

#[bridgeable]
struct Outer {
    inner: Inner,
    count: u32,
}

fn main() {}