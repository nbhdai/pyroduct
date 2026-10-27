//! Test #[magma] with multiple generic parameters

use pyroformat::{Layout, Writer, Reader};

#[derive(Layout, Writer, Reader)]
struct Address {
    street: String,
    city: String,
    zip: u32,
}

#[derive(Layout, Writer, Reader)]
struct Person<S, T> {
    name: S,
    age: i32,
    address: T,
}

fn main() {}
