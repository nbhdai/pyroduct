use pyroduct::format::Document;

#[derive(Document)]
struct Inner {
    value: i64,
}

#[derive(Document)]
struct Outer {
    inner: Inner,
    count: u32,
}

fn main() {}
