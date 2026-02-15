//! Generics are not currently supported

use pyroduct::{FromRow, DeepRef};

#[derive(FromRow, DeepRef)]
struct GenericStruct<T> {
    value: T,
}

fn main() {}