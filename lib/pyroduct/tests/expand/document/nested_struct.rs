use pyroduct::Document;

#[derive(Document)]
/// Inner struct
struct Inner {
    value: i64,
}

#[derive(Document)]
/// Outer struct
struct Outer {
    inner: Inner,
    count: u32,
}

fn main() {}