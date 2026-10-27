//! Test #[magma] with generic structs

use pyroformat::{Reader, Layout, Writer};

#[derive(Layout, Writer, Reader)]
struct Address {
    street: String,
    city: String,
    zip: u32,
}

#[derive(Layout, Writer, Reader)]
struct Person<T> {
    name: String,
    age: i32,
    address: T,
}

fn main() {}
