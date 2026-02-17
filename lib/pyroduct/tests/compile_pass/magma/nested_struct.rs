use pyroduct::magma;

#[magma]
struct Inner {
    value: i64,
}

#[magma]
struct Outer {
    inner: Vec<Inner>,
    count: u32,
}

fn main() {}