//! Generics are not currently supported

use pyroduct::format::{DeepRef, FromRow};

#[derive(FromRow, DeepRef)]
struct GenericStruct<T> {
    value: T,
}

fn main() {}
